# Upstream Authentication Providers

This section is about upstream authentication providers, for instance things like **Sign in with GitHub**.

This feature can be considered "in beta state" and the only setup guide is for GitHub for now.  
Additional ones may come, when people are using other providers and give feedback or actually contribute to these docs.

## Direct Login with `idp_hint`

Rauthy supports the `idp_hint` query parameter in the OIDC authorization request.
When this parameter is provided with the ID of an upstream authentication provider, Rauthy will automatically redirect the user to that provider's login page, bypassing the Rauthy login selection.

For example, if you have a GitHub provider with the ID `github` configured:
`GET /auth/v1/oidc/authorize?client_id=...&idp_hint=github`

This is useful if you want to provide a "Login with GitHub" button directly in your application that skips the intermediate Rauthy login selection page.
The `idp_hint` value must match the `ID` of the provider as configured in the Admin UI.
## MFA and Federated Logins

A client can force MFA. A user who signs in through an upstream provider has no passkey or OTP of their own, so
Rauthy needs to know whether the provider already did a second factor. There are two ways to tell it:

- **MFA claim** (`mfa_claim_path` and `mfa_claim_value`): the login counts as MFA only when the provider's token
  carries this claim with this value. Use it for providers that report their authentication method, like Entra ID,
  Keycloak or Okta.
- **Trust MFA of this provider** (`trust_mfa`, a checkbox on the provider): every login through the provider counts as
  MFA, no claim needed. Use it for a provider that enforces a second factor for every account it holds.

Without either, a federated user cannot enter a client that forces MFA until they register a passkey or an OTP in
Rauthy.

> **CAUTION**: `trust_mfa` moves the trust to the provider. A provider that allows password-only accounts, like a
> public Google account, then lets anyone with that password into a client that forces MFA. Only turn it on for
> providers whose accounts you control.

`trust_mfa` is optional in the API and defaults to `false`. An update through the API without the field sets it back to
`false`, so a script that saves a provider must send the current value.
