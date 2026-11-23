# Roadmap of our fork

What is planned for the next releases of `julaw-energy/rauthy` (branch `custom`). Released changes are in
`CHANGELOG_CUSTOM.md`. A pushed tag is never moved: every fix goes out as the next `-custom.N`.

## Next: v0.37.2-custom.11

- **Fix the (i) explanations with bold or code text.** In custom.10 the box of such a tip (for example "Access rules" and
  "Required fields") is laid out as a column of single words. The fix is already committed (`11b38b39`, `InfoTip.svelte`).
  Not released yet, so custom.10 keeps the broken tips until then. Changelog: a **Bugfix** entry, no migration.
- Ship together with it: the book update (new screenshots, Connected apps, (i) tips, S3 pictures), commit `3a87ab49`.

## Later

- **Check the "Trust MFA of this provider" checkbox** in the new UI. It was not found in the provider dialog, although the
  changelog says it exists.
- **Book:** the screenshots still from the old UI (`api_key_2`, `api_key_test`, `api_key_permission`,
  `4_rauthy_github_login`, `1_github_app_create`), a page for the Config pages (New users, Static settings), and the cache
  clear on a build change.
- **Check the PAM page:** placement of the "Add" button after the table toolbar change.
- **Production rollout** of the custom releases (see the rollout notes in the maintainer's memory).
- **FedCM / silent sign-in:** stays experimental, not used by our apps for now. Look again when the spec and the browsers
  settle.
- **Clean up dev leftovers:** `signin-lab`, the Traefik routes `accounts-dev-lab-authorize` and `accounts-dev-fedcm-apex`,
  the `signin_lab` and `pkce_test_app` clients.
