# Sign-in lab

A small relying party to compare four ways of signing a user in with Rauthy, and to time them:

| | what happens | needs |
|---|---|---|
| 1. Redirect | whole page goes to the IdP and back | nothing special |
| 2. Silent redirect | `prompt=none`: with an IdP session the page leaves and returns with a code, nothing shown; without one it comes back with `login_required` | nothing special |
| 3. Popup | small window runs the code flow, posts the code to the page | the popup must be opened by a click |
| 4. FedCM | the browser asks the IdP who is signed in and shows its own "Continue as ..." prompt, no navigation | Chrome/Edge, HTTPS, `[fedcm] experimental_enable = true` |

After each one the page sends the ID token to its own backend (`POST /verify`), which checks signature (JWKS), `iss`, `aud`,
`exp` and `nonce` and opens a session cookie. The table shows the time from click to "backend session open".

## Rauthy side

1. Enable FedCM (only for method 4): `[fedcm] experimental_enable = true` (env `EXPERIMENTAL_FED_CM_ENABLE=true`) and restart.
   The IdP must also answer `GET https://<idp-host>/.well-known/web-identity` (root path, not under `/auth/v1`). Add that
   path to your reverse proxy's rule for the backend.
2. Create a **public** client (e.g. `signin_lab`), flow `authorization_code`, PKCE S256 on.
   - Redirect URI: `<lab origin>/` (with the trailing slash), e.g. `https://lab.timothy.online/`
   - Allowed origin: `<lab origin>` (the page calls `/oidc/token` and `/fed_cm/token` from the browser)
   - Scopes `openid email profile`
3. Be signed in at the IdP in the same browser (for 2 and 4 that is the whole point).

## Run

```bash
docker build -t signin-lab tools/signin-lab
docker run --rm -p 8099:8099 \
  -e ISSUER=https://accounts-dev.timothy.online/auth/v1 -e CLIENT_ID=signin_lab signin-lab
```

FedCM and the secure cookies need HTTPS, so put the lab behind your reverse proxy under its own name (any `*.timothy.online`
works with your mkcert wildcard). `http://localhost:8099` is fine for methods 1 to 3 if the client lists it.

## What to look at

- Is 2 fast enough to be "invisible"? It only needs the IdP session and, for a first visit of a client, no consent screen.
- Does 3 feel like "on the same page"? Compare it with 4.
- 4 is the closest to Google One Tap: no navigation and no popup. Test it in Chrome with and without third-party cookie blocking.
- Which of these can your other backends use as is? They only need the `/verify` logic (JWKS check), shown in `server.py`.

Setup steps for real apps (root-domain file, trusted apps, the client code, what the backend must check) are in the book,
chapter *Sign in With No Click (FedCM)* (`book/src/work/fedcm_silent_signin.md`).
