# System Users and the Audit Log

## System users

System users own what the server does on its own, without an admin clicking something. They have a **number as id and
name** (101, 102, ...) and the email `<number>@system.invalid`. The first one, `101`, is "System Background tasks": the
reminders and the user expiry scheduler act as it.

They cannot log in (disabled, no password, no passkey), and they are read only: the server refuses every change to them
and every deletion, and the Admin UI shows them without forms, delete button or selection.

To keep the numbers for them, **no other user can have**

- a username that starts with a number. Usernames also need at least 4 characters. (This is checked in addition to the
  pattern you configure under `user_values.preferred_username`; a name that was accepted before can be saved again.)
- an email in the domain `system.invalid` or below it. `.invalid` can never be registered, so there is no mailbox.

## The audit log

Every successful change made through the API is recorded: who did it (the id of the user, `api_key:<name>` for an API
key, or `system`), when (Unix milliseconds), what happened, and to what. Entries are only added, never changed, and are
not cleaned up automatically. Nothing in the log refers to a user by a foreign key, so it stays complete when users are
deleted.

| Field | Meaning |
|---|---|
| `actor` | The id of the user who did it, `api_key:<name>`, or the id of a system user. `actor_label` is their email, if they are known. |
| `action` | `create`, `update`, `delete`, `action` (a change that is none of these, like sending a password reset), or a name for what the background tasks do: `inactivity_reminder_sent`, `user_expired_disabled`. |
| `target_type`, `target_id` | What it was about: `users`, `clients`, `roles`, `settings`, ... and its id or name. |
| `details` | For API changes the method and route, like `PUT /clients/{id}`. What was sent is never recorded, only the route. |

What is **not** recorded: reads, logins and token requests, the steps of a passkey login, requests that only send a code,
and the test buttons. One save of a client in the UI writes a few entries, one per request it makes.

The same line is also written to the log with the target `audit`, so a log collector such as Loki gets the trail too.

## Status Change history

Users, clients, providers and API keys have a tab **Status Change history** in their popup: who created it and when,
who changed it last and when, and every change in between, newest first. What was created before the audit log existed
has no creator on record, only a last change once someone changes it. By API:
`GET /auth/v1/events/audit/summary?target_id=<id>` for the two facts, and `GET /auth/v1/events/audit?target_id=<id>` for
the list.

## Deleted users

When a user is deleted, their id, email, names, username and phone are copied to a small table (`audit_users`) first. Old
entries keep naming them. Nothing refers to that table either. To remove the personal data on request (for example under
a deletion request), open **Users -> Deleted users**. It lists the deleted users with what is kept of them and a button
to wipe it; the same by API with `GET /auth/v1/events/audit/users` and `DELETE /auth/v1/events/audit/users/{id}`. Only the
id stays, and the entries about that user are shown by id.

## Reading it

There is no page with the whole log: it is read per thing, in the tab above. By API: `GET /auth/v1/events/audit` with an admin session or an API key that has `Events` read. Parameters:
`target_id` (only the entries about that user, client, ...), `limit` (1 to 200, default 50) and `before` (a Unix timestamp
in milliseconds, to page back; use the `at` of the last entry you got).

## Database Migration Tool

The Hiqlite <-> Postgres migration tool copies `audit_log` and `audit_users` together with the other tables, so the history
survives a change of database.

## Cost

One small row per change, indexed by time and by target, written in the background so the request does not wait for it.
A request that is not a change costs two string checks. Reminders add one entry per reminder sent (at most 100 per hour).
