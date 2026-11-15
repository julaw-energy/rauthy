# Services in the Same Network (Internal Hosts)

Rauthy tells every OIDC client where its endpoints are in the discovery document at
`/auth/v1/.well-known/openid-configuration`. The document is built for the **public address** (`pub_url`), so all the
URLs in it are the public `https` ones.

That is right for browsers and for services on the internet. A service **in the same Docker network** as Rauthy has two
problems with it:

- it reaches Rauthy as `http://rauthy:8080`, a name the public address does not tell it about, and
- it should fetch tokens and keys inside the network, not go out to the internet and back through the proxy
  (or it cannot, when the container has no route to the public address).

## The Setting

```toml
[server]
internal_hosts = ['rauthy:8080']
# or: INTERNAL_HOSTS="rauthy:8080"  (several: one per line)
```

A discovery request that arrives with one of these names in its `Host` header gets a different document:

| In the document | For a listed host | For everyone else |
|---|---|---|
| `issuer` | public | public |
| `authorization_endpoint`, `device_authorization_endpoint`, `end_session_endpoint` (a browser is sent there) | public | public |
| `registration_endpoint` | public | public |
| `token_endpoint`, `userinfo_endpoint`, `jwks_uri`, `introspection_endpoint`, `revocation_endpoint` (services call them) | `http://rauthy:8080/auth/v1/...` | public |

The `issuer` always stays public: it must be the `iss` in the tokens, and a client checks them against each other. Only
the endpoints that a service calls move.

Requests through the proxy carry the public host in their `Host` header (the proxy forwards it), so they get the usual
document. That is how Rauthy tells the two apart; it looks at the raw `Host` header only, never at `X-Forwarded-Host`.

```admonish note
The same applies to the aliases `/.well-known/oauth-authorization-server` and its `/auth/v1` forms. The response has
`Vary: Host`, because one URL answers differently by host.
```

## Good to Know

- **Only listed names count.** A `Host` that is not in the list, whatever it claims, gets the public document. Write
  `host[:port]` in lower case, without a scheme or a path. The case of the request does not matter.
- **The scheme** is `http`, as the network inside is. It is `https` only when Rauthy itself serves only `https`, or the
  host uses the `https` port of a listener that serves both.
- **Clients must be told the issuer.** A library that checks that the `issuer` of the document is the URL it fetched it
  from will refuse `http://rauthy:8080/...` with the public issuer in it. Configure such a client with the issuer
  explicitly, and the discovery URL separately, as a static document would require too.
- **Several names** (`rauthy:8080`, `rauthy.julaw-net:8080`) each get their own cached document, built from the public
  one and cleared whenever it is rebuilt.
