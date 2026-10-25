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
