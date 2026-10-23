# Verifying Email Addresses and Phone Numbers

Users can prove that an email address or phone number is theirs with a one-time code.

## What Can Be Verified

- the **email address** of the account (`email_verified`, which Rauthy already stored)
- the **phone number** of the account
- custom attributes of the type **Email address (verifiable)** or **Phone number (verifiable)**,
  see [Custom Scopes and Attributes](custom_scopes_attributes.md)

Where a value can be verified, the account page and the admin user panel show a **Verified** or
**Not verified** badge with a **Verify** button.

## How It Works

1. The user clicks **Verify**. Rauthy sends a 6-digit code to *that value*: an email for addresses,
   an SMS for phone numbers (this needs an SMS channel, see [Message Channels](message_channels.md)).
2. The user types the code in. If it is right, the value is marked as verified.

A code is valid for **10 minutes**, allows **5 wrong guesses**, and a new one can be requested after
**60 seconds**. Only a hash of the code is stored.

A verification belongs to the value it was done for. If the user changes the phone number or the
attribute value, it is **not verified** anymore until verified again.

Admins can additionally mark a phone number or attribute value as verified, or revoke it, without
a code.

## Tokens and Userinfo

With the `phone` scope, `phone_number_verified` reflects the real state. `email_verified` comes with
the `email` scope.

## API

| Endpoint                                              | Who          |
|-------------------------------------------------------|--------------|
| `GET /users/{id}/verification`                        | user / admin |
| `POST /users/{id}/verification/{email\|phone}/start`   | user / admin |
| `POST /users/{id}/verification/{email\|phone}/confirm` | user / admin |
| `PUT /users/{id}/verification/{email\|phone}`          | admin        |
| `GET\|PUT /users/{id}/attr/{name}/verification`        | user / admin (PUT: admin) |
| `POST /users/{id}/attr/{name}/verification/start\|confirm` | user / admin |
