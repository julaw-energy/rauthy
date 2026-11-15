#!/usr/bin/env python3
"""
Walks ONE database through a chain of Rauthy images, oldest first, and checks after every hop that
the next image starts on the data of the previous one and that nothing seeded earlier is lost.

  chain_test.py                         default chain, Hiqlite, candidate = rauthy-custom:0.37.2-custom.7
  chain_test.py --candidate IMG         test the upgrade into IMG at the end of the chain
  chain_test.py --db postgres           same on a throw-away Postgres container
  chain_test.py --images A B C          own chain (the candidate is appended)

What happens:
  hop 0   first image, fresh database, bootstrap admin + a bootstrap API key, seed data:
          a group, a role, a client, a user, an auth provider
  hop n   stop the container, start the next image on the SAME data, wait for health,
          scan the log for panics / migration errors, check the seeded data is all still there,
          write something new (so a half-migrated schema fails on an INSERT, not only on a SELECT)

It needs docker, python 3.8+ and nothing else. The data lives in a fresh directory each run
(the containers run as your own uid, so it can be deleted afterwards). Port 18081 is used.
Exit code 1 when any hop fails.
"""
import argparse
import base64
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
PORT = 18081
BASE = f"http://localhost:{PORT}/auth/v1"
SECRET = "chaintestchaintestchaintestchaintestchaintestchaintestchaintest00"
DEFAULT_CHAIN = [
    "ghcr.io/sebadob/rauthy:0.37.2",
    "ghcr.io/julaw-energy/rauthy:0.37.2-custom.4",
    "ghcr.io/julaw-energy/rauthy:0.37.2-custom.6",
    "ghcr.io/julaw-energy/rauthy:0.37.2-custom.7",
]
DEFAULT_CANDIDATE = "rauthy-custom:0.37.2-custom.7"
NAME = "rauthy-chain"
PG_NAME = "rauthy-chain-pg"
PG_PORT = 15432
ACCESS = [
    {"group": g, "access_rights": ["read", "create", "update", "delete"]}
    for g in ("Clients", "Roles", "Groups", "Users", "AuthProviders")
]


def sh(*args, check=True, quiet=True):
    res = subprocess.run(args, capture_output=True, text=True)
    if check and res.returncode != 0:
        raise RuntimeError(f"{' '.join(args)}\n{res.stdout}{res.stderr}")
    return res


