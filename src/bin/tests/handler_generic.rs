use crate::common::{get_backend_url, get_issuer};
use pretty_assertions::assert_eq;
use serde::Deserialize;
use std::error::Error;

mod common;

#[tokio::test]
async fn test_get_ping() -> Result<(), Box<dyn Error>> {
    let url = format!("{}/ping", get_backend_url());
    let res = reqwest::get(&url).await?;
    assert_eq!(res.status(), 200);
    Ok(())
}

// Re-defined and copy & pasted here to be able to get rid of lots of
// memory allocations in prod, because `Deserialize` will not work with
// `'static` lifetimes.
#[derive(Debug, Deserialize)]
pub struct WellKnown {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub backchannel_logout_supported: bool,
    pub backchannel_logout_session_supported: bool,
    pub device_authorization_endpoint: String,
    pub token_endpoint: String,
    pub introspection_endpoint: String,
    pub userinfo_endpoint: String,
    pub end_session_endpoint: String,
    pub registration_endpoint: Option<String>,
    pub jwks_uri: String,
    pub grant_types_supported: Vec<String>,
    pub response_types_supported: Vec<String>,
    pub subject_types_supported: Vec<String>,
    pub id_token_signing_alg_values_supported: Vec<String>,
    pub token_endpoint_auth_methods_supported: Vec<String>,
    pub token_endpoint_auth_signing_alg_values_supported: Vec<String>,
    pub claims_supported: Vec<String>,
    pub claim_types_supported: Vec<String>,
    pub scopes_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<String>,
    pub dpop_signing_alg_values_supported: Vec<String>,
    pub service_documentation: String,
    pub ui_locales_supported: Vec<String>,
    pub claims_parameter_supported: bool,
    pub authorization_response_iss_parameter_supported: bool,
    pub client_id_metadata_document_supported: bool,
}

#[tokio::test]
async fn test_get_well_known() -> Result<(), Box<dyn Error>> {
    let url = format!("{}/.well-known/openid-configuration", get_backend_url());
    let res = reqwest::get(&url).await?;

    assert_eq!(res.status(), 200);
    let content = res.json::<WellKnown>().await?;
    // strip trailing /
    assert_eq!(content.issuer[..content.issuer.len() - 1], get_issuer());
    // RFC 9207
    assert!(content.authorization_response_iss_parameter_supported);
    // don't test the rest for now as it might change soon again

    Ok(())
}

// Regression for issue #1643: because the issuer carries a path
// (`.../auth/v1/`), RFC 8414 §3.1 mandates the AS-metadata URL be formed by
// INSERTING `/.well-known/oauth-authorization-server` between host and path.
// claude.ai probes exactly this path-insertion form; it must return the same
// JSON well-known document (not the SPA fallback) so the client can read
// `client_id_metadata_document_supported`.
#[tokio::test]
async fn test_get_well_known_oauth_rfc8414() -> Result<(), Box<dyn Error>> {
    let backend = get_backend_url();
    let root = backend.strip_suffix("/auth/v1").unwrap_or(&backend);

    // Both the no-slash form (probed by claude.ai) and the trailing-slash form
    // (issuer path is `/auth/v1/`) must return the well-known document, not the SPA.
    for url in [
        format!("{root}/.well-known/oauth-authorization-server/auth/v1"),
        format!("{root}/.well-known/oauth-authorization-server/auth/v1/"),
    ] {
        let res = reqwest::get(&url).await?;

        assert_eq!(res.status(), 200, "unexpected status for {url}");
        assert_eq!(
            res.headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("application/json"),
            "unexpected content-type for {url}",
        );
        let content = res.json::<WellKnown>().await?;
        // strip trailing /
        assert_eq!(content.issuer[..content.issuer.len() - 1], get_issuer());
        assert!(content.authorization_response_iss_parameter_supported);
    }

    Ok(())
}

/// A service that reaches Rauthy under a name from `server.internal_hosts` (`config-test.toml`) is
/// told to call the back channel there. Everyone else, and a browser's endpoints, stay public.
#[tokio::test]
async fn test_well_known_for_internal_hosts() -> Result<(), Box<dyn Error>> {
    let url = format!("{}/.well-known/openid-configuration", get_backend_url());
    let get = async |host: Option<&str>| -> Result<serde_json::Value, Box<dyn Error>> {
        let mut req = reqwest::Client::new().get(&url);
        if let Some(host) = host {
            req = req.header("host", host);
        }
        let res = req.send().await?;
        assert_eq!(res.headers().get("vary").unwrap(), "Host");
        Ok(res.json().await?)
    };

    // the usual caller: the public document
    let public = get(None).await?;
    let issuer = public["issuer"].as_str().unwrap().to_string();
    assert!(issuer.ends_with("/auth/v1/"), "{issuer}");
    assert_eq!(public["token_endpoint"], format!("{issuer}oidc/token"));
    assert_eq!(public["jwks_uri"], format!("{issuer}oidc/certs"));

    // a service in the network
    let internal = get(Some("rauthy-internal:8081")).await?;
    let base = "http://rauthy-internal:8081/auth/v1/";
    for (key, path) in [
        ("token_endpoint", "oidc/token"),
        ("userinfo_endpoint", "oidc/userinfo"),
        ("jwks_uri", "oidc/certs"),
        ("introspection_endpoint", "oidc/introspect"),
        ("revocation_endpoint", "oidc/token/revoke"),
    ] {
        assert_eq!(internal[key], format!("{base}{path}"), "{key}");
    }
    // what a browser is sent to, and the issuer, stay public
    assert_eq!(internal["issuer"], public["issuer"]);
    for key in [
        "authorization_endpoint",
        "device_authorization_endpoint",
        "end_session_endpoint",
    ] {
        assert_eq!(internal[key], public[key], "{key}");
    }
    // the case of the host does not matter
    let upper = get(Some("RAUTHY-Internal:8081")).await?;
    assert_eq!(upper["token_endpoint"], internal["token_endpoint"]);

    // a host that is not listed gets the public document, whatever it claims to be
    for host in ["evil.example.com", "rauthy-internal:9999", "rauthy-internal"] {
        let other = get(Some(host)).await?;
        assert_eq!(other["token_endpoint"], public["token_endpoint"], "{host}");
    }

    // the alias for OAuth clients answers the same way
    let alias = reqwest::Client::new()
        .get(format!("{}/.well-known/oauth-authorization-server", get_backend_url()))
        .header("host", "rauthy-internal:8081")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    assert_eq!(alias["token_endpoint"], internal["token_endpoint"]);
    Ok(())
}
