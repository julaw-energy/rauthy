# Sign in With No Click (FedCM)

When a user is signed in to Rauthy, an app can sign them in **without a redirect, a popup or a click**, like "Sign in
with Google" does for apps the user already knows. The browser asks Rauthy who is signed in and hands the app an ID
token. This uses [FedCM](https://fedidcg.github.io/FedCM/), which works in Chrome and Edge over HTTPS.

There are two levels, and you choose per app:

| Level | What the user sees | Which apps |
|---|---|---|
| **With a prompt** | the browser's own "Continue as ..." button, once per visit | every public client of yours |
| **Silent** | nothing, the page is signed in when it loads | only apps you list in `silent_client_ids`, and only after the user signed in to them before |

```admonish warning
FedCM is marked experimental in Rauthy and in the browsers. Keep a normal login (redirect) as the fallback in every app,
and never make FedCM the only way in.
```

## What Stays Checked

A FedCM token is an ordinary Rauthy ID token, so it is not a way around anything. For every token request, silent or
not, Rauthy runs the checks of the normal login:

- the client is enabled and the request comes from one of its **exact** allowed origins (no prefix or suffix match)
- the user is enabled, not expired, and is the user of the session
- the client's login group prefix, `force_mfa`, [required user data](required_client_data.md) and
  [access rules](client_access_rules.md). A user who does not qualify is refused, not sent to a page, because FedCM has
  no page to send them to
- a **nonce** from the app is required. A token without one is never issued
- the token carries the `sid` of the real session, the real `auth_time`, and `amr` `pwd` or `mfa` as the session was
  opened. A logout with the token as `id_token_hint` ends the right session
- confidential clients get no FedCM token unless you list them in `confidential_client_ids`

The token is an **ID token only**. There is no access or refresh token, so the app opens its own session from it.

## Set It Up

### 1. Enable FedCM in Rauthy

```toml
[fedcm]
experimental_enable = true
```

Restart Rauthy. Rauthy must be served over HTTPS.

### 2. Publish the `web-identity` file on the root domain

Chrome fetches `https://<root domain>/.well-known/web-identity`, where the root domain is the registrable domain of
the issuer. For Rauthy on `accounts.example.com` that is `example.com`, **not** `accounts.example.com`.

The simplest way is to route that one path of the root domain to Rauthy in your reverse proxy. Rauthy answers it. If
you prefer a static file, it must look like this, with your own host:

```json
{
  "provider_urls": ["https://accounts.example.com/auth/v1/fed_cm/config"],
  "accounts_endpoint": "https://accounts.example.com/auth/v1/fed_cm/accounts",
  "login_url": "https://accounts.example.com/auth/v1/account"
}
```

Check it: `curl -H 'Sec-Fetch-Dest: webidentity' https://example.com/.well-known/web-identity` must return the JSON above.

### 3. Prepare the client

In the Admin UI, the client must be:

- **public** (no secret). A confidential client is refused unless listed in `confidential_client_ids`, see below
- enabled, with the **exact origin** of the app in its **allowed origins**, e.g. `https://app.example.com`. If the list
  is empty, the origins of the redirect URIs are used
- with the scopes the app needs as **default scopes** (e.g. `openid email profile`). A FedCM request carries no scope
  parameter, so the token has the claims of the default scopes

### 4. Make it silent (optional)

List the apps that may sign users in without a prompt:

```toml
[fedcm]
silent_client_ids = ["julaw_app", "julaw_admin"]
```

Only list apps you fully trust. A silent app is signed in by **any page on its allowed origins**, with no user action,
so a script injected into that app (XSS) can get a token for the signed-in user. Keep the allowed origins of these
apps to the origins you control.

An app becomes silent for a user **after that user signed in to it once**: through a normal login, or by clicking
"Continue as ..." once. Until then the user gets the prompt. The browser is only told about apps the user really
used, and never about any other.

`silent_client_ids` is in the config file on purpose. The admin of a client cannot switch it on for their own client.

## The App

```js
const configURL = 'https://accounts.example.com/auth/v1/fed_cm/config';
const clientId = 'julaw_app';

async function fedcm(mediation) {
  const nonce = crypto.randomUUID();
  const cred = await navigator.credentials.get({
    identity: { providers: [{ configURL, clientId, params: { nonce } }] },
    mediation, // 'silent' = no UI at all, 'optional' = the browser may show "Continue as"
  });
  // Send cred.token and the nonce to YOUR backend, which verifies the token (below).
  const res = await fetch('/session/fedcm', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ token: cred.token, nonce }),
  });
  if (!res.ok) throw new Error('token rejected');
}

async function start() {
  if (!('IdentityCredential' in window)) return showNormalLogin();
  try {
    await fedcm('silent'); // known app: signed in with no click
  } catch {
    showSignInButton(); // not known yet, or the browser said no
  }
}
// the button: fedcm('optional').catch(() => redirectToNormalLogin())
```

Pass the nonce as `params: { nonce }`. The top-level `nonce` option is deprecated in Chrome and goes away.

### What your backend must check

Never trust the token because the browser handed it over. Verify it like any other ID token:

1. the **signature**, with the keys at `https://accounts.example.com/auth/v1/oidc/certs`
2. `iss` is your issuer. Rauthy's issuer ends with a `/`, so compare exactly what the discovery document says
3. `aud` is your client ID
4. `exp` is in the future
5. `nonce` is **the one this browser session generated**. Store it in the session before the call and compare it.
   This is what stops a leaked token from being replayed

Then open your own session. Keep `sid` if you want to end the Rauthy session on logout.

## What a User Can Do

- **Remove an app** in *Connected apps*. It also ends silent sign-in for that app until the user signs in to it again
- **Log out of Rauthy**. The FedCM cookie is the same session, so silent sign-in stops at once everywhere
- The browser has its own switch for this feature (in Chrome under the privacy settings, "third-party sign-in")

## Troubleshooting

| What you see | Likely cause |
|---|---|
| `NetworkError: Error retrieving a token` straight away | the `web-identity` file is missing on the **root domain**, or has no `accounts_endpoint` / `login_url` |
| the same, after the browser asked for accounts | the token request was refused. Look at Rauthy's log: origin not allowed, client confidential, group, MFA, access rule, or missing nonce |
| a prompt instead of a silent sign-in | the user has not signed in to this app before, the app is not in `silent_client_ids`, or the user removed it in *Connected apps* |
| silent works once, then fails | the browser limits silent sign-in for a while after a success. Wait a few minutes when you test |
| works on `localhost` but not in production | FedCM needs HTTPS and a registrable domain for the issuer, see step 2 |
| `Expected header Sec-Fetch-Dest: webidentity` | you called a FedCM endpoint from a script. Only the browser can |

## Try It First

The repository contains a small test app, `tools/signin-lab`, that signs in with a redirect, a silent redirect, a popup
and FedCM, and times each of them. See its `README.md`. It is the quickest way to check steps 1 to 3 before you touch a
real app.
