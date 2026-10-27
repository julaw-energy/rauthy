# Changelog of our fork

This file is for the changes of our fork (`julaw-energy/rauthy`, branch `custom`). Upstream's own history stays in
`CHANGELOG.md`, which we do not edit, so merging a new upstream release never conflicts here.

A release is named `<upstream version>-custom.<n>`, for example `0.37.2-custom.2` is upstream v0.37.2 plus our second
release. The counter starts again at 1 with every new upstream version. The sections are the same as upstream's:
**Security**, **Breaking**, **Changes**, **Bugfix**. A release note says what an operator has to do (config, migration,
rollout), not only what changed.

---

## v0.37.2-custom.4 (not released yet)

### Changes

- **Access rules per client.** A client can say who may log in by the user's values and attributes: `country is one of
  UG, KE, TZ`, `age is at least 18`, `email ends with @example.com`, `roles contains staff`, `attr:<name> has a value`.
  All rules of a client must hold; a missing value never meets a rule (except "is empty"). The conditions are `is`,
  `is not`, `>`, `>=`, `<`, `<=`, `is one of`, `is none of`, `contains`, `starts with`, `ends with`, `has a value`,
  `is empty`. Checked at every login, token refresh, password grant and forward auth request, never for the `rauthy` client.
  New section in the client editor, `GET` / `PUT /auth/v1/clients/{id}/access_rules`. A client without rules costs one cache
  lookup. See [Access Rules per Client](book/src/work/client_access_rules.md).
- **Default roles and groups for new users** (Config -> New users). Users who register themselves and users created by
  their first provider login get them. `rauthy_admin` and the delegated admin roles cannot be chosen. Users an admin
  creates are not changed. `GET` / `PUT /auth/v1/settings/new_users`.
- **Reminders for users who have not logged in** (Config -> New users). Off by default; 30 days and 3 reminders in a
  row by default, wording and numbers set by the admin, "Save and send a test to me". At most 100 users per hour, sent
  by the leader node, tracked per user so nobody gets the same reminder twice. `GET` / `PUT /auth/v1/settings/reminders`.
  See [New Users](book/src/work/new_users.md).
- Two new fork migrations: `10_access_rules_and_reminders` (two tables) and `11_reminder_indexes` (two indexes). Both tables
  are copied by the Hiqlite / Postgres migration tool.

### Operators

- Reminders do nothing until you switch them on. When you do, everybody who has been away longer than the time is due at
  once; the page tells you how many, and they are mailed over the next hours.
- The hourly reminder run is cheap at a large scale: on 1,000,000 users it takes about 75 ms when nobody is due (an earlier
  version of the query needed 2.1 s), thanks to an index on the last activity and one on the reminder state. Once a day,
  and after a start, it looks at every user (about 1.6 s at that size).

### Bugfix

- **The profile picture of a federated user (Google, Github, ...) was never shown.** The link was stored and sent to the page,
  but every page carried a Content-Security-Policy with `img-src 'self' data:`, so the browser refused to load an image from
  another host. The policy now also allows `https:` images (`frontend-next/svelte.config.js`). Plain `http` images stay blocked,
  scripts are still locked to hashes. The browser loads the image from the provider, Rauthy never fetches it.
  - The custom.2 / custom.3 notes said the picture was shown in the UI. It only reached the ID token and userinfo.
  - Trade-off: with `https:` an injected `<img>` could send data to any https host. Script injection is not made easier. If that is
    not acceptable, the alternative is a small avatar proxy in Rauthy (more code, Rauthy then fetches the provider's URL).
  - A user gets the picture at their next login through the provider.

---

## v0.37.2-custom.3

### Bugfix

- **Do not use `0.37.2-custom.2`.** Reading the events read marker (`GET /events/read`, and the event stream with
  `?read=true`, which the admin UI opens on every page) selected one column too few and panicked with `Cannot convert
  column index 'user_id'`. Because Rauthy aborts on a panic, the process went down as soon as an admin page was opened.
  It came from a refactor made after the feature had been tested. custom.3 fixes it and was checked against the real
  endpoints (marker read, marker moved, stream with the `read` event).

- **A failing SMTP server could flood the event log (upstream behaviour).** With an unreachable SMTP server and an events
  address set, a failed email raises a Critical `EmailSendError` event; that event is itself notified by email, which fails
  and raises the next event, without end (1558 events in 4 seconds were seen in a test). A failed *event notification*
  email no longer raises an event, so one failure gives two events and stops. It is logged instead.

No new migration. Moving from custom.1 or custom.2 needs nothing special (the data written by custom.2 is compatible).

---

## v0.37.2-custom.2

### Security

- The documentation book at `/auth/v1/book/` was readable by anyone. It now needs a logged-in session. Set
  `server.book_public = true` (`BOOK_PUBLIC`) to open it again.
- Webhook channels: a request can no longer be replayed unnoticed. With a signing secret set, it carries
  `X-Rauthy-Timestamp` and `X-Rauthy-Signature-Timestamped` (HMAC of `<timestamp>.<body>`), see
  [Message Channels](book/src/work/message_channels.md). `X-Rauthy-Signature` stays unchanged, so existing receivers
  keep working.
- The API reference needs a session unless `server.scalar_public` is set (see Breaking).

### Breaking

- **The Swagger UI has been removed**, together with its dependency (smaller binary, ~13 MB less memory when it was on).
  [Scalar](book/src/scalar.md) is the only API reference, at `/auth/v1/docs/`.
  - `server.swagger_ui_enable` / `SWAGGER_UI_ENABLE` is ignored (a warning is logged if it is `true`). Use
    `server.scalar_enable`.
  - `server.swagger_ui_public` / `SWAGGER_UI_PUBLIC` keeps working as an alias of the new `server.scalar_public`.
  - The separate page `/auth/v1/docs/scalar` is gone, `/auth/v1/docs/` is the one address.
  - An existing config file does not need to change. Nothing fails to start.

### Changes

- **Profile pictures of federated users.** The `picture` claim of the provider (`avatar_url` for Github) is stored when
  a user logs in through an auth provider and kept current on every login. It is shown in the account and admin UI, and
  it is the `picture` claim of the ID token and userinfo for client applications (scope `profile`). Only `https` links
  are taken, Rauthy never fetches them. A picture the user uploads replaces the link (Rauthy then serves its own URL).
  No migration: the link is kept in the existing `picture_id` column.
- **Static settings page** under Config: a read-only view of the values from the config file / environment that can
  only change with a restart (server, database, encryption key ids, WebAuthn, lifetimes, events, pictures, TLS,
  logging, ...). Secrets only show `set` / `not set`. `GET /auth/v1/settings/static`.
- **Webhook channels** send an `Idempotency-Key` header for every message (also in the body as `idempotency_key`); a
  retried system email carries the same key.
- **"Send a test to me"** shows what the endpoint answered (HTTP status and the start of the answer).
- **The events bell shows unread events, per admin, on every device.** The header bell has a badge with the number of
  events newer than where you last read, and the Events entry in the side navigation shows a dot. Opening the list marks
  everything as read. The marker is stored per admin in the database (new table `event_read_state`), so it is the same
  on every browser and device, and it survives a reload.
  - `GET /auth/v1/events/read` and `PUT /auth/v1/events/read` read and move the marker (it only moves forward).
  - The event stream (`GET /auth/v1/events/stream?read=true`, sessions only) sends the marker as the named SSE event
    `read`, once when it opens and again whenever another tab or device moves it. Clients that do not ask for it, and
    every existing API consumer of the stream, get exactly what they got before.
  - In a cluster, the live update only reaches the connections on the node that took the change (the other nodes catch up
    when the tab regains focus or reconnects).

### Bugfix

- **Moving from Hiqlite to Postgres (`MIGRATE_DB_FROM=sqlite:...`) failed at the very start** with `no such table: config`, in
  upstream's code: the version check read the target process instead of the source file. A second bug stopped it later,
  on the FIDO device table (`expected tuple struct KeyProtectionMask`). Both are fixed. It was rehearsed end to end on a
  copy of the dev data with Postgres 18: all 60 tables that exist on both sides, including every table and column this fork added, arrived
  with the same row counts, and users, clients, attributes and read markers have the same content. The tool copies
  `event_read_state` too. The fork's Postgres migrations (1-9) also ran for the first time on a real Postgres in this test.
- The workflow `.github/workflows/image.yml` was invalid, so GitHub reported "No jobs were run" on every push. It is a
  manual workflow again (Actions -> Build image -> Run workflow).
- The button to unlink a federated account in the account dashboard was drawn without border and hardly visible.
- The image had no default command. It now starts `serve` like upstream's image.

### Rollout

One new fork migration: `9_event_read_state` (a new table, nothing existing is changed). It applies on the first start. Image
and upgrade steps as for custom.1 (stop, back up the data volume, start the new image).

---

## v0.37.2-custom.1

The first release of the fork, based on upstream v0.37.2.

### Breaking

- **Only the new admin and account UI** (`frontend-next`, SvelteKit + shadcn) ships in the image. The old UI is not
  built in any more.
- `notifyd` is not a channel driver any more. A stored `notifyd` channel stays disabled until a new driver is chosen
  (a warning is logged); a config file or environment using `driver = "notifyd"` stops with a message at startup.
- `whatsapp` is not built in. Create it as a custom channel if you need it.
- Our database changes are **not** part of upstream's migration list. They live in `migrations_fork/` with their own
  table `_fork_migrations`, applied after upstream's. Instances that ran a pre-release with migrations 36-43 adopt them
  once. See [Upgrading Hiqlite](book/src/config/upgrading_hiqlite.md) for the in-place upgrade.

### Changes

- **Message channels.** `email` and `sms` always exist (recipient: the user's email address / phone number). Custom
  channels can be created, each with a driver, a recipient (`email`, `phone` or a custom attribute `attr:<name>`) and an
  optional expiry date; only expired channels can be deleted.
  - Drivers: `smtp` (a custom channel has its own server and credentials), `webhook` (Rauthy's signed JSON format) and
    `http` (any provider: method, URL, headers, JSON or form body, secret values, placeholders).
  - **System emails follow the `email` channel**: with `webhook` or `http`, password resets and codes go to a
    communication agent and Rauthy does not need the SMTP credentials.
  - Secrets are encrypted in the database and decrypted once when the settings load.
- **Verifiable attributes.** New attribute type `verified`; every verifiable attribute is linked to the channels that
  can verify it, and the user picks one. The verification code goes to the attribute's value.
- **Editable message templates** (Config -> Message templates): wording of the password and verification messages per
  language, validated before saving, with a test that sends to the admin's own address or phone. A save swaps the
  wording in memory, so sending stays as fast as before. Other nodes of a cluster pick it up within 15 seconds.
- **Settings in the database** (Config): SMTP, general settings, user fields, lifetimes and channels are edited in the
  admin UI. Values from the config file seed them once.
- **Required client data**: a client can require user values or attributes before a login completes.
- **User gender**, per-user client list and the SMS / phone verification flow.
- **Documentation book** is embedded in the binary at `/auth/v1/book/`.
- **Scalar API reference** instead of the Swagger UI (optional, `server.scalar_enable`).
- **Image and tooling**: `Dockerfile.fork`, `tools/image/build.sh` (local build and push to ghcr.io),
  `tools/upgrade/hiqlite_upgrade.py` (snapshot, inspect, rehearse an upgrade on a copy).

### Rollout

Snapshot the data, rehearse on a copy, stop the old container, start the new image on the same data volume with the
same config and secrets. The fork migrations apply on the first start (1-8 for custom.1, 9 is added by custom.2).
