# Changelog of our fork

This file is for the changes of our fork (`julaw-energy/rauthy`, branch `custom`). Upstream's own history stays in
`CHANGELOG.md`, which we do not edit, so merging a new upstream release never conflicts here.

A release is named `<upstream version>-custom.<n>`, for example `0.37.2-custom.2` is upstream v0.37.2 plus our second
release. The counter starts again at 1 with every new upstream version. The sections are the same as upstream's:
**Security**, **Breaking**, **Changes**, **Bugfix**. A release note says what an operator has to do (config, migration,
rollout), not only what changed.

---

## v0.37.2-custom.10 (2026-11-23)

No migration. **Security: update soon**, one request could stop the server. Also: sign-in with no click (FedCM) for trusted
apps, an S3 server that is down no longer stops the start, "Unlink" asks first.

### Security

- **One request with an empty or short cookie value stopped the whole server.** The encryption library panics when an
  encrypted value is shorter than its 6 byte header, and Rauthy aborts on a panic. Anyone could send
  `Cookie: __Host-RauthySession=` to any endpoint, with no login, and the process was gone until it was restarted. This is
  an upstream bug that every earlier release has. All cookie, MFA cookie and forward auth values from a client go through a
  length check now. Nothing to do for you but update.

- **FedCM applies the same checks as the normal login.** Until now a FedCM token skipped the group prefix, `force_mfa`,
  required user data and access rules of a client, so FedCM was a way around them. These are refused now. A token also
  needs a nonce from the app, carries the real `sid`, `auth_time` and `amr` of the session, and origins are matched exactly
  (the old matching accepted a prefix or a suffix). Confidential clients get no FedCM token unless you list them in
  `[fedcm] confidential_client_ids`. `/fed_cm/status` refuses cross-site requests.

### Bugfix

- **An S3 server that is not reachable at startup no longer stops Rauthy.** With `user_pictures.storage_type = 's3'`
  Rauthy listed the bucket at boot and panicked when that failed, so a MinIO that itself needs the OIDC discovery of
  Rauthy to start (and Rauthy needing MinIO) could never come up. Rauthy starts and serves at once now. The bucket is
  checked in the background: a warning every 10 seconds, an error after 5 minutes, it keeps trying and uses the bucket
  as soon as it is there, no restart needed. Only the picture requests fail meanwhile. A wrong configuration (missing
  URL, key, bucket name) still stops the start. Hiqlite's own S3 use (backups) does not touch the network at startup.

- **No more `Bincode: invalid length 26, expected struct Client with 27 elements` after an upgrade.** The cache is kept
  on disk (`cache_storage_disk`), and after an upgrade the old cached copies of a changed struct could not be read, which
  logged an ERROR at the first start and skipped the anti-lockout check once. The cache only holds copies of the
  database, so it is now dropped once whenever the build changes (the image carries `RAUTHY_BUILD_ID`, our version). The
  log says `Build changed from ... to ..., clearing the cache`. Nothing to do for you; the first start of this release
  does it, so the error does not show up.

### Changes

- **UI.** Long section descriptions (access rules, redirect URIs, scopes, ...) are an (i) tip next to the title. Dark-mode
  inputs, selects and text areas have a visible border and fill. On tables, *Entries* comes first and *Add new* sits at
  the right edge. The multi-value input (redirect URIs, ...) keeps its outer border and shows its typing field only
  while it has the focus.

- **Sign in with no click (FedCM) for trusted apps.** List an app in `[fedcm] silent_client_ids` and the browser signs the
  user in without a prompt, once that user has signed in to it. Removing an app in *Connected apps* ends it. The well-known
  file now has `accounts_endpoint` and `login_url` (Chrome 145), and the nonce may come in `params`. See the book,
  *Sign in With No Click (FedCM)*. `tools/signin-lab` is a test app for it.

- Removing an app in *Connected apps* also removes it from that list until the user signs in to it again.

- **Phones (new UI):** the account page tabs were centered in a way that cut off the first ones when they did not fit
  (the "Info" tab and the MFA tab could not be reached); they scroll from the start now. The date format and the language
  pickers are icons on a narrow screen, the "CSV" and "Deleted users" buttons too, and the search of the user list has its
  own line. Dialogs keep a margin to the screen edge and no longer push their content out of the box.
