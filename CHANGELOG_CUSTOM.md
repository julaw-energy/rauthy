# Changelog of our fork

This file is for the changes of our fork (`julaw-energy/rauthy`, branch `custom`). Upstream's own history stays in
`CHANGELOG.md`, which we do not edit, so merging a new upstream release never conflicts here.

A release is named `<upstream version>-custom.<n>`, for example `0.37.2-custom.2` is upstream v0.37.2 plus our second
release. The counter starts again at 1 with every new upstream version. The sections are the same as upstream's:
**Security**, **Breaking**, **Changes**, **Bugfix**. A release note says what an operator has to do (config, migration,
rollout), not only what changed.

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
- The events bell shows a badge with the number of new events (notice and above) that arrived while the list is not on
  screen. The Events entry in the side navigation shows a dot.

### Bugfix

- The workflow `.github/workflows/image.yml` was invalid, so GitHub reported "No jobs were run" on every push. It is a
  manual workflow again (Actions -> Build image -> Run workflow).
- The button to unlink a federated account in the account dashboard was drawn without border and hardly visible.
- The image had no default command. It now starts `serve` like upstream's image.

### Rollout

No new database migration. Image and upgrade steps as for custom.1 (stop, back up the data volume, start the new image).

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
same config and secrets. The fork migrations 1-8 apply on the first start.
