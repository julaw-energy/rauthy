# API Reference (Scalar)

Rauthy has its own OpenAPI document and shows it with [Scalar](https://scalar.com). The Swagger UI that older versions
shipped has been removed, which also saves ~13 MB of memory and a large chunk of the binary.

It is disabled by default. Rauthy is open source and anyone can start it and read the API docs, but exposing them in
production is not good practice, because it increases the attack surface. Enable it with:

```toml
[server]
# default: false
# overwritten by: SCALAR_ENABLE
scalar_enable = true
```

Scalar is loaded from a CDN, pinned to one version. Nothing is bundled into Rauthy and it costs no extra memory, the
OpenAPI document is built per request. The page is at `/auth/v1/docs/` (and `/auth/v1/docs/scalar`), and the `Docs`
page in the Admin UI links to it. Scalar has a built-in client and uses your **current session** for requests, so no
API key is needed to try endpoints. API keys can still be used from outside with
`Authorization: API-Key <name>$<secret>`.

## External Access

By default, you can only open the page with a valid session. If non-logged-in users need access too:

```toml
[server]
# default: false
# overwritten by: SCALAR_PUBLIC
scalar_public = true
```

```admonish note
`swagger_ui_enable` is ignored since the Swagger UI is gone. `swagger_ui_public` still works as an alias for
`scalar_public`, so an existing config keeps its meaning.
```

## The Documentation Book

The documentation you are reading is served by Rauthy itself at `/auth/v1/book/`. It describes configuration and
internals, so it needs a logged-in session as well. `book_public = true` (env `BOOK_PUBLIC`) opens it to everyone.