- **More phone and header work (new UI):** a search box is only an icon on a phone until it is tapped (lists of users,
  sessions and every table with a search); the date format and language pickers open a small popup with the choices on
  a phone, and the language picker has a globe icon everywhere; the controls in the header are closer together; the tick
  in the pickers is closer to its text; the account page stays at the top while you switch tabs instead of moving to the
  middle of the screen; the id in "Deleted users" stays on one line (the table scrolls sideways).
- **Profile picture:** tapping it only shows it bigger (no more upload prompt); the picture is changed with the pencil
  on its corner, or with "Change picture" in the enlarged view.
- **The service name** (`server.app_name`, "Julaw Accounts" for you) now also shows in the (i) explanations and in the
  help texts of the settings pages, where they said "Rauthy".
- **Explanations (i):** a small info icon explains a static setting. On a computer it shows on hover or keyboard focus, on
  a touch screen it opens on a tap (there is no hover there). The component (`InfoTip`) can be used anywhere.
- **"Unlink federation" asks first.** The button opens a dialog that says what happens: no more sign-in with the
  provider, only your own password or passkey is left, the provider's own MFA no longer counts, the account stays. The
  unlink only runs after "Unlink" in the dialog.
- `tools/upgrade/chain_test.py` warms the cache before every hop and fails on `Bincode` errors.
- `tools/upgrade/ldap_smoke.py`: tests the LDAP server of an image end to end (off by default, group prefix and our access
  rules apply to binds, own-entry search). Tooling only.

## v0.37.2-custom.9 (2026-11-16)

UI only, no migration and no config change. A build of custom.8 can be replaced by this one without any step.

### Changes

- **Consents in the "Connected apps" tab of the new UI.** Upstream's consent feature (`[consent]`, off by default) records
  what each app may do and lets the user revoke it. Our "Connected apps" tab shows the scopes, the resource and the date
  of a recorded consent next to the app, with a two-step "Revoke access" button (revoking removes the refresh tokens of
  the app). Nothing changes while consents are off. Upstream has no consent step at login, so there is nothing else to port.
- **SvelteKit 3 and current tools for the new UI** (`frontend-next`): SvelteKit 3.2, adapter-static 4, Vite 8.3.8,
  Svelte 5.57.3, Tailwind 4.3.5. This is the same major version upstream's UI has now. The SvelteKit configuration moved
  from `svelte.config.js` into `vite.config.ts` (SvelteKit 3 does not read the old file), `$lib` is kept as an alias for
  the shadcn components. All 46 pages pass the build check.

### Docs

- custom.8's notes claimed that `force_mfa` is enforced on all user grants. That was wrong: upstream checks it when a
  device is approved, the password grant and the token exchange cannot do MFA and do not check it. Corrected here and
  in the GitHub Release of custom.8.

## v0.37.2-custom.8 (2026-11-16)

Merges upstream `main` (`b76a0728`, 12 Nov 2026, 55 commits after 0.37.2, not a tagged upstream release yet) and moves the
`trust_mfa` migration of custom.7 to where it belongs. Read the upstream notes in `CHANGELOG.md` ("UNRELEASED") as well.

### Upgrade

- **Back up first, restart ALL nodes at once.** Upstream binds refresh tokens to their client and adds user consents
  (two new migrations). Do not do a rolling release, and a rollback to an older build is not supported.
- **Forward Auth: invalidate all sessions after the upgrade** (Admin UI > Sessions). The cookie was replayable as a
  Rauthy session until its session ends.
- Databases that ran custom.1 to custom.7 are adopted at the first start (log line `Adopting the fork migration`),
  before upstream's migrations run. Hiqlite and Postgres. Tested one step at a time: upstream 0.37.2, custom.4, custom.6,
  custom.7, then this build (`tools/upgrade/chain_test.py`).

### Security (from upstream)

- The group prefix is checked in the password grant, and so in LDAP binds. The device grant checks the user again and
  checks `force_mfa` when the device is approved. The password grant and the token exchange cannot do MFA and do not
  check `force_mfa`, that is by design.
- Token exchange is limited to the scopes of the exchanging client and to the lifetime of the subject token and the user.
- The Forward Auth cookie cannot be replayed as a Rauthy session any more, and is separated per client.
- Refresh tokens are bound to their client. Logout tokens with an `iat` in the future are refused, the `exp` check uses
  the clock skew the right way round.