def api(method, path, body=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(f"{BASE}{path}", data=data, method=method)
    req.add_header("Authorization", f"API-Key chain${SECRET}")
    req.add_header("User-Agent", "chain-test")
    if data is not None:
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            raw = r.read()
            return r.status, (json.loads(raw) if raw else None)
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode(errors="replace")[:300]


def config_for(tmp: Path) -> Path:
    """The test config without keys an older build does not know (it refuses unknown keys)."""
    src = (REPO / "config-test.toml").read_text().splitlines()
    out = [ln for ln in src if not ln.strip().startswith("internal_hosts")]
    p = tmp / "config.toml"
    p.write_text("\n".join(out) + "\n")
    return p


def env_args(db: str):
    key = [{"name": "chain", "exp": 4102444800, "secret": {"Plain": SECRET}, "access": ACCESS}]
    env = {
        "HQL_DATA_DIR": "/app/data",
        "HQL_NODES": "1 127.0.0.1:18110 127.0.0.1:18210",
        "METRICS_PORT": "19291",
        "LISTEN_PORT_HTTP": str(PORT),
        "PUB_URL": f"localhost:{PORT}",
        "RP_ORIGIN": f"http://localhost:{PORT}",
        "BOOTSTRAP_ADMIN_EMAIL": "admin@localhost",
        "BOOTSTRAP_ADMIN_PASSWORD_PLAIN": "123SuperSafe",
        "BOOTSTRAP_API_KEY": base64.b64encode(json.dumps(key[0]).encode()).decode(),
        "BOOTSTRAP_API_KEY_SECRET": SECRET,
    }
    if db == "postgres":
        env.update({
            "HIQLITE": "false",
            "PG_HOST": "127.0.0.1",
            "PG_PORT": str(PG_PORT),
            "PG_USER": "rauthy",
            "PG_PASSWORD": "chainpw",
            "PG_DB_NAME": "rauthy",
        })
    args = []
    for k, v in env.items():
        args += ["-e", f"{k}={v}"]
    return args


def start(image, data: Path, conf: Path, db: str):
    sh("docker", "rm", "-f", NAME, check=False)
    uid, gid = os.getuid(), os.getgid()
    sh(
        "docker", "run", "-d", "--name", NAME, "--network", "host", "--user", f"{uid}:{gid}",
        "-v", f"{data}:/app/data", "-v", f"{conf}:/app/config.toml:ro",
        *env_args(db), image, "serve", "-c", "/app/config.toml",
    )


def wait_healthy(secs=90):
    end = time.time() + secs
    while time.time() < end:
        st = sh("docker", "inspect", "-f", "{{.State.Running}}", NAME, check=False).stdout.strip()
        if st != "true":
            return False, "container exited"
        try:
            with urllib.request.urlopen(f"{BASE}/health", timeout=3) as r:
                if r.status == 200 and b'"db_healthy":true' in r.read():
                    return True, ""
        except Exception:
            pass
        time.sleep(1.5)
    return False, "not healthy in time"


def log_problems():
    import re
    logs = sh("docker", "logs", NAME, check=False)
    text = re.sub(r"\x1b\[[0-9;]*m", "", logs.stdout + logs.stderr)
    bad = []
    for ln in text.splitlines():
        low = ln.lower()
        if "panicked" in low:
            bad.append(ln.strip()[:200])
        elif (" ERROR " in ln or " WARN " in ln) and ("migrat" in low or "schema" in low) \
                and "dev data" not in low:
            bad.append(ln.strip()[:200])
    return bad[:5]


def seed():
    steps = [
        ("group", "POST", "/groups", {"group": "chain_group"}),
        ("role", "POST", "/roles", {"role": "chain_role"}),
        ("client", "POST", "/clients", {
            "id": "chain_client", "name": "Chain Client", "confidential": False,
            "redirect_uris": ["http://localhost:9/cb"],
        }),
        ("user", "POST", "/users", {
            "email": "chain@localhost", "given_name": "Chain", "family_name": "User",
            "language": "en", "roles": ["chain_role"], "groups": ["chain_group"],
        }),
        ("provider", "POST", "/providers/create", {
            "name": "Chain Provider", "typ": "custom", "enabled": False,
            "issuer": "http://localhost:1/chain",
            "authorization_endpoint": "http://localhost:1/authorize",
            "token_endpoint": "http://localhost:1/token",
            "userinfo_endpoint": "http://localhost:1/userinfo",
            "use_pkce": True, "client_secret_basic": False, "client_secret_post": False,
            "auto_onboarding": False, "auto_link": False, "client_id": "chain",
            "scope": "openid",
        }),
    ]
    for what, m, p, b in steps:
        code, body = api(m, p, b)
        if code not in (200, 201):
            raise RuntimeError(f"seed {what}: {code} {body}")


def verify(hop):
    """Everything seeded must still be there; then write something new."""
    problems = []
    code, groups = api("GET", "/groups")
    if code != 200 or not any(g.get("name") == "chain_group" for g in groups):
        problems.append(f"group chain_group missing ({code})")
    code, roles = api("GET", "/roles")
    if code != 200 or not any(r.get("name") == "chain_role" for r in roles):
        problems.append(f"role chain_role missing ({code})")
    code, body = api("GET", "/clients")
    if code != 200 or not any(c.get("id") == "chain_client" for c in body):
        problems.append(f"client chain_client missing ({code})")
    code, body = api("POST", "/providers")
    if code != 200 or not any(p.get("name") == "Chain Provider" for p in body):
        problems.append(f"provider missing ({code})")
    code, body = api("GET", "/users")
    if code != 200 or not any(u.get("email") == "chain@localhost" for u in body):
        problems.append(f"user chain@localhost missing ({code})")
    else:
        # the list only has the short form, roles and groups are on the single user
        uid = next(u for u in body if u.get("email") == "chain@localhost")["id"]
        code, u = api("GET", f"/users/{uid}")
        if code != 200 or "chain_role" not in (u.get("roles") or []) \
                or "chain_group" not in (u.get("groups") or []):
            problems.append(f"user lost its role or group ({code})")
    code, body = api("POST", "/groups", {"group": f"chain_hop_{hop}"})
    if code not in (200, 201):
        problems.append(f"cannot write after the upgrade: {code} {body}")
    return problems


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--db", choices=["hiqlite", "postgres"], default="hiqlite")
    ap.add_argument("--images", nargs="*", default=DEFAULT_CHAIN)
    ap.add_argument("--candidate", default=DEFAULT_CANDIDATE)
    ap.add_argument("--keep", action="store_true", help="keep the data directory")
    args = ap.parse_args()
    chain = list(args.images) + [args.candidate]

    tmp = Path(tempfile.mkdtemp(prefix="rauthy-chain-"))
    data = tmp / "data"
    data.mkdir()
    conf = config_for(tmp)
    results = []
    try:
        if args.db == "postgres":
            sh("docker", "rm", "-f", PG_NAME, check=False)
            sh(
                "docker", "run", "-d", "--name", PG_NAME, "--network", "host",
                "-e", "POSTGRES_USER=rauthy", "-e", "POSTGRES_PASSWORD=chainpw",
                "-e", "POSTGRES_DB=rauthy", "-e", f"PGPORT={PG_PORT}", "postgres:18",
            )
            time.sleep(8)
        for hop, image in enumerate(chain):
            print(f"[{hop}] {image}", flush=True)
            start(image, data, conf, args.db)
            ok, why = wait_healthy()
            problems = [] if ok else [why]
            if ok and hop == 0:
                try:
                    seed()
                except RuntimeError as e:
                    problems.append(str(e))
            if ok and hop > 0:
                problems += verify(hop)
            problems += log_problems()
            results.append((image, problems))
            print("    " + ("ok" if not problems else "FAILED: " + "; ".join(problems)), flush=True)
            sh("docker", "stop", "-t", "30", NAME, check=False)
            if problems:
                logs = sh("docker", "logs", "--tail", "25", NAME, check=False)
                print((logs.stdout + logs.stderr)[-1800:])
                break
    finally:
        sh("docker", "rm", "-f", NAME, check=False)
        if args.db == "postgres":
            sh("docker", "rm", "-f", PG_NAME, check=False)
        if not args.keep:
            shutil.rmtree(tmp, ignore_errors=True)

    print()
    for image, problems in results:
        print(f"{'ok    ' if not problems else 'FAILED'}  {image}")
    failed = any(p for _, p in results) or len(results) != len(chain)
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
