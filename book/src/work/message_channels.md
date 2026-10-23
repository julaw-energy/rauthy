# Message Channels (Email, SMS, WhatsApp)

Besides the built-in mailer, Rauthy can deliver messages through an external **notification agent**.
This is used for admin messages to users and for verification codes, and it lets you add SMS or
WhatsApp without Rauthy knowing anything about the providers.

## Sending a Message

In the admin UI, the envelope button on the **Users**, **Roles** and **Groups** tables opens the
send dialog. Pick any number of roles, groups and users (a user receives it if it matches any of
them), optionally exclude roles or groups, and pick the channel. Selecting nothing means all
users. With a channel other than email, the message is plain text and users without a phone number
are skipped. Messages can be scheduled and are processed in batches like any bulk email job.

Delegated [group admins](group_admins.md) can only pick groups they manage.

## Channels and Drivers

Every channel (`email`, `sms`, `whatsapp`) has a **driver**:

| Driver    | Meaning                                                                         |
|-----------|---------------------------------------------------------------------------------|
| `smtp`    | the built-in mailer, only for the `email` channel (the default)                 |
| `notifyd` | adapter for [notifyd](https://github.com/rmzlb/notifyd), uses `POST /v1/send`   |
| `webhook` | generic: Rauthy POSTs its own message format to a URL you provide               |
| empty     | the channel is disabled and does not show up in the UI                          |

Configure them under **Config → Channels**, or seed them from the config file:

```toml
[channels.sms]
driver = "notifyd"
url = "http://notifyd:3400"
api_key = "sk_myapp_..."
channel_name = "sms"     # the name the agent knows this channel by
```

The same values exist as environment variables, `CHANNEL_<EMAIL|SMS|WHATSAPP>_<DRIVER|URL|API_KEY|NAME>`.
See [Settings in the Database](../config/settings_db.md) for how file values and the UI interact.

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

`purpose` is `admin` (bulk messages), `verification` or `security`. If an API key / signing secret
is set, the request carries `X-Rauthy-Signature: sha256=<hex>`, an HMAC-SHA256 of the raw body, so
the receiver can check that it came from Rauthy. Any `2xx` answer counts as delivered; an optional
`{"job_id": "..."}` in the answer is logged.

## notifyd

The `notifyd` driver maps the message to notifyd's `POST /v1/send` (`channel`, `to`, `body`,
`subject`, `priority`, `transactional`, `idempotency_key`) with your key in `X-Api-Key`. Admin
messages are sent with `bulk` priority, verification codes as `critical` and `transactional` so
they skip subscription preferences. The recipient is passed as `to`, so the users do not need to be
registered in notifyd first.

## When Delivery Fails

A bulk job skips a user whose delivery fails and carries on. After 5 failures in a row the job
stops, because the agent is probably down. The **Test the agent** button in the UI checks
reachability without sending anything to a user.
