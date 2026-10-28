# User Profile Fields

Besides email and name, Rauthy stores a few more standard OpenID Connect claims per user. They are released with the
`profile`, `phone` and `address` scopes.

| Field | Claim | Notes |
|---|---|---|
| Birthdate | `birthdate` | A real date between 1900 and today |
| Gender | `gender` | `female`, `male` or `other` |
| Phone | `phone_number` | International format `+256700216664` (E.164, 8 to 15 digits). Can be verified, see [Verifying Email and Phone](verification.md) |
| Street, ZIP, City | `address` | |
| Country | `address` | ISO 3166-1 alpha-2 code like `UG`. The UI shows a country list; older free-text values are still accepted |
| Time zone | `zoneinfo` | An IANA name like `Africa/Kampala` |

## Required, Optional or Hidden

For each field you decide if it is `required`, `optional` or `hidden`, for registration and profile editing. This is
in `Config` → `User fields` in the Admin UI. There you also choose:

- whether users are checked again at every client login (`revalidate_during_login`), and
- the order in which names are shown.

These settings live in the database and apply immediately. On the first start they are copied from the
`[user_values]` section of the config file, see [Settings in the Database](../config/settings_db.md).
The username rules (`preferred_username`, with its blacklist and regex) stay in the config file.

To require more only for a single application, see [Required User Data per Client](required_client_data.md).

## Connected Apps

Every time a client gets tokens for a user, Rauthy remembers the client, the scope, the first and last access and the
number of logins. Users see this on their account page under **Connected apps**, and admins see it on the user.
It answers "which applications can see my data". The list can be read with `GET /users/{id}/clients`.

## Profile Pictures of Federated Users

When a user logs in through an auth provider (Google, Github, ...), the `picture` claim of the provider (`avatar_url` for
Github) is stored and kept current on every login. It is shown in the account and admin UI, and it is the `picture`
claim of the ID token and of userinfo for client applications (scope `profile`).

- Only `https` links are taken. Rauthy never fetches the image, the browser loads it from the provider.
- A picture the user uploads replaces the link, Rauthy then serves its own URL.
- There is no migration: the link is kept in the existing `picture_id` column.
- A user gets the picture at their next login through the provider.

```admonish warning title="Content-Security-Policy"
Every page carries a CSP. Its `img-src` is `'self' data: https:` so the browser may load the provider's image. Plain
`http` images stay blocked and scripts are still locked to `'self'` and hashes. The trade-off: an injected `<img>` could
send data to any `https` host. If that is not acceptable, the alternative is a small avatar proxy in Rauthy (more code,
and Rauthy would then fetch the provider's URL).
```
