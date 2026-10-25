# The New Admin UI

Rauthy ships with the original UI. This fork additionally contains a second UI, in `frontend-next/`, which uses
the same routes, the same API and the same texts, so it is a drop-in replacement. It is built with SvelteKit,
Tailwind and shadcn-svelte.

## What Is Different

- all lists are sortable, searchable and paginated tables with column selection, CSV export and keyboard navigation
- details open in a side panel, and dialogs offer **Save and add new** wherever you add many entries
- bulk messages to users, roles and groups by email, SMS or WhatsApp, see [Message Channels](message_channels.md)
- verification of emails and phone numbers, see [Verifying Email and Phone](verification.md)
- typed attributes and per-client required data, see [Custom Scopes and Attributes](custom_scopes_attributes.md)
  and [Required User Data per Client](required_client_data.md)
- `Config` pages for settings stored in the database: General, User fields, Sessions & tokens, Email (SMTP) and
  Channels, see [Settings in the Database](../config/settings_db.md)

## Building It

```bash
cd frontend-next
pnpm install
pnpm check
pnpm build:dropin
```

`build:dropin` **replaces** `templates/html` and `static/v1` with the new UI, which is what `cargo build` embeds.
Use `pnpm build` to build into `./build` only, without touching anything.

```admonish warning
Every admin page that exists as its own route also needs a page handler in the backend for a browser refresh to work
in production. The new `Config` pages (General, User fields, Sessions & tokens, Email, Channels) work when you
navigate to them inside the UI, but reloading them directly is not wired up yet.
```