- RSA signing blinding, TLS verification stays on in release builds with `dev_mode`, dependency security updates,
  `session_validate_ip` for Forward Auth, user values are no longer logged at warn level, a token with a non-JSON custom
  attribute no longer panics.
- Postgres: renaming a role or group kept deleting the memberships. Fixed.

### New from upstream (all off by default)

- A read-only LDAP server for apps that can only log in with LDAP (`[ldap]`). A password login by LDAP goes through the
  same checks as the password grant, including our access rules.
- User consents (`[consent]`), RFC 8707 resource indicators for the device grant, an externally provisioned client
  secret via the Admin API, re-applying the bootstrap files on every start, an always visible "password forgotten" link
  and a password-reset redirect timeout.
- The new UI (`frontend-next`) has no screens for consents yet.

### Bugfix

- **`provider_trust_mfa` is a fork migration.** custom.7 put it into upstream's numbering as 36 / V31, which upstream uses
  now. It is `migrations_fork` number 15 now, and a database that has it in the old place is adopted before upstream's
  migrations run. Adopting after them, as it was done for the first 8 fork migrations, can not work on Postgres:
  refinery stops on an applied migration it does not know. Now both kinds are adopted first, on both databases.

### Changes

- `tools/upgrade/chain_test.py`: walks one database through a chain of images and checks the data after each step,
  Hiqlite and Postgres.
- The unit tests of upstream set up their database with our migrations too.

## v0.37.2-custom.7 (2026-11-15)

Bug fixes for the access rules, plus a trust-MFA switch per provider. **One migration** (adds `auth_providers.trust_mfa`, default false, on Hiqlite and Postgres). No config change.

### Security

