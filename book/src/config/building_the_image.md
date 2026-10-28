# Building and Releasing This Fork

This build is the upstream release plus a set of extra features. Its version is `<upstream version>-custom.<n>`, for
example `0.37.2-custom.4`, and the git tag is `v0.37.2-custom.4`. A tag that was pushed is never moved: a fix is a new
number. The release notes are in `CHANGELOG_CUSTOM.md`, the upstream `CHANGELOG.md` is left alone.

## Build

```bash
# local image, loaded into the local docker
tools/image/build.sh --tag rauthy-custom:0.37.2-custom.4

# a quicker test image (thin LTO), never for a rollout
tools/image/build.sh --fast

# build and push (needs `docker login ghcr.io` first)
tools/image/build.sh --push --tag ghcr.io/<owner>/rauthy:0.37.2-custom.4
```

The build uses `Dockerfile.fork`: the UI, this documentation and a Rust release build, in one image. The first build
takes about 17 minutes, later ones reuse the layers. The version is taken from the newest `-custom.<n>` tag on the
commit, or from `--version`.

The GitHub workflow `.github/workflows/image.yml` does the same and only runs when started by hand, so a push never
overwrites a published image.

## Release Checklist

1. Move the entries of `CHANGELOG_CUSTOM.md` under the new version and date.
2. Commit, then tag: `git tag -a v0.37.2-custom.N`.
3. Build the image. The UI build checks its own output (`frontend-next/tools/check-build.mjs`: CSP hashes, no `data:`
   scripts, every script file exists) and stops the build on a problem. Then run the test suite against it, not only against a debug build. Release builds differ: no `--test`
   flag, no `/auth/v1/dev/*` routes (the back-channel logout test cannot pass there), and requests without a
   `User-Agent` are refused.
4. Open the built UI in a browser once and look at the console: a CSP error there is a bug, the three requests that
   answer 405 / 406 / 404 for WebID, self-delete and PAM are not (they are features that are switched off).
5. Push the image, the branch and the tag, then create the GitHub release.
6. Roll out by **digest**, not by tag, after a snapshot and a rehearsal on a copy of the real data
   ([Upgrading a Hiqlite Instance](upgrading_hiqlite.md)).

## Fork Migrations

Our schema changes are not part of the upstream migrations. They live in `migrations_fork/{hiqlite,postgres}` and run at
start, tracked in the table `_fork_migrations`. A migration that was deployed is never edited.

| # | What it does |
|---|---|
| 1 | `gender` on `users_values` |
| 2 | `user_clients`, the apps a user has used |
| 3 | more types for user attributes |
| 4 | a channel on `email_jobs` |
| 5 | `contact_verifications`, codes for email and phone |
| 6 | `settings`, settings kept in the database |
| 7 | `client_required_fields`, required data per client |
| 8 | `attr_verify_channels`, which channel verifies an attribute |
| 9 | `event_read_state`, the unread marker of the events bell |
| 10 | `client_access_rules` and `inactivity_reminders` |
| 11 | two indexes for the reminders (`users_last_activity_idx`, `inactivity_reminders_due_idx`) |
| 12 | the system user and `audit_log` |
| 13 | `audit_users`, the small copy of deleted users |
| 14 | the system user gets the id `101`, an index on `audit_users` |

Moving from `custom.1` to `custom.4` applies migrations 9 to 14 on the first start.
