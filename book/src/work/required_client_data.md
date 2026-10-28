# Required User Data per Client

Some applications can only work when the user has certain data: a phone number, an address, a national ID.
Rauthy lets you say, **per client**, which user data must exist before a login for that client may finish.

## Set It Up

1. Open the client in the Admin UI (`Clients` → the client).
2. In the `Scopes` section, enable the scopes the client may use (allowed or default).
3. The **Required user data** section below lists what those scopes release, grouped by scope:

| Scope | What you can require |
|---|---|
| `profile` | given name, family name, username, birthdate, gender, time zone |
| `email` | a verified email address |
| `phone` | phone number |
| `address` | street, ZIP code, city, country |
| custom scope | every custom attribute mapped into that scope, see [Custom Scopes and Attributes](custom_scopes_attributes.md) |

4. Tick what is mandatory and save the client.

Data of a scope that is not enabled for the client cannot be required. If you disable a scope later, its
requirements are dropped on the next save.

## What Users See

When someone logs in to the client and something is missing:

1. The login is **not completed**; the user is told that their account needs updating.
2. The button takes them to their account page, which lists exactly what the client still needs.
3. Once everything is filled in, the same page shows **Continue**, which returns to the login. Since the
   session already exists, the client then gets its login.

Custom attributes are managed by admins unless you marked them as editable by users. For those the user is told
to ask an administrator.

```admonish info
This is checked at **every login** for the client, not just the first one. If you add a requirement later,
existing users are asked to complete their data the next time they sign in to that client.
```

```admonish note
The `rauthy` client (the Admin UI and account page) is never blocked by this, otherwise users could not reach the
page where they fix their data.
```

## How It Relates to the Global Settings

The global **User fields** settings (`Config` → `User fields`, see [Settings in the Database](../config/settings_db.md))
apply to *every* registration and profile edit. The per-client list is stricter only where one client needs more
than the rest, so your other applications are not forced to collect data they do not use.

## API

- `GET /clients/{id}/required_fields` returns `{"fields": ["phone", "city", "attr:nin"]}`
- `PUT /clients/{id}/required_fields` with the same body replaces the list. An empty list removes all requirements.
- `GET /users/{id}/missing_fields/{client_id}` returns what that user still lacks for the client. Users can read
  their own.

Built-in fields use their plain name (`given_name`, `family_name`, `preferred_username`, `email_verified`,
`birthdate`, `gender`, `tz`, `phone`, `street`, `zip`, `city`, `country`). Custom attributes are `attr:<name>`.

A profile field that is set to `hidden` under `Config` → `User fields` is marked **(hidden)** in the list. Hover it for the
hint: users cannot see or fill in a hidden field, so a client that requires it would lock them out. Set the field to
`optional` or `required` to unhide it. See [User Profile Fields](user_profile.md#what-hidden-means).
Unknown names are rejected.
