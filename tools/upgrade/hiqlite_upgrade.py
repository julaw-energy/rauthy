#!/usr/bin/env python3
"""
Safe upgrade of a Hiqlite based Rauthy database to this build.

The schema migration itself is automatic: this build carries every new migration (hiqlite 36-43)
and applies the missing ones at startup, in order, inside the database. Nothing is dropped or
rewritten, the migrations only add tables and columns. What this script adds is the safety
around it: a consistent snapshot, a check of what would happen, a rehearsal on a throw-away copy,
and a comparison of the data before and after.

  snapshot  DATA_DIR OUT.sqlite       consistent copy of the live database (safe while it runs)
  inspect   BACKUP.sqlite             what is applied, what is pending, anything that blocks
  rehearse  BACKUP.sqlite --bin PATH  start this build on a copy, migrate, compare, throw away

Python 3.8+, standard library only. The rehearsal never touches the original database.
"""
import argparse
import hashlib
import os
import shutil
import signal
import socket
import sqlite3
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
MIGRATIONS = REPO / "migrations" / "hiqlite"

# Short-lived data: it changes by itself while the instance runs, so a different row count after
# a start is no sign of loss.
EPHEMERAL = {
    "sessions", "refresh_tokens", "refresh_tokens_devices", "issued_tokens", "magic_links",
    "events", "login_locations", "user_login_states", "one_time_password", "devices",
    "recent_passwords", "pwd_exp_mails", "contact_verifications", "user_revoke", "_metadata",
    "sqlite_sequence", "sqlite_stat1", "sqlite_stat4",
}
# What the rehearsal switches off on its copy, so nothing leaves the machine
NEUTRALIZED = {"email_jobs", "settings", "failed_backchannel_logouts", "failed_scim_tasks"}


def ok(msg):
    print(f"  ok    {msg}")


def warn(msg):
    print(f"  WARN  {msg}")


def fail(msg):
    print(f"  FAIL  {msg}")


def connect_ro(path):
    return sqlite3.connect(f"file:{path}?mode=ro", uri=True)


def table_counts(db):
    out = {}
    for (name,) in db.execute("select name from sqlite_master where type='table' order by 1"):
        out[name] = db.execute(f'select count(*) from "{name}"').fetchone()[0]
    return out


def migration_files(directory):
    files = {}
    for f in directory.glob("*.sql"):
        num, _, name = f.stem.partition("_")
        if num.isdigit():
            files[int(num)] = (name, hashlib.sha256(f.read_bytes()).hexdigest(), f)
    return files


def applied(db):
    try:
        return {r[0]: (r[1], r[2]) for r in db.execute("select id, name, hash from _migrations")}
    except sqlite3.Error:
        return None


# ---------------------------------------------------------------------------------------------
def cmd_snapshot(args):
    src = Path(args.data_dir) / "state_machine" / "db" / args.db_name
    if not src.is_file():
        sys.exit(f"No database at {src}")
    out = Path(args.out)
    if out.exists():
        sys.exit(f"{out} exists already, refusing to overwrite it")
    out.parent.mkdir(parents=True, exist_ok=True)
    # sqlite's backup API reads a consistent state, including the WAL
    with connect_ro(src) as s, sqlite3.connect(out) as d:
        s.backup(d)
    os.chmod(out, 0o600)
    with sqlite3.connect(out) as d:
        res = d.execute("pragma integrity_check").fetchone()[0]
    print(f"Snapshot written to {out} ({out.stat().st_size} bytes), integrity: {res}")
    print("It holds all your data, including password hashes and encrypted secrets. Protect it.")
    return 0 if res == "ok" else 1