- **Values an identity provider sends are checked before they are stored.** A login with Google, Github or another provider
  copied `birthdate`, `phone`, the street, the postal code, the country and the time zone from the provider into the user's
  profile **without any validation**, which the account page and the API do apply. A provider could send a text of any
  length, with line breaks (which would forge lines in the `formatted` address of the ID token), a time zone that does not
  exist or a birthdate in the future. Now every value goes through the same checks as the same value typed into the account
  page (`UserValues::from_provider`). A value that fails is left out, the login goes on, and the user can fill the field in.
  A phone number loses the spaces and dashes it was written with, a country code is made upper case. The provider's
  `locality` is stored as the city (it was ignored, so a provider's city never reached the profile).

### Bugfix

- **Access rules were checked before the required user data.** A user who signed in with Google has only an email, a
  given name and a family name. A rule like `country in UG` refused that user at once, although the client requires the
  country and the user would have been asked for it. Now a login first looks at the required user data: while a field is
  missing the rules wait, the user fills it in, and the rules decide at the login that follows. Refresh, password grant
  and forward auth keep checking the rules as before.
- **A user without groups failed every group rule, also the negations.** A negation now holds when there is nothing to
  compare: `groups not in [admin_group]` and `groups is not admin_group` let in a user who has no groups (and
  `country is not RU` a user without a country). Positive rules (`in`, `contains`, `is`, `>=`, ...) still fail on a
  missing value. Add `is set` to a rule set that must turn away users without a value.
  See [Client Access Rules](book/src/work/client_access_rules.md).
- **The order of the login checks.** Each check runs only after the one before it passed:
  1. *The group of the client* (`restrict_group_prefix`). A user outside of it is turned away at once, before an MFA is
     asked for (until now a client that forces MFA told such a user to set up a second factor first) and before any data
     is asked. The message says *"You do not have permission to view this application"* (it said "client").
  2. *The data the client requires.* A missing field sends the user to fill it in.
  3. *The access rules*, with the values filled in.
- **The popup after a login with a provider (Google, ...).** The page that comes back from the provider showed only a
  line of text when the account lacked required data, and its button led to the account page without telling it which
  application the login was for, so the account page did not list what was missing. It is the same popup as at the
  password login again (*"Some information for this account is missing. ..."*), and its button opens the account page
  with the list of the missing details and a way back to the login. The login page remembers the application before it
  goes to the provider (`rauthy_login_client` in the browser's local storage).

### Changes

- **"Trust MFA of this provider" per auth provider.** When set, every login through the provider counts as MFA, so a client that forces MFA accepts a Google or similar login without asking for a local passkey. Off by default. The API field `trust_mfa` is optional (absent = false). The old UI does not know it, saving a provider there turns it off.

- **Warning when an account has only one passkey.** The MFA page of the account shows a red box that explains that losing this one device or key locks the user out until an administrator removes it, and asks for a second passkey. The register button is highlighted while only one exists.
- **`server.internal_hosts` is listed on the static settings page** (read only).

- **The discovery document for services in the same network (`server.internal_hosts`).** The document at
  `/.well-known/openid-configuration` has only public URLs, which a container in the Docker network either cannot reach or
  reaches by going out through the proxy and back. With `internal_hosts = ['rauthy:8080']` (or `INTERNAL_HOSTS`), a request
  that arrives with that name in its `Host` header gets `token_endpoint`, `userinfo_endpoint`, `jwks_uri`,
  `introspection_endpoint` and `revocation_endpoint` at `http://rauthy:8080/auth/v1/...`. The `issuer` and the endpoints a
  browser is sent to (`authorize`, `logout`, `device`) stay public, and so does the document for every other host, the proxy
  included (it forwards the public `Host`). Only listed names count and only the raw `Host` header is read, never
  `X-Forwarded-Host`. The response has `Vary: Host`. Off by default (empty list), so nothing changes without the setting.
  See [Services in the Same Network](book/src/config/internal_hosts.md). Test: `test_well_known_for_internal_hosts`
  (`config-test.toml` lists `rauthy-internal:8081`).
- **A time zone must be an IANA id** like `Africa/Kampala` (or `UTC`) in every request that carries one (profile, admin
  update, registration), with the error on the field. Abbreviations (`EAT`, `CST`, `IST`) and offsets are refused: they
  are ambiguous and carry no daylight saving rules. The `zoneinfo` claim is the IANA id, as OIDC asks.
- **Faster login: the required user data of a client is cached.** Every login for a client read `client_required_fields`
  from the database, and now twice (once to decide about the access rules, once for the popup). It is read once per login
  and from the cache (like the access rules), cleared when the list is saved or the client is deleted. The cache entries
  of the rules and of the required data are also cleared when a client is deleted, so a client created again under the same id
  does not inherit them. No index was missing: the audit, reminder and client tables have the ones their queries use.
- **The `address` claim is written like an address.** The ID token and `/userinfo` had `formatted` as the user's name, the
  street, the postal code and the city and the country, each on a line and with a newline at the end
  (`"Jane Doe\n8th Street\nUG\n"`), and the **city was missing** unless the user also had a postal code (neither
  `formatted` nor `locality`). Now `formatted` is the street, then the postal code with the city, then the country, one line
  each and no trailing newline (`"8th Street\n00100 Nairobi\nKenya"`). The name is not part of it (the `name` claim has it).
  A part that is not filled in is left out, a blank value counts as not filled in, and spaces around a value are removed.
  The country in `formatted` is its English name (`Uganda`); the `country` member keeps the code (`UG`), because other
  systems look a country up by its code. A country that was typed in as a name stays as typed. `street_address`, `locality`,
  `postal_code` and `country` are as before. Clients that read `formatted` get no name line.
  Unit tests in `claims.rs`.
- **Errors are shown as a box.** An error text is now a bordered, tinted box with an icon (`ErrorAlert.svelte`), in the
  login, registration, password reset, logout, device and callback pages, the forms of the account page and the whole
  Admin UI, where it was a red line.
- **The *unlink provider* button** on the account page has an orange border and a small corner radius.

## v0.37.2-custom.6 (2026-10-29)

Bug fix for `custom.5`. No migration, no config change.

### Bugfix

- **A user's time zone (and other hidden profile fields) was cleared whenever they saved their profile.**
  - *Symptom:* with a profile field set to `hidden` (found with `tz = hidden` in production), the stored value of that
    field was lost as soon as the user saved their account page.
  - *Cause:* a hidden field is not on the page, but the save still sent the form. The page treats an empty time zone as
    `UTC` and sends `UTC` as "none", and the server replaced all profile values with what the request carried, so the
    stored time zone became empty. Upstream's old UI has the same logic.
  - *Fix, two parts:* the page no longer sends hidden fields (`AccEdit.svelte`), and the server keeps the stored value of
    every hidden field on a self-update, whatever the request says (`UserValues::keep_hidden`). Hiding a field never
    deletes values. An admin can still edit them. Test: `zzm_hidden_profile_fields`, unit test in `users_values.rs`.
    Users who already lost a value do not get it back.

### Changes

- **Add or remove ONE group of a user.** `PUT /auth/v1/users/{id}/groups/{group}` and
  `DELETE /auth/v1/users/{id}/groups/{group}` change a single group and answer with the user. No list is sent, the user's
  other groups stay, repeating a call changes nothing, and calls at the same time do not overwrite each other (a compare
  and swap in SQL on the `groups` column, on Hiqlite and Postgres). The group must exist (`404`). Same permissions as
  `PATCH /users/{id}`: admin, API key with `Users` `Update`, or a delegated group admin for the groups they manage
  (roles are never touched). Until now only `PUT` / `PATCH /users/{id}` could change groups, and both replace the whole
  list. In the audit log the change is an `update` of the user. No migration. Why groups are a list in the API: they are
  stored as one comma separated text column of names on the user, see
  [Adding and Removing One Group](book/src/work/user_groups.md). Group names can no longer contain a comma (create and rename): the groups of a user are one comma separated text, so a comma
  split one name into two groups. Existing names are not touched. Tests: `zzn_user_groups` (incl. 8 changes at once).
- **Hidden fields are marked in the client editor.** Under *Required user data*, a profile field that is hidden for all
  users shows **(hidden)** next to its name. Hovering it explains that users cannot fill it in, so requiring it would lock
  them out, and says how to show it (Config -> User fields). See
  [User Profile Fields](book/src/work/user_profile.md#what-hidden-means).

## v0.37.2-custom.5 (2026-10-28)

Bug fixes for `custom.4`. No migration, no config change. Safe to roll over `custom.4` directly.

### Bugfix

- **The theme script was blocked by the Content-Security-Policy in the built UI.**
  - *Symptom:* on every page of the deployed image the browser console showed
    `Loading the script 'data:text/javascript;base64,...' violates the following Content Security Policy directive`.
    The saved light / dark theme was applied late (a flash of the wrong theme). Nothing else broke.
  - *Cause:* the theme script is a small file imported with `?url`. Vite turns every file under 4 KB into a `data:` URI,
    and every page carries `script-src 'self'`, which refuses `data:` scripts. It never showed in development, because
    the dev server serves the file as a file; only the production build inlines it.
  - *Fix:* scripts are never inlined (`assetsInlineLimit` in `frontend-next/vite.config.ts`).
- **A user's language was only kept in a cookie.** The language selector on the account page set the `locale` cookie and
  reloaded, but never saved the language on the account, so a new browser, a cleared cookie or another device showed the
  old language again. The selector on the account page now also saves it (`POST /auth/v1/update_language`, which was
  there already and which upstream's UI never calls from the account page).
  (`frontend-next/src/lib/account/AccMain.svelte`)

### Not errors

These three lines in the browser console are expected and are not bugs: `GET /users/{id}/webid/data` answers 405 when
WebID is not enabled, `GET /users/{id}/self/delete` answers 406 when self-delete is not enabled
(`enable_self_delete`), and `GET /pam/users/self` answers 404 for a user who has no PAM account. The account page asks
to find out what to show.

### Open

- Reported: a user's **timezone** is lost in the deployed image. Not reproduced, no cause found in the code (the profile
  save sends it, the server stores it; the UI deliberately stores `UTC` as "none"). Needs the exact steps.

### Catching this earlier

- **The build now checks the pages it produced** (`frontend-next/tools/check-build.mjs`, run by `postbuild` and
  `postbuild:dropin`, so `pnpm build`, `pnpm build:dropin` and therefore the image build stop with an error). For every page
  it checks that the Content-Security-Policy exists and does not allow `unsafe-inline`, `unsafe-eval`, `data:` or `https:`
  scripts, that every inline script has its hash in the policy, and that every `<script src>` is a local file that exists in
  the build (no `data:`, no other origin). Run against the `custom.4` configuration it reports all 46 pages, which is how
  this bug would have been found before the image. Unit tests: `frontend-next/tests/checkBuild.test.ts`.
- See [Building and Releasing This Fork](book/src/config/building_the_image.md) for the release checklist.

## v0.37.2-custom.4 (2026-10-28)

### Changes

- **System users and an audit log of who did what.** System users have numbers as ids and names (101, 102, ...) and the
  email `<number>@system.invalid`; `101` owns what the background tasks do. They cannot log in and are read only (the server
  refuses changes and deletion, the UI shows no forms). No other user can have a username that starts with a number (names
  also need 4 characters) or an email in `system.invalid`. Every successful change through the API (users, clients, roles,
  groups, scopes, providers, settings, API keys, ...) is recorded with the id of the user or API key that made it, the time
  and the target; reads, logins and test buttons are not. Nothing references users by foreign key. When a user is deleted, a
  small copy (id, email, names, username, phone) is kept so old entries still name them; **Users -> Deleted users** lists
  them and wipes the personal data on request. Users, clients, providers and API keys get a **Status Change history** tab
  (created by / when, last modified by / when, every change). `GET /auth/v1/events/audit`, `/events/audit/summary`,
  `/events/audit/users`, `DELETE /events/audit/users/{id}`, and each entry is also written to the log (target `audit`) for
  a collector like Loki. See [System Users and the Audit Log](book/src/work/audit_log.md).
- Saving a client no longer scrolls the panel back to the top, and shows a "Client saved" toast (errors as a toast too), plus "Saved" next to the tick.
- **Access rules per client.** A client can say who may log in by the user's values and attributes: `country is one of
  UG, KE, TZ`, `age is at least 18`, `email ends with @example.com`, `roles contains staff`, `attr:<name> has a value`.
  All rules of a client must hold; a missing value never meets a rule (except "is empty"). The conditions are `is`,
  `is not`, `>`, `>=`, `<`, `<=`, `between`, `is one of`, `is none of`, `contains`, `starts with`, `ends with`, `has a value`,
  `is empty`. Checked at every login, token refresh, password grant and forward auth request, never for the `rauthy` client.
  Each rule is a numbered card (rules are tried in that order, they can be moved up and down). The editor offers the right input for each field: math signs and a **between** range for numbers (both ends included), a
  true / false choice for boolean fields, a searchable country picker, choice lists for fixed values, roles and groups.
  New section in the client editor, `GET` / `PUT /auth/v1/clients/{id}/access_rules`. A client without rules costs one cache
  lookup. Each rule can have its own message for the user; the first failing rule's message is shown, otherwise "Your
  account does not meet the requirements to access this application". See [Access Rules per Client](book/src/work/client_access_rules.md).
- **Default roles and groups for new users** (Config -> New users). Users who register themselves and users created by
  their first provider login get them. `rauthy_admin` and the delegated admin roles cannot be chosen. Users an admin
  creates are not changed. `GET` / `PUT /auth/v1/settings/new_users`.
- **Reminders for users who have not logged in** (Config -> New users). Off by default; 30 days and 3 reminders in a
  row by default, wording and numbers set by the admin, "Save and send a test to me". At most 100 users per hour, sent
  by the leader node, tracked per user so nobody gets the same reminder twice. `GET` / `PUT /auth/v1/settings/reminders`.
  See [New Users](book/src/work/new_users.md).
- Five new fork migrations: `10_access_rules_and_reminders` (two tables), `11_reminder_indexes` (two indexes),
  `12_system_user_and_audit` (the `system` user and the `audit_log` table) and `13_audit_users` (the copy of deleted users) and
  `14_system_user_numbered` (system user 101, an index).
  The tables are copied by the Hiqlite / Postgres migration tool.

### Operators

- Reminders do nothing until you switch them on. When you do, everybody who has been away longer than the time is due at
  once; the page tells you how many, and they are mailed over the next hours.
- The hourly reminder run is cheap at a large scale: on 1,000,000 users it takes about 75 ms when nobody is due (an earlier
  version of the query needed 2.1 s), thanks to an index on the last activity and one on the reminder state. Once a day,
  and after a start, it looks at every user (about 1.6 s at that size).

### Bugfix

- **The profile picture of a federated user (Google, Github, ...) was never shown.** The link was stored and sent to the page,
  but every page carried a Content-Security-Policy with `img-src 'self' data:`, so the browser refused to load an image from
  another host. The policy now also allows `https:` images (`frontend-next/svelte.config.js`). Plain `http` images stay blocked,
  scripts are still locked to hashes. The browser loads the image from the provider, Rauthy never fetches it.
  - The custom.2 / custom.3 notes said the picture was shown in the UI. It only reached the ID token and userinfo.
  - Trade-off: with `https:` an injected `<img>` could send data to any https host. Script injection is not made easier. If that is
    not acceptable, the alternative is a small avatar proxy in Rauthy (more code, Rauthy then fetches the provider's URL).
  - A user gets the picture at their next login through the provider.

---

## v0.37.2-custom.3

### Bugfix

- **Do not use `0.37.2-custom.2`.** Reading the events read marker (`GET /events/read`, and the event stream with
  `?read=true`, which the admin UI opens on every page) selected one column too few and panicked with `Cannot convert
  column index 'user_id'`. Because Rauthy aborts on a panic, the process went down as soon as an admin page was opened.
  It came from a refactor made after the feature had been tested. custom.3 fixes it and was checked against the real
  endpoints (marker read, marker moved, stream with the `read` event).

- **A failing SMTP server could flood the event log (upstream behaviour).** With an unreachable SMTP server and an events
  address set, a failed email raises a Critical `EmailSendError` event; that event is itself notified by email, which fails
  and raises the next event, without end (1558 events in 4 seconds were seen in a test). A failed *event notification*
  email no longer raises an event, so one failure gives two events and stops. It is logged instead.

No new migration. Moving from custom.1 or custom.2 needs nothing special (the data written by custom.2 is compatible).

---

## v0.37.2-custom.2

### Security

- The documentation book at `/auth/v1/book/` was readable by anyone. It now needs a logged-in session. Set
  `server.book_public = true` (`BOOK_PUBLIC`) to open it again.
- Webhook channels: a request can no longer be replayed unnoticed. With a signing secret set, it carries
  `X-Rauthy-Timestamp` and `X-Rauthy-Signature-Timestamped` (HMAC of `<timestamp>.<body>`), see
  [Message Channels](book/src/work/message_channels.md). `X-Rauthy-Signature` stays unchanged, so existing receivers
  keep working.
- The API reference needs a session unless `server.scalar_public` is set (see Breaking).

### Breaking

- **The Swagger UI has been removed**, together with its dependency (smaller binary, ~13 MB less memory when it was on).
  [Scalar](book/src/scalar.md) is the only API reference, at `/auth/v1/docs/`.
  - `server.swagger_ui_enable` / `SWAGGER_UI_ENABLE` is ignored (a warning is logged if it is `true`). Use
    `server.scalar_enable`.
  - `server.swagger_ui_public` / `SWAGGER_UI_PUBLIC` keeps working as an alias of the new `server.scalar_public`.
  - The separate page `/auth/v1/docs/scalar` is gone, `/auth/v1/docs/` is the one address.
  - An existing config file does not need to change. Nothing fails to start.

### Changes

- Saving a client no longer scrolls the panel back to the top, and shows a "Client saved" toast (errors as a toast too), plus "Saved" next to the tick.

- **Profile pictures of federated users.** The `picture` claim of the provider (`avatar_url` for Github) is stored when
  a user logs in through an auth provider and kept current on every login. It is shown in the account and admin UI, and
  it is the `picture` claim of the ID token and userinfo for client applications (scope `profile`). Only `https` links
  are taken, Rauthy never fetches them. A picture the user uploads replaces the link (Rauthy then serves its own URL).
  No migration: the link is kept in the existing `picture_id` column.
- **Static settings page** under Config: a read-only view of the values from the config file / environment that can
  only change with a restart (server, database, encryption key ids, WebAuthn, lifetimes, events, pictures, TLS,
  logging, ...). Secrets only show `set` / `not set`. `GET /auth/v1/settings/static`.
- **Webhook channels** send an `Idempotency-Key` header for every message (also in the body as `idempotency_key`); a
  retried system email carries the same key.
- **"Send a test to me"** shows what the endpoint answered (HTTP status and the start of the answer).
- **The events bell shows unread events, per admin, on every device.** The header bell has a badge with the number of
  events newer than where you last read, and the Events entry in the side navigation shows a dot. Opening the list marks
  everything as read. The marker is stored per admin in the database (new table `event_read_state`), so it is the same
  on every browser and device, and it survives a reload.
  - `GET /auth/v1/events/read` and `PUT /auth/v1/events/read` read and move the marker (it only moves forward).
  - The event stream (`GET /auth/v1/events/stream?read=true`, sessions only) sends the marker as the named SSE event
    `read`, once when it opens and again whenever another tab or device moves it. Clients that do not ask for it, and
    every existing API consumer of the stream, get exactly what they got before.
  - In a cluster, the live update only reaches the connections on the node that took the change (the other nodes catch up
    when the tab regains focus or reconnects).

### Bugfix

- **Moving from Hiqlite to Postgres (`MIGRATE_DB_FROM=sqlite:...`) failed at the very start** with `no such table: config`, in
  upstream's code: the version check read the target process instead of the source file. A second bug stopped it later,
  on the FIDO device table (`expected tuple struct KeyProtectionMask`). Both are fixed. It was rehearsed end to end on a
  copy of the dev data with Postgres 18: all 60 tables that exist on both sides, including every table and column this fork added, arrived
  with the same row counts, and users, clients, attributes and read markers have the same content. The tool copies
  `event_read_state` too. The fork's Postgres migrations (1-9) also ran for the first time on a real Postgres in this test.
- The workflow `.github/workflows/image.yml` was invalid, so GitHub reported "No jobs were run" on every push. It is a
  manual workflow again (Actions -> Build image -> Run workflow).
- The button to unlink a federated account in the account dashboard was drawn without border and hardly visible.
- The image had no default command. It now starts `serve` like upstream's image.

### Rollout

One new fork migration: `9_event_read_state` (a new table, nothing existing is changed). It applies on the first start. Image
and upgrade steps as for custom.1 (stop, back up the data volume, start the new image).

---

## v0.37.2-custom.1

The first release of the fork, based on upstream v0.37.2.

### Breaking

- **Only the new admin and account UI** (`frontend-next`, SvelteKit + shadcn) ships in the image. The old UI is not
  built in any more.
- `notifyd` is not a channel driver any more. A stored `notifyd` channel stays disabled until a new driver is chosen
  (a warning is logged); a config file or environment using `driver = "notifyd"` stops with a message at startup.
- `whatsapp` is not built in. Create it as a custom channel if you need it.
- Our database changes are **not** part of upstream's migration list. They live in `migrations_fork/` with their own
  table `_fork_migrations`, applied after upstream's. Instances that ran a pre-release with migrations 36-43 adopt them
  once. See [Upgrading Hiqlite](book/src/config/upgrading_hiqlite.md) for the in-place upgrade.

### Changes

- Saving a client no longer scrolls the panel back to the top, and shows a "Client saved" toast (errors as a toast too), plus "Saved" next to the tick.

- **Message channels.** `email` and `sms` always exist (recipient: the user's email address / phone number). Custom
  channels can be created, each with a driver, a recipient (`email`, `phone` or a custom attribute `attr:<name>`) and an
  optional expiry date; only expired channels can be deleted.
  - Drivers: `smtp` (a custom channel has its own server and credentials), `webhook` (Rauthy's signed JSON format) and
    `http` (any provider: method, URL, headers, JSON or form body, secret values, placeholders).
  - **System emails follow the `email` channel**: with `webhook` or `http`, password resets and codes go to a
    communication agent and Rauthy does not need the SMTP credentials.
  - Secrets are encrypted in the database and decrypted once when the settings load.
- **Verifiable attributes.** New attribute type `verified`; every verifiable attribute is linked to the channels that
  can verify it, and the user picks one. The verification code goes to the attribute's value.
- **Editable message templates** (Config -> Message templates): wording of the password and verification messages per
  language, validated before saving, with a test that sends to the admin's own address or phone. A save swaps the
  wording in memory, so sending stays as fast as before. Other nodes of a cluster pick it up within 15 seconds.
- **Settings in the database** (Config): SMTP, general settings, user fields, lifetimes and channels are edited in the
  admin UI. Values from the config file seed them once.
- **Required client data**: a client can require user values or attributes before a login completes.
- **User gender**, per-user client list and the SMS / phone verification flow.
- **Documentation book** is embedded in the binary at `/auth/v1/book/`.
- **Scalar API reference** instead of the Swagger UI (optional, `server.scalar_enable`).
- **Image and tooling**: `Dockerfile.fork`, `tools/image/build.sh` (local build and push to ghcr.io),
  `tools/upgrade/hiqlite_upgrade.py` (snapshot, inspect, rehearse an upgrade on a copy).

### Rollout

Snapshot the data, rehearse on a copy, stop the old container, start the new image on the same data volume with the
same config and secrets. The fork migrations apply on the first start (1-8 for custom.1, 9 is added by custom.2).
