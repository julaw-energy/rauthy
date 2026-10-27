# Access Rules per Client

A client can say who may log in to it, by what the user's values and attributes are. Only users in Uganda, Kenya and
Tanzania. Only users of 18 and older. Only users who have a membership number. You set this per client in the Admin UI,
in the client's **Access rules** section, and it is checked for every login and every token refresh.

This is a different thing than the login group prefix of a client, which looks at groups only, and
than [required user data](required_client_data.md), which asks users to fill something in. Access rules **refuse** a user
who does not qualify.

## Rules

A rule is a **field**, a **condition** and, for most conditions, a **value**. **All** rules of a client must hold.

| Field                                                                            | What it is                                        |
|----------------------------------------------------------------------------------|---------------------------------------------------|
| `country`, `city`, `zip`, `street`, `phone`, `gender`, `tz`, `birthdate`         | the user values                                   |
| `given_name`, `family_name`, `preferred_username`, `email`, `email_verified`     | the profile (`email_verified` is true / false)    |
| `age`                                                                            | full years, calculated from the `birthdate`       |
| `roles`, `groups`                                                                | lists                                             |
| `attr:<name>`                                                                    | a custom attribute                                |

| Condition                           | Meaning                                                              | Value                |
|-------------------------------------|----------------------------------------------------------------------|----------------------|
| `eq`, `ne`                          | is / is not                                                          | one value            |
| `gt`, `gte`, `lt`, `lte`            | greater than, at least, less than, at most. Numbers only             | a number             |
| `between`                           | in a range, **both ends included**. Numbers only                     | `[lowest, highest]`  |
| `in`, `not_in`                      | is one of / is none of                                               | a list               |
| `contains`, `starts_with`, `ends_with` | text (case does not matter), or an element of a list              | one value            |
| `is_set`, `not_set`                 | has a value / is empty                                               | none                 |

## The Editor

The input of a rule follows the **kind of its field**, so nobody has to type `true` or remember a country code:

| Field is a            | You get                                                                                         |
|-----------------------|-------------------------------------------------------------------------------------------------|
| number (`age`, a number attribute) | the conditions as math signs `=  ≠  >  ≥  <  ≤` and **between**, with two inputs for the range |
| true / false (`email verified`, a boolean attribute) | `is` and a true / false choice                                          |
| country               | a searchable country picker with flags. Several countries for "is one of" / "is none of"         |
| fixed choices (`gender`, an enum attribute) | a choice list, several for "is one of"                                    |
| `roles`, `groups`     | the existing roles / groups to choose from, with "includes", "includes any of", "includes none of" |
| date (`birthdate`)    | a date input. For comparisons use `age`, which is calculated from it                            |
| anything else         | a text input. "is one of" takes values typed with Enter or a comma                              |

Examples, as they are shown in the UI:

- `country` is one of `UG, KE, TZ`
- `age` is at least `18`, or `age` between `18` and `65`
- `email verified` is `true`
- `email` ends with `@julawenergy.com`
- `roles` contains `staff`
- `attr:membership` has a value

Numbers compare as numbers (`18` equals `18.0`), text without regard to case, and a list field (`roles`, `groups`, a list
attribute) matches when **any** of its elements does.

```admonish warning
**A missing value never meets a rule**, except `is empty`. "country is one of UG" does not let in a user who has no
country, and "country is not RW" does not either. Rules deny by default, so a client that is restricted by country
should also have the country in its [required user data](required_client_data.md): the user then fills it in and is not
turned away for a field they never saw.
```

## When It Is Checked

At every login (password, passkey, provider), every token refresh, every request through forward auth and the password
grant. A user who does not qualify gets *"Your account does not meet the requirements to access this application"*; the
log has the rule that failed. The `rauthy` client, which is the Admin UI, is never checked, so rules cannot lock the
admins out.

An access token that was issued already stays valid until it expires. The admin changes the user, or the rules, and the
effect is there at the next login or refresh.

## Cost

A client without rules costs one cache lookup per login. For a client with rules, the user's values and attributes are
read from the same caches as everywhere else, and only the ones a rule needs. The rules are at most 20 per client, with
at most 100 values in a list.

## API

`GET /auth/v1/clients/{id}/access_rules` and `PUT` with `{"rules": [{"field": "country", "op": "in", "value": ["UG",
"KE"]}]}`. `PUT` replaces all rules, an empty list removes them. The `GET` answer lists the fields, with their kind (`text`, `number`, `bool`, `date`, `country`, `choice`, `list`) and choices,
and the conditions that can be used. In JSON a true / false value is `true` or `false`, a range is `[18, 65]`. It needs the `Clients` permission of an API key, or an admin session.
