# Upgrading an Existing Hiqlite Instance to This Build

This build adds features (users' connected apps, typed attributes, channels, message templates, settings in the
database, required client data, ...). An existing Rauthy v0.37 database moves to it in place: the build carries 8 new
migrations (Hiqlite `36` to `43`, Postgres `V31` to `V38`) and applies the missing ones at the first start. They only
**add** tables and columns, nothing is dropped or rewritten, and your users, clients, passkeys, roles and groups are
not touched.

`tools/upgrade/hiqlite_upgrade.py` (Python 3, nothing to install) adds the safety around that: a consistent snapshot,
a check of what would happen, a rehearsal on a throw-away copy, and a before / after comparison of your data.

## 1. Snapshot the Live Database

Works while the instance runs, and gives one consistent file:

```bash
python3 tools/upgrade/hiqlite_upgrade.py snapshot /path/to/rauthy_data /safe/place/before-upgrade.sqlite
```

The file holds everything, password hashes and encrypted secrets included. Protect it like the database itself. It is
also the file you restore from if you ever want to go back (see below).

## 2. See What Would Happen

```bash
python3 tools/upgrade/hiqlite_upgrade.py inspect /safe/place/before-upgrade.sqlite
```

It checks that the file is intact, lists the migrations that will be applied, and **blocks** if the database was written
by a newer build, or if an already applied migration differs from the one in this build (an applied migration must never
be edited).

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

1. Stop the old container.
2. Copy the data dir (`cp -a rauthy_data rauthy_data.before`). This is your quickest way back.
3. Start the new image on the same data dir. The migrations are applied at startup and logged.
4. Log in, open the admin UI, check **Config → Channels** and send a test message.

If you prefer to start from the snapshot in a fresh data dir, set `HQL_BACKUP_RESTORE=file:/app/data/restore.sqlite`
(with the snapshot copied there) for the first start, and **remove it again** afterwards, or the snapshot is restored
over and over.

## Going Back

Stop the new container, put `rauthy_data.before` (or the snapshot, via `HQL_BACKUP_RESTORE`) back and start the old
image. Anything written after the upgrade is lost with it. Do not start an upstream image on an upgraded database: this
is not tested.

## For Maintainers

- The migrations `36` to `43` (Hiqlite) and `V31` to `V38` (Postgres) belong to this fork. Never edit one after it was
  deployed, always add a new number.
- Upstream numbers its migrations too. Before merging upstream, compare `migrations/` of both sides: a migration with the
  same number on both sides has to be renumbered here, before anything is deployed.
- This was rehearsed on a database rebuilt to the state of upstream v0.37.2 (migration 35) with real data: 8 migrations
  applied, all rows kept. Postgres is not covered by the script, use a `pg_dump` and the same rehearsal idea there.
