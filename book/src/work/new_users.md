# New Users: Default Roles and Reminders

Both settings are under **Config → New users** in the Admin UI and are stored in the database, like the other settings
there. A change is used right away on the node that saved it, and within 15 seconds on all others.

## Default Roles and Groups

Pick roles and groups, and every **new user** gets them. New users are the ones who register themselves, and the ones
who are created by their first login through an [auth provider](../auth_providers/index.md) (Google, GitHub, ...). Users that an admin creates in
the UI or the API are **not** changed: the admin chooses their roles there.

- Roles and groups that Rauthy comes with can be chosen, as well as your own.
- `rauthy_admin` and the delegated `rauthy_admin:<prefix>` roles cannot be chosen. They would make every new user an
  admin.
- A role or group that you delete later is skipped when a user is created. It never makes a registration fail.
- Existing users are not changed when you change the defaults.

## Reminders for Users Who Stay Away

A user who did not log in for a number of days gets an email. After the same time, another one, up to a number of
reminders in a row, until they log in again. A user who never logged in counts from the day the account was created.

| Setting              | Default | Range      |
|----------------------|---------|------------|
| Send reminders       | **off** | on / off   |
| Days without a login | 30      | 1 - 3650   |
| Reminders in a row   | 3       | 1 - 10     |

```admonish warning
It is **off** by default. When you turn it on, every user who has already been away for longer than the time is due at
once. The page shows how many that are. They are reminded over the next hours, at most 100 per hour, so the mail server
is not hit by one burst.
```

The subject and the text can be changed. They can use `{{name}}`, `{{days}}` (the days since the last login), `{{link}}`
(the account page of this Rauthy) and `{{app_name}}`; the subject cannot use the link. Unknown placeholders are refused
when you save. The same text goes to every language. **Save and send a test to me** saves and sends the reminder to your
own address, so you see the real mail before any user does.

Reminders are sent by the leader node only, once an hour, and only to enabled users whose account has not expired.
Which user got how many reminders is remembered, so nobody gets the same one twice, and the count starts again when a
user logs in.

### Large Installations

The hourly run does not look at every user: it looks at users who became inactive during the last 3 days, plus users
who are in the middle of their round. Once a day, and right after a start, it looks at everyone. With a million users
and most of them long gone, that is an indexed range lookup of a few milliseconds, instead of a scan of the whole user
table every hour.