def inspect(backup, mig_dir):
    """Prints the findings, returns True if the upgrade can go ahead"""
    print(f"Inspecting {backup}")
    good = True
    db = connect_ro(backup)

    res = db.execute("pragma integrity_check").fetchone()[0]
    if res == "ok":
        ok("the database file is intact")
    else:
        fail(f"integrity check: {res}")
        return False

    have = applied(db)
    if have is None:
        fail("no _migrations table: this is not a Rauthy / Hiqlite database")
        return False
    files = migration_files(mig_dir)

    unknown = sorted(set(have) - set(files))
    if unknown:
        fail(
            f"the database has migrations this build does not know: {unknown}. It was written by a "
            "NEWER build. Do not start this build on it."
        )
        good = False
    changed = [i for i, (n, h) in have.items() if i in files and files[i][1] != h]
    if changed:
        fail(
            f"the applied migrations {sorted(changed)} differ from the ones in this build. A "
            "migration that was applied must never be edited."
        )
        good = False
    if not unknown and not changed:
        ok(f"all {len(have)} applied migrations match this build")

    pending = sorted(set(files) - set(have))
    if pending:
        print(f"  The next start will apply {len(pending)} migration(s):")
        for i in pending:
            print(f"        {i:>3}  {files[i][0]}")
    else:
        ok("nothing to migrate, the database is up to date")

    # a gap would mean an applied migration is missing in the middle
    top = max(have) if have else 0
    gaps = [i for i in range(1, top + 1) if i not in have]
    if gaps:
        fail(f"migrations are missing in the middle of the history: {gaps}")
        good = False

    counts = table_counts(db)
    print("  Data in the database:")
    for t in ("users", "clients", "roles", "groups", "user_attr_config", "user_attr_values", "passkeys"):
        if t in counts:
            print(f"        {t:<18}{counts[t]}")
    return good


def cmd_inspect(args):
    return 0 if inspect(Path(args.backup), Path(args.migrations)) else 1


# ---------------------------------------------------------------------------------------------
def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def neutralize(path):
    """Switch off everything on the copy that could reach the outside world"""
    db = sqlite3.connect(path)
    notes = []
    names = {r[0] for r in db.execute("select name from sqlite_master where type='table'")}
    if "email_jobs" in names:
        n = db.execute("update email_jobs set status = 2 where status = 0").rowcount
        notes.append(f"{n} open bulk message job(s) cancelled")
    if "settings" in names:
        n = db.execute("delete from settings where key = 'smtp' or key like 'channel.%'").rowcount
        notes.append(f"{n} SMTP / channel setting(s) removed")
    for t in ("failed_backchannel_logouts", "failed_scim_tasks"):
        if t in names:
            db.execute(f'delete from "{t}"')
    db.commit()
    db.close()
    return notes


def wait_healthy(port, proc, log, timeout):
    url = f"http://127.0.0.1:{port}/auth/v1/health"
    end = time.time() + timeout
    while time.time() < end:
        if proc.poll() is not None:
            return False, f"the process exited with {proc.returncode}"
        try:
            with urllib.request.urlopen(url, timeout=2) as r:
                if r.status == 200:
                    return True, "healthy"
        except Exception:
            pass
        time.sleep(1)
    return False, "no healthy answer in time"


