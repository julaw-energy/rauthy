# Settings in the Database

Most of Rauthy's config comes from `config.toml` and environment variables, which means a change
needs a restart, and in a container usually a new container. A growing set of settings can instead
live in the database and be changed from the admin UI (**Config**) without a restart:

| Group                   | Where in the UI           |
|-------------------------|---------------------------|
| Service name (`server.app_name`) | Config → General |
| User fields (`[user_values]`, except the preferred username rules) | Config → User fields |
| Session, token and link lifetimes (`[lifetimes]`, except the JWK rotation cron) | Config → Sessions & tokens |
| SMTP server (`[email]`) | Config → Email (SMTP)     |
| Message channels        | Config → Channels         |

The settings that Rauthy needs *before* it can reach its database (`[cluster]`, `[database]`,
encryption keys, listen addresses, TLS, `PUB_URL`) always stay in the config file.

## Rauthy Starts Without a Mail Server

Rauthy does not need a working SMTP server to start. As long as none is set up, or the configured
one cannot be reached, emails are skipped with a log line (and an event for failed sends) and the
admin UI shows a banner with a link to the setup page. As soon as the server works again, mails go
out again; no restart is needed.

## Moving Your Config File Values Into the Database

On the **first start** after upgrading, the values you already have in the file are copied into the
database once ("seeded"), and Rauthy logs which groups it moved. From then on the database is
authoritative for these groups:

- the matching keys can be removed from `config.toml` / the environment
- changes are made in the admin UI and take effect immediately on the node where you save them, and
  on all other nodes within about 15 seconds
- the admin UI offers **Import from config file** per group, which replaces the database values with
  whatever the file / environment says right now

If a group has no database entry yet (fresh install), the file values are used as they are.

~~~admonish note
`smtp_conn_mode = "xoauth2"` and `"microsoft_graph"` stay file-only for now. Everything else about
the built-in mailer is managed in the database.
~~~

## Secrets

The SMTP password and the API keys of notification agents are stored **encrypted** with the active
encryption key (see [Encryption](encryption.md)) and are never sent back by the API. The UI shows
only whether a secret is set; leaving the field empty keeps the stored value.

## API

All endpoints need an admin session. See the [API reference](../scalar.md) for details.

| Endpoint                                | Purpose                                         |
|-----------------------------------------|-------------------------------------------------|
| `GET /settings`                         | current settings, without secrets               |
| `PUT /settings/smtp`                    | change the SMTP settings                        |
| `POST /settings/smtp/test`              | send a test email to the signed-in admin        |
| `PUT /settings/channel/{channel}`       | change a message channel                        |
| `POST /settings/channel/{channel}/test` | check that the notification agent is reachable  |
| `POST /settings/import/{group}`         | re-import `smtp` or `channel.<name>` from file  |

## The Service Name

`server.app_name` (env `APP_NAME`, default `Rauthy`) is the name your users see: page titles, the
admin UI, the texts of the login and account pages, emails and these docs show it instead of the word
"Rauthy". Set it in the config file for a fresh install, or change it any time under **Config →
General**. Names of users, clients and data are never touched, only the product's own texts, and code
blocks in the docs stay as they are.

The name shown in passkey prompts (`webauthn.rp_name`) follows it unless set explicitly, and needs a
restart. Sessions already open pick a new name up on their next page load.
