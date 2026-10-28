# The System User and the Audit Log

## Who did it

Rauthy has a built-in user with the id `system`, the email `system@rauthy.invalid` and the name "System Background
tasks". It owns everything the server does on its own, without an admin clicking something. It cannot log in: it is
disabled, has no password and no passkey, and the server refuses every change to it or deletion of it. You will see it
in the user list. Leave it alone.

## The audit log

Every entry says **who** (`actor`: the id of a user, or `system`), **what** (`action`), on **what** (`target_type`,
`target_id`), **when** (`at`, Unix seconds) and a short `details` text. Entries are only added, never changed, and are
not cleaned up automatically.

What is written today:

| Action | Actor | Meaning |
|---|---|---|
| `inactivity_reminder_sent` | `system` | The reminder for a user who has not logged in was queued. |
| `user_expired_disabled` | `system` | The user expiry scheduler disabled an account whose expiry date had passed. |
| `settings_changed` | the admin | The "new users" or "reminders" settings were saved. |

## Reading it

`GET /auth/v1/events/audit` with an admin session or an API key that has `Events` read. Parameters: `target_id` (only
the entries about that user), `limit` (1 to 200, default 50) and `before` (a Unix timestamp, to page back; use the `at`
of the last entry you got). Newest first.

```
GET /auth/v1/events/audit?target_id=<user id>&limit=20
```

## Cost

One small row per entry, indexed by time and by target. Reminders add one entry per reminder sent (at most 100 per hour).
