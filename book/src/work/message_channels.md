# Message Channels

Besides the built-in mailer, Rauthy can deliver messages through an external **notification agent** or provider.
This is used for admin messages to users, verification codes and the system emails, and it lets you add SMS, WhatsApp or
anything else without Rauthy knowing anything about the providers.

## Sending a Message

In the admin UI, the envelope button on the **Users**, **Roles** and **Groups** tables opens the
send dialog. Pick any number of roles, groups and users (a user receives it if it matches any of
them), optionally exclude roles or groups, and pick the channel. Selecting nothing means all
users. With a channel other than email, the message is plain text and users without a phone number
are skipped. Messages can be scheduled and are processed in batches like any bulk email job.

Delegated [group admins](group_admins.md) can only pick groups they manage.

## Channels and Drivers

`email` and `sms` always exist: they deliver to the email address and the phone number every user has. Every channel
has a **driver**:

| Driver    | Meaning                                                                                              |
|-----------|------------------------------------------------------------------------------------------------------|
| `smtp`    | email through an SMTP server. For `email` it is the server under Config → Email (SMTP); a custom channel has its **own server and credentials** |
| `webhook` | Rauthy POSTs its own signed message format to a URL you provide, for a communication agent you run   |
| `http`    | you describe the call to any provider: method, URL, headers and body attributes, secrets marked      |
| empty     | the channel is disabled (`email` and `sms` only)                                                     |

Configure them under **Config → Channels**. The built-in channels can also be seeded from the config file (`smtp` and
`webhook` only):

```toml
[channels.sms]
driver = "webhook"
url = "https://agent.example.com/hook"
api_key = "signing secret"
```

The same values exist as environment variables, `CHANNEL_<EMAIL|SMS>_<DRIVER|URL|API_KEY>`.
See [Settings in the Database](../config/settings_db.md) for how file values and the UI interact.

```admonish note
The `notifyd` driver has been removed. Use `webhook` (or `http`). A channel stored with `notifyd` stays disabled until
you choose a new driver.
```

## System Emails Follow the Email Channel

Password resets, one-time codes, security notices and every other email go through the `email` channel. With the `smtp`
driver they are sent by the built-in mailer as before. If you switch the channel to `webhook` or `http`, **all** of them
are handed to that agent instead, and Rauthy no longer needs the SMTP credentials at all: the agent holds them, and you
only whitelist the IP of your Rauthy nodes there. A failed hand-over is retried once, then logged and raised as an
event, like a failed SMTP send. (With the Microsoft Graph connection mode, the mailer keeps its own path.)

## The HTTP Driver

For providers that do not speak Rauthy's format, describe the call yourself. Everything is stored in the database,
secrets encrypted, and decrypted once when the settings load, not for every message.

- **Method and URL** (`POST`, `PUT` or `PATCH`). Placeholders in the URL are URL-encoded.
- **Headers**, for example `Authorization`.
- **Body** as JSON or as a form. Each attribute has a name and a value. In JSON a name like `messages.0.to` builds
  nested objects and lists.
- Tick **secret** on keys and tokens: they are encrypted, never shown again, and used exactly as typed (placeholders are
  not replaced in them). Leave a stored secret empty when saving to keep it.

Values can contain `{{to}}` (the recipient, required somewhere), `{{text}}` (the message, required somewhere),
`{{subject}}`, `{{html}}`, `{{name}}`, `{{purpose}}`, `{{channel}}`, `{{app_name}}` and `{{idempotency_key}}`.
Any `2xx` answer counts as delivered. Starting points for a JSON API and a form API with an auth header are in the UI.

## Channels With Their Own SMTP Server

A custom channel with the `smtp` driver is a copy of the default mailer with its own details: server, port, encryption,
username, password (stored encrypted) and sender. Use it to send, for example, newsletters or a second brand through a
different account.

## The Message Format (Webhook Driver)

Rauthy always builds the same message, whatever agent is behind a channel. That way the agent can be
exchanged without touching anything else. The `webhook` driver sends it as JSON (version 1):

```json
{
  "version": 1,
  "channel": "sms",
  "purpose": "verification",
  "recipient": { "user_id": "...", "name": "...", "to": "+256700000000" },
  "subject": null,
  "text": "Your Rauthy verification code is 123456. It is valid for 10 minutes.",
  "html": null,
  "idempotency_key": null
}
```

`purpose` is `admin` (bulk messages), `verification` or `security`. Any `2xx` answer counts as delivered; an optional
`{"job_id": "..."}` in the answer is logged.

### Authenticity and replay protection

