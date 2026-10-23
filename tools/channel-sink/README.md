# Channel sink

A tiny stand-in for a notification agent, to test Rauthy's SMS / WhatsApp channels and emails
without sending anything to anyone. It receives what Rauthy would hand to a real provider, checks
the signature or key, and lists the messages on a web page.

One file, Python 3.8+, **no dependencies**.

```bash
VIEW_TOKEN=pick-a-long-random-string SIGNING_SECRET=another-secret SMTP_PORT=2525 python3 sink.py
```

or with Docker:

```bash
docker run -d --name channel-sink -p 3400:3400 -p 2525:2525 \
  -e VIEW_TOKEN=... -e SIGNING_SECRET=... -e SMTP_PORT=2525 \
  -v "$PWD/sink.py:/sink.py:ro" python:3-alpine python /sink.py
```

Then open `http://your-server:3400/?token=<VIEW_TOKEN>`.

> **Verification codes show up in clear text on that page.** Always set `VIEW_TOKEN`, put it behind
> HTTPS (a reverse proxy) when it is reachable from the internet, and stop it when you are done.

## Pointing Rauthy at it

In the admin UI under **Config → Channels** (or in `config.toml` / env, see the Channels docs):

| What to test        | Driver    | URL                           | Key                       |
|---------------------|-----------|-------------------------------|---------------------------|
| webhook format      | `webhook` | `http://your-server:3400/hook`| = `SIGNING_SECRET`        |
| notifyd adapter     | `notifyd` | `http://your-server:3400`     | = `API_KEY` (if you set it)|

"Test the agent" works for both. Under **Config → Email (SMTP)**, choose *None / insecure*, set
the server to `your-server` and the port to `2525` (`SMTP_PORT`) to see emails in the same list.

## What it checks

- `POST /hook`: `X-Rauthy-Signature: sha256=<hmac of the raw body>` against `SIGNING_SECRET`
  (rejected with 401 if wrong), answers `{"job_id": ...}`, and `{"pong": true}` to Rauthy's ping.
- `POST /v1/send` (notifyd): `X-Api-Key` against `API_KEY`, records `to`, `body`, `priority`,
  `transactional` and the idempotency key.
- `GET /v1/health`: `{"status":"ok"}`.
- `GET /api/messages` (JSON) and `DELETE /api/messages` (clear), both need the view token.

Messages live in memory only (the last `MAX_MESSAGES`, default 200) and are gone on restart.
