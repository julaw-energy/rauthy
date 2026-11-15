use crate::database::{Cache, DB};
use crate::entity::scopes::Scope;
use crate::language::Language;
use crate::rauthy_config::RauthyConfig;
use rauthy_api_types::oidc::GrantType;
use rauthy_common::constants::CACHE_TTL_APP;
use rauthy_error::ErrorResponse;
use serde::Serialize;
use utoipa::ToSchema;

/// The struct for the `.well-known` endpoint for automatic OIDC discovery.
///
/// An update on this struct needs an update in `src/bin/tests/handler_generic.rs` as well.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct WellKnown {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub backchannel_logout_supported: bool,
    pub backchannel_logout_session_supported: bool,
    pub device_authorization_endpoint: String,
    pub token_endpoint: String,
    pub introspection_endpoint: String,
    pub revocation_endpoint: String,
    pub userinfo_endpoint: String,
    pub end_session_endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration_endpoint: Option<String>,
    pub jwks_uri: String,
    pub grant_types_supported: [&'static str; 6],
    pub response_types_supported: [&'static str; 1],
    pub subject_types_supported: [&'static str; 1],
    pub id_token_signing_alg_values_supported: [&'static str; 5],
    pub token_endpoint_auth_methods_supported: [&'static str; 3],
    pub token_endpoint_auth_signing_alg_values_supported: [&'static str; 5],
    pub claims_supported: [&'static str; 12],
    pub claim_types_supported: [&'static str; 3],
    pub scopes_supported: Vec<String>,
    pub code_challenge_methods_supported: [&'static str; 2],
    pub dpop_signing_alg_values_supported: [&'static str; 5],
    pub service_documentation: &'static str,
    pub ui_locales_supported: Vec<&'static str>,
    pub claims_parameter_supported: bool,
    /// RFC 9207
    pub authorization_response_iss_parameter_supported: bool,
    /// SEP-991 / draft-jonesmichael-oauth-cimd. Signals that this AS accepts
    /// clients identified by a Client ID Metadata Document URL (Rauthy already
    /// implements this via `ephemeral_from_url`). ChatGPT's custom-connector UI
    /// greys out the CIMD option until the AS discovery doc carries this flag.
    pub client_id_metadata_document_supported: bool,
}

// Versioned, because the cached document is shared across the cluster and its contents change
// with releases: after an upgrade, the new version builds its own document instead of serving
// the one cached by the previous version. This does not cover a rollback to a version whose
// entry is still cached, or `rebuild()`, which only refreshes the current version's entry, so
// other versions may serve a stale document until it expires after `CACHE_TTL_APP`.
static IDX: &str = concat!(".well-known-", env!("CARGO_PKG_VERSION"));

impl WellKnown {
    pub async fn json() -> Result<String, ErrorResponse> {
        let client = DB::hql();
        if let Some(slf) = client.get(Cache::App, IDX).await? {
            return Ok(slf);
        }

        let scopes = Scope::find_all()
            .await?
            .into_iter()
            .map(|s| s.name)
            .collect::<Vec<String>>();
        let slf = Self::new(scopes);
        let json = serde_json::to_string(&slf)?;

        client.put(Cache::App, IDX, &json, CACHE_TTL_APP).await?;

        Ok(json)
    }

    /// The document for a caller that reached Rauthy with `host` in its `Host` header.
    ///
    /// The document is built for the public address, and so are all the URLs in it. A service on
    /// the same network as Rauthy (`rauthy:8080` in a Docker network) should not send its token and
    /// JWKS requests out through the public proxy and back, and it cannot call the public `https`
    /// address at all when it has no route to it. So when `host` is one of
    /// `server.internal_hosts`, the endpoints that SERVICES call move to that host. What a browser is
    /// sent to and the `issuer`, which must be the `iss` of the tokens, always stay public. Any other
    /// host, or none, gets the public document, whatever it claims to be.
    pub async fn json_for(host: Option<&str>) -> Result<String, ErrorResponse> {
        let Some(host) = host.and_then(Self::internal_host) else {
            return Self::json().await;
        };

        let idx = Self::internal_idx(&host);
        if let Some(cached) = DB::hql().get(Cache::App, idx.clone()).await? {
            return Ok(cached);
        }

        let public = Self::json().await?;
        let base = format!("{}://{host}/auth/v1/", Self::internal_scheme(&host));
        let json = internalize(&public, &RauthyConfig::get().issuer, &base)?;
        DB::hql().put(Cache::App, idx, &json, CACHE_TTL_APP).await?;
        Ok(json)
    }

    /// `host` as it is written in `server.internal_hosts`, if it is listed there.
    fn internal_host(host: &str) -> Option<String> {
        let host = host.trim().to_ascii_lowercase();
        RauthyConfig::get()
            .vars
            .server
            .internal_hosts
            .contains(&host)
            .then_some(host)
    }

    fn internal_idx(host: &str) -> String {
        format!("{IDX}-internal-{host}")
    }

    /// Plain `http` inside the network, unless Rauthy only serves `https` or the caller used the
    /// `https` port of a listener that serves both.
    fn internal_scheme(host: &str) -> &'static str {
        use crate::ListenScheme;
        let c = RauthyConfig::get();
        match c.listen_scheme {
            ListenScheme::Https => "https",
            ListenScheme::HttpHttps
                if host.ends_with(&format!(":{}", c.vars.server.port_https)) =>
            {
                "https"
            }
            _ => "http",
        }
    }

    /// Rebuilds the WellKnown, serializes it as json and updates it inside the cache.
    /// Should be called after any update on the Scopes.
    pub async fn rebuild() -> Result<(), ErrorResponse> {
        let scopes = Scope::find_all()
            .await?
            .into_iter()
            .map(|s| s.name)
            .collect::<Vec<String>>();
        let slf = Self::new(scopes);
        let json = serde_json::to_string(&slf)?;

        DB::hql().put(Cache::App, IDX, &json, CACHE_TTL_APP).await?;
        // the documents for the internal hosts are made from this one: they are built again on demand
        for host in &RauthyConfig::get().vars.server.internal_hosts {
            DB::hql()
                .delete(Cache::App, Self::internal_idx(host))
                .await?;
        }

        Ok(())
    }
}

/// The endpoints other SERVICES call: the back channel. A browser is never sent to these.
const BACK_CHANNEL: &[&str] = &[
    "token_endpoint",
    "userinfo_endpoint",
    "jwks_uri",
    "introspection_endpoint",
    "revocation_endpoint",
];

/// Moves the back channel endpoints of a discovery document from the `issuer` URL to `base`.
/// Everything else, the `issuer` included, is left as it is.
fn internalize(public_json: &str, issuer: &str, base: &str) -> Result<String, ErrorResponse> {
    let mut doc: serde_json::Value = serde_json::from_str(public_json)?;
    if let Some(map) = doc.as_object_mut() {
        for key in BACK_CHANNEL {
            if let Some(serde_json::Value::String(url)) = map.get_mut(*key)
                && let Some(rest) = url.strip_prefix(issuer)
            {
                *url = format!("{base}{rest}");
            }
        }
    }
    Ok(serde_json::to_string(&doc)?)
}

impl WellKnown {
    pub fn new(scopes_supported: Vec<String>) -> Self {
        let issuer = &RauthyConfig::get().issuer;

        let authorization_endpoint = format!("{issuer}oidc/authorize");
        let device_authorization_endpoint = format!("{issuer}oidc/device");
        let token_endpoint = format!("{issuer}oidc/token");
        let introspection_endpoint = format!("{issuer}oidc/introspect");
        let revocation_endpoint = format!("{issuer}oidc/token/revoke");
        let userinfo_endpoint = format!("{issuer}oidc/userinfo");
        let registration_endpoint = RauthyConfig::get()
            .vars
            .dynamic_clients
            .enable
            .then_some(format!("{issuer}clients_dyn"));
        let end_session_endpoint = format!("{issuer}oidc/logout");
        let jwks_uri = format!("{issuer}oidc/certs");

        // Filter all available languages
        let langs = &RauthyConfig::get().vars.i18n.filter_lang_common;
        let ui_locales_supported = Language::all_available()
            .into_iter()
            .filter(|l| langs.iter().any(|f| l.starts_with(&f.as_ref()[..2])))
            .collect::<Vec<&str>>();

        WellKnown {
            issuer: String::from(issuer),
            authorization_endpoint,
            backchannel_logout_supported: true,
            backchannel_logout_session_supported: true,
            device_authorization_endpoint,
            token_endpoint,
            introspection_endpoint,
            revocation_endpoint,
            userinfo_endpoint,
            end_session_endpoint,
            registration_endpoint,
            jwks_uri,
            grant_types_supported: [
                GrantType::AuthorizationCode.as_str(),
                GrantType::ClientCredentials.as_str(),
                GrantType::Password.as_str(),
                GrantType::RefreshToken.as_str(),
                GrantType::DeviceCode.as_str(),
                GrantType::TokenExchange.as_str(),
            ],
            response_types_supported: ["code"],
            subject_types_supported: ["public"],
            id_token_signing_alg_values_supported: ["RS256", "RS384", "RS512", "EdDSA", "Ed25519"],
            // `none`: public clients, including ephemeral (CIMD) clients, authenticate at the
            // token endpoint with PKCE only.
            token_endpoint_auth_methods_supported: [
                "client_secret_post",
                "client_secret_basic",
                "none",
            ],
            token_endpoint_auth_signing_alg_values_supported: [
                "RS256", "RS384", "RS512", "EdDSA", "Ed25519",
            ],
            claims_supported: [
                "iss",
                "azp",
                "amr",
                "sub",
                "preferred_username",
                "email",
                "email_verified",
                "given_name",
                "family_name",
                "roles",
                "groups",
                "custom",
            ],
            claim_types_supported: ["normal", "aggregated", "distributed"],
            scopes_supported,
            code_challenge_methods_supported: ["plain", "S256"],
            dpop_signing_alg_values_supported: ["RS256", "RS384", "RS512", "EdDSA", "Ed25519"],
            service_documentation: "https://sebadob.github.io/rauthy/",
            ui_locales_supported,
            claims_parameter_supported: true,
            authorization_response_iss_parameter_supported: true,
            client_id_metadata_document_supported: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ISSUER: &str = "https://accounts.example.com/auth/v1/";

    fn public() -> String {
        serde_json::json!({
            "issuer": ISSUER,
            "authorization_endpoint": format!("{ISSUER}oidc/authorize"),
            "device_authorization_endpoint": format!("{ISSUER}oidc/device"),
            "token_endpoint": format!("{ISSUER}oidc/token"),
            "introspection_endpoint": format!("{ISSUER}oidc/introspect"),
            "revocation_endpoint": format!("{ISSUER}oidc/token/revoke"),
            "userinfo_endpoint": format!("{ISSUER}oidc/userinfo"),
            "end_session_endpoint": format!("{ISSUER}oidc/logout"),
            "registration_endpoint": format!("{ISSUER}clients_dyn"),
            "jwks_uri": format!("{ISSUER}oidc/certs"),
            "scopes_supported": ["openid", "email"],
        })
        .to_string()
    }

    #[test]
    fn services_get_the_back_channel_on_the_internal_host() {
        let json = internalize(&public(), ISSUER, "http://rauthy:8080/auth/v1/").unwrap();
        let doc: serde_json::Value = serde_json::from_str(&json).unwrap();
        let get = |k: &str| doc[k].as_str().unwrap().to_string();

        // what services call: inside the network
        assert_eq!(
            get("token_endpoint"),
            "http://rauthy:8080/auth/v1/oidc/token"
        );
        assert_eq!(
            get("userinfo_endpoint"),
            "http://rauthy:8080/auth/v1/oidc/userinfo"
        );
        assert_eq!(get("jwks_uri"), "http://rauthy:8080/auth/v1/oidc/certs");
        assert_eq!(
            get("introspection_endpoint"),
            "http://rauthy:8080/auth/v1/oidc/introspect"
        );
        assert_eq!(
            get("revocation_endpoint"),
            "http://rauthy:8080/auth/v1/oidc/token/revoke"
        );

        // what a browser is sent to, and the issuer: public
        assert_eq!(get("issuer"), ISSUER);
        assert_eq!(
            get("authorization_endpoint"),
            format!("{ISSUER}oidc/authorize")
        );
        assert_eq!(
            get("device_authorization_endpoint"),
            format!("{ISSUER}oidc/device")
        );
        assert_eq!(get("end_session_endpoint"), format!("{ISSUER}oidc/logout"));
        assert_eq!(get("registration_endpoint"), format!("{ISSUER}clients_dyn"));
        assert_eq!(
            doc["scopes_supported"],
            serde_json::json!(["openid", "email"])
        );
    }

    #[test]
    fn an_endpoint_that_is_not_under_the_issuer_is_left_alone() {
        let json = serde_json::json!({
            "issuer": ISSUER,
            "token_endpoint": "https://elsewhere.example.com/token",
        })
        .to_string();
        let out = internalize(&json, ISSUER, "http://rauthy:8080/auth/v1/").unwrap();
        let doc: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(doc["token_endpoint"], "https://elsewhere.example.com/token");
    }
}
