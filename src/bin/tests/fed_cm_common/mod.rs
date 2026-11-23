//! Helpers shared by the FedCM integration tests (`zzp_fed_cm_gate`, `zzq_fed_cm_attacks`).
#![allow(dead_code)]

use crate::common::{
    CLIENT_ID, PASSWORD, USERNAME, check_status, cookie_csrf_headers_from_res, get_auth_headers,
    get_backend_url, get_solved_pow,
};
use rauthy_api_types::oidc::LoginRequest;
use reqwest::header::{COOKIE, ORIGIN};
use serde_json::{Value, json};
use std::error::Error;

pub const ORIGIN_OK: &str = "http://localhost:3000";
pub const REDIRECT: &str = "http://localhost:3000/oidc/callback";
pub const LOGGED_OUT: &str = "http://localhost:3000/logged-out";
pub const PREFIX: &str = "fedcm_gate_";

/// Creates a public client that FedCM may serve for `ORIGIN_OK`, then lets `tweak` change it
pub async fn make_client(id: &str, tweak: impl FnOnce(&mut Value)) -> Result<(), Box<dyn Error>> {
    assert!(id.starts_with("fedcm_"));
    let url = format!("{}/clients", get_backend_url());
    let res = reqwest::Client::new()
        .post(&url)
        .headers(get_auth_headers().await?)
        .json(&json!({
            "id": id,
            "name": id,
            "confidential": id.contains("_conf"),
            "redirect_uris": [REDIRECT],
        }))
        .send()
        .await?;
    let mut client: Value = check_status(res, 200).await?.json().await?;
    client["allowed_origins"] = json!([ORIGIN_OK]);
    client["challenges"] = json!(["S256", "plain"]);
    client["default_scopes"] = json!(["openid", "email"]);
    client["post_logout_redirect_uris"] = json!([LOGGED_OUT]);
    tweak(&mut client);
    let res = reqwest::Client::new()
        .put(format!("{url}/{id}"))
        .headers(get_auth_headers().await?)
        .json(&client)
        .send()
        .await?;
    check_status(res, 200).await?;
    Ok(())
}

pub async fn put_for(client: &str, what: &str, body: Value) -> Result<(), Box<dyn Error>> {
    let res = reqwest::Client::new()
        .put(format!("{}/clients/{client}/{what}", get_backend_url()))
        .headers(get_auth_headers().await?)
        .json(&body)
        .send()
        .await?;
    check_status(res, 200).await?;
    Ok(())
}

pub async fn my_user_id() -> Result<String, Box<dyn Error>> {
    let res = reqwest::Client::new()
        .get(format!("{}/users", get_backend_url()))
        .headers(get_auth_headers().await?)
        .send()
        .await?;
    let users: Value = check_status(res, 200).await?.json().await?;
    Ok(users
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == USERNAME)
        .expect("the test user exists")["id"]
        .as_str()
        .unwrap()
        .to_string())
}

/// Logs the test user in the way the browser does and returns the FedCM session cookie
/// (`name=value`), which the browser sends to the FedCM endpoints.
pub async fn fed_cm_cookie() -> Result<String, Box<dyn Error>> {
    let challenge = "oDXug9zfYqfz8ejcqMpALRPXfW8QhbKV2AVuScAt8xrLKDAmaRYQ4yRi2uqcH9ys";
    let url = format!(
        "{}/oidc/authorize?client_id={CLIENT_ID}&redirect_uri=http://localhost:3000/oidc/callback\
        &response_type=code&code_challenge={challenge}",
        get_backend_url()
    );
    let res = check_status(reqwest::get(&url).await?, 200).await?;

    let cookie = res
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|c| c.to_str().ok())
        .filter_map(|c| c.split_once(';').map(|(nv, _)| nv))
        .find(|nv| nv.starts_with("__Host-RauthySessionFedCM="))
        .map(str::to_string)
        .expect(
            "a FedCM session cookie: run the test backend with EXPERIMENTAL_FED_CM_ENABLE=true",
        );

    let headers = cookie_csrf_headers_from_res(res).await?;
    let login = LoginRequest {
        email: Some(USERNAME.to_string()),
        password: Some(PASSWORD.to_string()),
        pow: get_solved_pow().await,
        client_id: CLIENT_ID.to_string(),
        redirect_uri: REDIRECT.to_string(),
        scopes: None,
        state: None,
        nonce: Some("MySuperNonce".to_string()),
        code_challenge: Some(challenge.to_string()),
        code_challenge_method: Some("plain".to_string()),
        resource: None,
        resident_key_token: None,
        fwda: None,
    };
    let res = reqwest::Client::new()
        .post(&url)
        .headers(headers)
        .json(&login)
        .send()
        .await?;
    check_status(res, 202).await?;
    Ok(cookie)
}

pub const NONCE: &str = "fedcmgatenonce";

/// What the browser sends to the ID assertion endpoint. Returns the status and the body.
pub async fn fed_cm_token(
    cookie: &str,
    client_id: &str,
    account_id: &str,
    origin: &str,
) -> Result<(u16, String), Box<dyn Error>> {
    fed_cm_token_nonce(cookie, client_id, account_id, origin, Some(NONCE)).await
}

pub async fn fed_cm_token_nonce(
    cookie: &str,
    client_id: &str,
    account_id: &str,
    origin: &str,
    nonce: Option<&str>,
) -> Result<(u16, String), Box<dyn Error>> {
    let mut form = vec![
        ("client_id", client_id),
        ("account_id", account_id),
        ("disclosure_text_shown", "true"),
    ];
    if let Some(nonce) = nonce {
        form.push(("nonce", nonce));
    }
    let res = reqwest::Client::new()
        .post(format!("{}/fed_cm/token", get_backend_url()))
        .header(COOKIE, cookie)
        .header("sec-fetch-dest", "webidentity")
        .header(ORIGIN, origin)
        .form(&form)
        .send()
        .await?;
    let status = res.status().as_u16();
    Ok((status, res.text().await?))
}

/// The `approved_clients` the account list tells the browser
pub async fn approved_clients(cookie: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let res = reqwest::Client::new()
        .get(format!("{}/fed_cm/accounts", get_backend_url()))
        .header(COOKIE, cookie)
        .header("sec-fetch-dest", "webidentity")
        .send()
        .await?;
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await?;
    Ok(body["accounts"][0]["approved_clients"]
        .as_array()
        .expect("approved_clients")
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect())
}

/// A refusal: the right status, and no token in the answer
pub fn assert_refused(res: &(u16, String), status: u16, why: &str) {
    assert_eq!(res.0, status, "{why}: {}", res.1);
    assert!(
        !res.1.contains("\"token\""),
        "{why}: a token was handed out: {}",
        res.1
    );
}

pub struct Cleanup(pub &'static [&'static str]);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let ids = self.0;
        let _ = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("a runtime for the cleanup")
                .block_on(async {
                    if let Ok(headers) = get_auth_headers().await {
                        for id in ids {
                            let _ = reqwest::Client::new()
                                .delete(format!("{}/clients/{id}", get_backend_url()))
                                .headers(headers.clone())
                                .send()
                                .await;
                        }
                    }
                });
        })
        .join();
    }
}