The request is sent over TLS, which keeps it confidential. A signing secret additionally lets the receiver check that it
came from Rauthy and was not changed. If one is set, the request carries:

| Header | Value |
|--------|-------|
| `X-Rauthy-Signature` | `sha256=<hex>`, HMAC-SHA256 of the raw body |
| `X-Rauthy-Timestamp` | unix time in seconds when the request was made |
| `X-Rauthy-Signature-Timestamped` | `sha256=<hex>`, HMAC-SHA256 of `<timestamp>.<raw body>` |

Verify the **timestamped** signature and reject a request whose timestamp is older than a few minutes. That stops
someone who captured a request from sending it again. `X-Rauthy-Signature` stays for receivers that only check the body.

Every request has an `Idempotency-Key` header (the same value as `idempotency_key` in the body). A retry of the same
message, for example a system email that is handed over a second time, carries the same key, so the agent can drop
duplicates.

```python
import hmac, hashlib, time

def verify(body: bytes, headers: dict, secret: str, max_age=300) -> bool:
    ts = int(headers["X-Rauthy-Timestamp"])
    if abs(time.time() - ts) > max_age:
        return False
    mac = hmac.new(secret.encode(), f"{ts}.".encode() + body, hashlib.sha256).hexdigest()
    return hmac.compare_digest(f"sha256={mac}", headers["X-Rauthy-Signature-Timestamped"])
```

## When Delivery Fails

A bulk job skips a user whose delivery fails and carries on. After 5 failures in a row the job
stops, because the agent is probably down. The **Check connection** button in the UI checks
reachability (webhook) or the login (own SMTP server) without sending anything to a user.

## Testing Without a Real Provider

`tools/channel-sink` in the repository is a single dependency-free Python script that plays the notification agent
for tests. It accepts Rauthy's webhook messages (and checks the signature) and,
optionally, plain SMTP, and shows everything it received on a small web page. Run it on any server, point the
channels at it and watch the codes and bulk messages arrive.

```bash
VIEW_TOKEN=change-me SIGNING_SECRET=... SMTP_PORT=2525 python3 tools/channel-sink/sink.py
```

```admonish warning
Verification codes are shown in clear text. Always set `VIEW_TOKEN`, and never use the sink for real users.
```

See `tools/channel-sink/README.md` for the Rauthy settings to use against it.


## Custom Channels and Recipients

Under **Config → Channels → Add a channel** you can create more channels, for example `whatsapp_user`. A custom channel
uses the `smtp`, `webhook` or `http` driver.

`email` and `sms` always deliver to the user's email address and phone number. A custom channel is tied to the user
value it delivers to (**Delivers to**):

| Value | Delivers to |
|-------|-------------|
| `email` | the user's email address |
| `phone` | the user's phone number |
| `attr:<name>` | the value of a custom attribute, like `attr:whatsapp_username` |

A user without that value is skipped for a bulk message, and a single message to them is rejected. The recipient is
looked up from memory-cached user data, so this adds no database round trip.

**Send a test to me** delivers a real message to the logged-in admin, to the value the channel is tied to. If the admin
has none (no phone number, for example), the test fails. On success it shows what the other side answered (the HTTP
status and the start of the answer), so you can see that the message really arrived there.

## Verifying Attributes Through Channels

A custom attribute of type *Text (verifiable through a channel)* (`verified`), or one of the verifiable email / phone
types, is linked to the channels that may verify it: open the attribute and choose **Verified through**.

- The code goes to the **value of the attribute** through the driver of the channel the user picks. The first linked
  channel is the default.
- A channel can only be linked if it can deliver to the attribute: it is tied to the attribute itself (`attr:<name>`),
  or to `email` / `phone` for attributes of the matching type.
- `verified_email` without links uses the `email` channel and `verified_phone` the `sms` channel, as before.
- A `verified` attribute without links cannot be verified until a channel is linked.
- The user's own email address and phone number can be verified through any channel tied to `email` / `phone`.

Note: if the `email` channel uses the `webhook` or `http` driver instead of SMTP, verification codes for email
addresses go through it, like every other message of that channel.

## Expiry and Deletion

A custom channel can have an **expiry date**. When it is reached the channel stops delivering: sends through it are
rejected, and it disappears from the pickers. Tell your users before, with a message through that channel. An admin can
change the date at any time, or clear it.

Only an **expired** channel can be deleted (the delete button stays disabled until then; **Expire now** closes a channel
right away). Deleting also removes the links of attributes to it. `email` and `sms` cannot expire or be deleted, only
disabled.
