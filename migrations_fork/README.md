# Fork migrations

Our own database changes. They are NOT in `migrations/hiqlite` / `migrations/postgres` on purpose:
those are upstream's, numbered by upstream, and Hiqlite (and refinery) refuse to start when the
applied list is not exactly a prefix of the built-in one. Sharing the numbering would make every
upstream migration collide with ours.

Ours run after upstream's, from `src/data/src/migration/fork_migrations.rs`, tracked in their own
table `_fork_migrations` (id, name, hash).

- Numbered 1, 2, 3, ... independently of upstream. Add new ones at the end only.
- NEVER edit a migration that has been deployed: its hash is checked and Rauthy refuses to start.
- Every statement must be plain DDL / DML without `;` inside of strings (the file is split on `;`).
- Each migration runs in one transaction.
