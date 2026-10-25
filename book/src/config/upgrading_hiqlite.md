# Upgrading an Existing Hiqlite Instance to This Build

This build adds features (users' connected apps, typed attributes, channels, message templates, settings in the
database, required client data, ...). An existing Rauthy v0.37 database moves to it in place: the build carries 8 new
migrations of its own and applies them at the first start. They only **add** tables and columns, nothing is dropped
or rewritten, and your users, clients, passkeys, roles and groups are not touched.

Our migrations are kept **apart from upstream's**: they live in `migrations_fork/`, are numbered 1 to 8 and are tracked
in their own table `_fork_migrations`. Hiqlite refuses to start when its applied migrations are not exactly a prefix of
the built-in ones, so sharing upstream's numbering would make every migration that upstream adds collide with ours.
Upstream's table `_migrations` stays exactly as upstream has it, so upstream releases can be merged without touching
the database.

`tools/upgrade/hiqlite_upgrade.py` (Python 3, nothing to install) adds the safety around that: a consistent snapshot,
a check of what would happen, a rehearsal on a throw-away copy, and a before / after comparison of your data.

## 1. Snapshot the Live Database

Works while the instance runs, and gives one consistent file:

```bash
python3 tools/upgrade/hiqlite_upgrade.py snapshot /path/to/rauthy_data /safe/place/before-upgrade.sqlite
```

The data dir of the container belongs to the container user (`10001`), so your own user cannot read it. Run the script
as root (`sudo python3 ...`), or without installing anything, in a throw-away container that has Python:

```bash
docker run --rm -v /path/to/rauthy_data:/data:ro -v $PWD/tools/upgrade:/t:ro -v /safe/place:/out python:3-slim \
    python /t/hiqlite_upgrade.py snapshot /data /out/before-upgrade.sqlite
```

The file holds everything, password hashes and encrypted secrets included. Protect it like the database itself. It is
also the file you restore from if you ever want to go back (see below).

## 2. See What Would Happen

```bash
python3 tools/upgrade/hiqlite_upgrade.py inspect /safe/place/before-upgrade.sqlite
```

It checks that the file is intact, lists the upstream and the fork migrations that will be applied, and **blocks** if the
database was written by a newer build, or if an already applied migration differs from the one in this build (an applied
migration must never be edited).

## 3. Rehearse on a Copy

```bash
# the binary of the new image
docker create --name rauthy_new <your image> && docker cp rauthy_new:/app/rauthy ./rauthy_new && docker rm rauthy_new

python3 tools/upgrade/hiqlite_upgrade.py rehearse /safe/place/before-upgrade.sqlite \
    --bin ./rauthy_new --cwd /dir/with/your/config --env-file prod.env
```

The rehearsal restores the snapshot into a temporary data dir (the official `HQL_BACKUP_RESTORE` path), starts the new
build on free local ports, waits for the health check, stops it and compares. `PASS` means: it started, all migrations
are applied, the integrity check is fine and no row of any permanent table was lost. Your original is never touched.

On the copy it cancels open bulk message jobs, removes the SMTP and channel settings and the failed SCIM / back-channel
logout retries, and blanks `SMTP_URL`, so nothing is sent to real users. Event notifications (Slack, Matrix) that are set
in a **config file** cannot be switched off from outside: use a config without them for the rehearsal.

## 4. Check Your Config

- A channel that still uses the removed **`notifyd`** driver (`CHANNEL_SMS_DRIVER=notifyd` in the environment) stops
  Rauthy from starting. Use `webhook`. A `notifyd` channel stored in the database stays disabled until you choose a new
  driver.
- `[channels.whatsapp]` in the config file is ignored. Create the channel in the admin UI (Config → Channels).
- At the first start, SMTP, the channels, user fields, session / token lifetimes and the app name from your config / env
  are **copied once into the database** and are managed in the admin UI from then on. Keys you do not change can stay
  in the config.

## 5. Upgrade

The storage directory that the running container mounts (it holds the Hiqlite database, its raft logs / WAL and the
snapshots) is upgraded **in place**: the new image is started on the same mount. Hiqlite opens the existing logs and
state, applies the 8 missing migrations inside the database and carries on. Tested with the built image: a restore of a
v0.37.2 database applied migrations 36 to 43, and a second start on the same data dir found "nothing to migrate".

1. Stop the old container gracefully (`docker stop -t 30`), so Hiqlite flushes and closes its logs.
2. Copy the whole data dir (`cp -a rauthy_data rauthy_data.before`). This is your quickest way back.
3. Start the new image with the same mount, the same config and the same secrets and encryption keys. The migrations
   are applied at startup and logged (`Applying database migration 36 user_gender` ...).
4. Log in, open the admin UI, check **Config → Channels** and send a test message.

If you prefer to start from the snapshot in a fresh data dir, set `HQL_BACKUP_RESTORE=file:/app/data/restore.sqlite`
(with the snapshot copied there) for the first start, and **remove it again** afterwards, or the snapshot is restored
over and over.

## Going Back

Stop the new container, put `rauthy_data.before` (or the snapshot, via `HQL_BACKUP_RESTORE`) back and start the old
image. Anything written after the upgrade is lost with it. Do not start an upstream image on an upgraded database: this
is not tested.

## For Maintainers

- Our migrations go into `migrations_fork/hiqlite` and `migrations_fork/postgres` (same number, same name), numbered
  1, 2, 3 ... at the end. Never edit one after it was deployed: its hash is checked and Rauthy refuses to start.
- Never put our changes into upstream's `migrations/` folders. When you merge upstream, its migrations arrive there and
  nothing collides.
- A development database from the first version of this fork has our 8 migrations as 36-43 (Hiqlite) / V31-V38
  (Postgres) in upstream's table. At the first start they are adopted once: recorded in `_fork_migrations` and removed
  from upstream's table.
- Rehearsed with the script: (1) a database rebuilt to the state of upstream v0.37.2 (migration 35): the 8 fork
  migrations applied and all rows were kept; (2) a database with the early 36-43 numbering: adopted, then "nothing to
  migrate". The Postgres code path compiles and is not rehearsed: test it against a copy before using Postgres.
