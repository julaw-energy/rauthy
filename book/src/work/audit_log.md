# The System User and the Audit Log

## Who did it

Rauthy has a built-in user with the id `system`, the email `system@rauthy.invalid` and the name "System Background
tasks". It owns everything the server does on its own, without an admin clicking something. It cannot log in: it is
disabled, has no password and no passkey, and the server refuses every change to it or deletion of it. You will see it
in the user list. Leave it alone.

## The audit log

Every successful change made through the API is recorded: who did it (the id of the user, `api_key:<name>` for an API
key, or `system`), when (Unix milliseconds), what happened, and to what. Entries are only added, never changed, and are
not cleaned up automatically. Nothing in the log refers to a user by a foreign key, so it stays complete when users are
deleted.

| Field | Meaning |
|---|---|
| `actor` | The id of the user who did it, `api_key:<name>`, or `system`. `actor_label` is their email, if they are known. |
| `action` | `create`, `update`, `delete`, `action` (a change that is none of these, like sending a password reset), or a name for what the background tasks do: `inactivity_reminder_sent`, `user_expired_disabled`. |
| `target_type`, `target_id` | What it was about: `users`, `clients`, `roles`, `settings`, ... and its id or name. |
| `details` | For API changes the method and route, like `PUT /clients/{id}`. What was sent is never recorded, only the route. |

What is **not** recorded: reads, logins and token requests, the steps of a passkey login, requests that only send a code,
and the test buttons. One save of a client in the UI writes a few entries, one per request it makes.

The same line is also written to the log with the target `audit`, so a log collector such as Loki gets the trail too.

## Created and last changed by

`GET /auth/v1/events/audit/summary?target_id=<id>` tells who created something and who changed it last, read from the
log. The Admin UI shows it under the tabs of a user and of a client. What was created before the audit log existed has
no creator on record, only a last change once someone changes it.

## Deleted users

When a user is deleted, their id, email, names, username and phone are copied to a small table (`audit_users`) first. Old
entries keep naming them. Nothing refers to that table either. To remove the personal data on request (for example under
a deletion request), use **Config -> Audit log -> Deleted users**, or `DELETE /auth/v1/events/audit/users/{id}`: the
details are wiped and only the id stays. The entries about that user remain, shown by id.

## Reading it

**Config -> Audit log** in the Admin UI: newest first, filter by an id, "Older entries" to page back.

By API: `GET /auth/v1/events/audit` with an admin session or an API key that has `Events` read. Parameters:
`target_id` (only the entries about that user, client, ...), `limit` (1 to 200, default 50) and `before` (a Unix timestamp
in milliseconds, to page back; use the `at` of the last entry you got).

## Cost

One small row per change, indexed by time and by target, written in the background so the request does not wait for it.
A request that is not a change costs two string checks. Reminders add one entry per reminder sent (at most 100 per hour).
