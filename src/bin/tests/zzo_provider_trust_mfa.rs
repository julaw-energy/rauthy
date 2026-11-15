use crate::common::{get_auth_headers, get_backend_url};
use pretty_assertions::assert_eq;
use std::error::Error;

mod common;

/// `trust_mfa` is optional in a request (older API clients do not send it), defaults to `false`,
/// and survives an update.
#[tokio::test]
async fn test_provider_trust_mfa() -> Result<(), Box<dyn Error>> {
    let auth_headers = get_auth_headers().await?;
    let backend_url = get_backend_url();
    let http = reqwest::Client::new();

    let upstream = "http://localhost:1";
    let mut body = serde_json::json!({
        "name": "Trust MFA Test",
        "typ": "custom",
        "enabled": false,
        "issuer": format!("{upstream}/trust_mfa"),
        "authorization_endpoint": format!("{upstream}/authorize"),
        "token_endpoint": format!("{upstream}/token"),
        "userinfo_endpoint": format!("{upstream}/userinfo"),
        "use_pkce": true,
        "client_secret_basic": false,
        "client_secret_post": false,
        "auto_onboarding": false,
        "auto_link": false,
        "client_id": "rauthy",
        "scope": "openid",
    });

    // an older client does not know the field
    let res = http
        .post(format!("{backend_url}/providers/create"))
        .headers(auth_headers.clone())
        .json(&body)
        .send()
        .await?;
    assert_eq!(res.status(), 200);
    let created = res.json::<serde_json::Value>().await?;
    assert_eq!(created["trust_mfa"], false);
    let id = created["id"].as_str().expect("the provider id").to_string();

    body["trust_mfa"] = true.into();
    let res = http
        .put(format!("{backend_url}/providers/{id}"))
        .headers(auth_headers.clone())
        .json(&body)
        .send()
        .await?;
    assert_eq!(res.status(), 200);

    let res = http
        .post(format!("{backend_url}/providers"))
        .headers(auth_headers.clone())
        .send()
        .await?;
    assert_eq!(res.status(), 200);
    let all = res.json::<Vec<serde_json::Value>>().await?;
    let found = all.iter().find(|p| p["id"] == id.as_str()).expect("the provider");
    assert_eq!(found["trust_mfa"], true);

    let res = http
        .delete(format!("{backend_url}/providers/{id}"))
        .headers(auth_headers)
        .send()
        .await?;
    assert_eq!(res.status(), 200);

    Ok(())
}