def cmd_rehearse(args):
    backup = Path(args.backup).resolve()
    mig_dir = Path(args.migrations)
    if not inspect(backup, mig_dir):
        print("\nNot starting the rehearsal: fix the findings above first.")
        return 1

    work = Path(args.workdir) if args.workdir else Path(tempfile.mkdtemp(prefix="rauthy-rehearsal-"))
    work.mkdir(parents=True, exist_ok=True)
    data = work / "data"
    data.mkdir(exist_ok=True)
    restore = work / "restore.sqlite"
    shutil.copyfile(backup, restore)
    print(f"\nRehearsal in {work} (a copy, the original is not touched)")
    for n in neutralize(restore):
        ok(n)

    http, metrics, raft, api = (free_port() for _ in range(4))
    env = dict(os.environ)
    if args.env_file:
        for line in Path(args.env_file).read_text().splitlines():
            line = line.strip()
            if line and not line.startswith("#") and "=" in line:
                k, _, v = line.partition("=")
                env[k.strip()] = v.strip().strip("'\"")
    env.update(
        {
            "HQL_DATA_DIR": str(data),
            "HQL_NODES": f"1 127.0.0.1:{raft} 127.0.0.1:{api}",
            "HQL_BACKUP_RESTORE": f"file:{restore}",
            "LISTEN_PORT_HTTP": str(http),
            "LISTEN_SCHEME": "http",
            "METRICS_PORT": str(metrics),
            # no mail to the outside
            "SMTP_URL": "",
        }
    )
    # an empty value counts as "set", so these are removed, not blanked. Values from a config
    # file cannot be removed from here: use a scrubbed config for the rehearsal.
    for k in [k for k in env if k.startswith("EVENT_") and k != "EVENT_EMAIL"]:
        env.pop(k)
    logfile = work / "rauthy.log"
    print(f"Starting {args.bin} (log: {logfile})")
    with open(logfile, "wb") as log:
        proc = subprocess.Popen(
            [args.bin, "serve"],
            cwd=args.cwd or REPO,
            env=env,
            stdout=log,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )
        healthy, why = wait_healthy(http, proc, logfile, args.timeout)
        # let it finish its start-up work before it is stopped
        if healthy:
            time.sleep(3)
        if proc.poll() is None:
            os.killpg(proc.pid, signal.SIGTERM)
            try:
                proc.wait(20)
            except subprocess.TimeoutExpired:
                os.killpg(proc.pid, signal.SIGKILL)

    text = logfile.read_text(errors="replace")
    if not healthy:
        fail(f"the new build did not come up: {why}")
        print("  last log lines:")
        for line in text.splitlines()[-15:]:
            print("    " + line[:200])
        return 1
    ok("the new build started and answered the health check")

    after = work / "data" / "state_machine" / "db" / args.db_name
    if not after.is_file():
        fail(f"no database at {after} after the start")
        return 1

    good = True
    db_after = connect_ro(after)
    have = applied(db_after) or {}
    files = migration_files(mig_dir)
    if set(have) == set(files):
        ok(f"migrations are at {max(have)}, same as this build")
    else:
        fail(f"migrations applied: {sorted(set(files) - set(have))} are still missing")
        good = False
    res = db_after.execute("pragma integrity_check").fetchone()[0]
    (ok if res == "ok" else fail)(f"integrity check after the migration: {res}")
    good &= res == "ok"

    before = table_counts(connect_ro(backup))
    now = table_counts(db_after)
    print("  Data check (rows before -> after):")
    lost = False
    for t, n in sorted(before.items()):
        if t in NEUTRALIZED or t in EPHEMERAL:
            continue
        m = now.get(t)
        if m is None:
            fail(f"table {t} is gone")
            lost = True
        elif m < n:
            fail(f"{t}: {n} -> {m}")
            lost = True
        elif m > n:
            print(f"        {t:<26}{n} -> {m}")
    if not lost:
        ok("no row of any permanent table was lost")
    good &= not lost

    new_tables = sorted(set(now) - set(before))
    if new_tables:
        print(f"  New tables: {', '.join(new_tables)}")

    if not args.keep:
        shutil.rmtree(work, ignore_errors=True)
    else:
        print(f"  Kept {work}")
    print("\nRESULT:", "PASS, the upgrade can be done" if good else "FAIL, do not upgrade yet")
    return 0 if good else 1


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)

    s = sub.add_parser("snapshot", help="consistent copy of the live database")
    s.add_argument("data_dir")
    s.add_argument("out")
    s.add_argument("--db-name", default="hiqlite.db")
    s.set_defaults(fn=cmd_snapshot)

    s = sub.add_parser("inspect", help="what would be applied, anything blocking")
    s.add_argument("backup")
    s.add_argument("--migrations", default=str(MIGRATIONS))
    s.set_defaults(fn=cmd_inspect)

    s = sub.add_parser("rehearse", help="migrate a copy with this build and compare")
    s.add_argument("backup")
    s.add_argument("--bin", required=True, help="the rauthy binary of the new build")
    s.add_argument("--cwd", help="where the binary finds its config.toml (default: the repository)")
    s.add_argument("--env-file", help="KEY=VALUE lines, like the environment of the real instance")
    s.add_argument("--migrations", default=str(MIGRATIONS))
    s.add_argument("--workdir")
    s.add_argument("--db-name", default="hiqlite.db")
    s.add_argument("--timeout", type=int, default=240)
    s.add_argument("--keep", action="store_true", help="keep the working directory")
    s.set_defaults(fn=cmd_rehearse)

    args = p.parse_args()
    sys.exit(args.fn(args))


if __name__ == "__main__":
    main()
