# frontend-next — shadcn-svelte drop-in replacement for ../frontend

SvelteKit 2 + Svelte 5 (runes) + TS + Tailwind 4 + shadcn-svelte ("luma") + bits-ui + @lucide/svelte
+ TanStack Table v9. pnpm. Same routes, same base path `/auth/v1`, same prerendered-static output as
`../frontend`. The old UI is NOT modified (upstream-owned). Reference for copying components:
/home/tendo/GolandProjects/julaw/julaw/frontend.

## How it mirrors the old UI (read this first)
- Logic is SHARED, not copied: aliases `$api $state $utils $mfa $i18n $types $workers $wasm` point
  into `../frontend/src/*` (svelte.config.js). Upstream changes to API types, i18n texts, session
  logic, passkey code flow in automatically. Only presentation lives here.
- `$lib5` -> `src/lib`, `$icons` -> `src/icons` (lucide shims with the old Icon* names), so ported
  files keep their imports. Each `src/lib/**.svelte` = old file's `<script>` verbatim + new markup.
- Kit with the OLD props API on shadcn: `button/Button`, `Modal`, `form/Input*`, `Options`, `Tabs`,
  `SelectList`, `Expandable`, `Popover`, `Switch`, ... plus `Msg`, `FormSection`, `DetailsSheet`.
- Lists = julaw's DataTable (`components/data-table`, adapted: `clientSide`, `searchable`,
  `onRowClick`, `toolbar`, `rowActions`, `remove`). Column helpers: `data-table/columns.ts`.
  Server pagination (users/sessions) via `ssp.svelte.ts`. Details open in `DetailsSheet`.
- Backend theme vars (`--bg`, `--action`…) are bridged onto shadcn tokens (`rauthy-theme` in
  `routes/layout.css`) so client branding works.
- `app.html` / `hooks.server.js` keep the backend template placeholders: don't touch them.

## Commands
`pnpm check` · `pnpm build` (-> ./build, safe) · `pnpm build:dropin` (REPLACES ../templates/html +
../static/v1) · dev: `../local-dev/next.sh` (127.0.0.2:15174).
View next to the old UI: https://accounts-dev.timothy.online/__ui/next  (and /__ui/old to go back).

## Token-saving rules
- Never read `src/lib/components/ui/**` (generated shadcn), `node_modules`, `.svelte-kit`, `build`.
- Add shadcn parts: `pnpm dlx shadcn-svelte@latest add <name> -y --no-deps`.
- Old files not ported on purpose: SessionRow, pagination/*, search_bar/Order*, text_edit/html/*.
- When upstream changes `../frontend/src/lib/**.svelte`: re-port that file (script verbatim, new markup).

## Gotchas
- shadcn `ref` props have a null fallback: wrappers use an `inner` variable + `$effect` (see Button).
- `static` is a symlink to ../frontend/static (git-ignored by the repo's `static` rule).
- Dev mode: Vite UI needs backend DEV_MODE=true; see ../CLAUDE.local.md for the Origin/:443 workaround.
