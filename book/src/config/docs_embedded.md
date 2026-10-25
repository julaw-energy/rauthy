# Built-in Documentation

Rauthy serves this documentation itself at `/auth/v1/book/`, so your users and admins read the docs of the exact
version they run, without leaving the server. The `Docs` page in the Admin UI links to it, next to the API reference.

## How It Is Built

The book is compiled into the binary, gzip-compressed (about 2.5 MB). It is not a runtime dependency and needs no
extra memory worth mentioning. Before `cargo build` (also in container builds and CI) run once:

```bash
cargo install mdbook@0.4.52 mdbook-admonish --locked
book/build-embed.sh
```

This fills `static/book/`. If the folder is missing, Rauthy still builds and simply answers `404` on `/book/`.
The embedded files are gzip-only, so a client that does not accept `gzip` gets `406`; all browsers do.

```admonish note
Use `mdbook` 0.4.x. The 0.5 series changed its config format and breaks the book's setup.
```

## Your Own Service Name

If you changed the service name (`server.app_name`, see [Settings in the Database](settings_db.md)), the docs show
that name instead of `Rauthy` in titles and text. Code blocks, commands and config keys are never touched.
