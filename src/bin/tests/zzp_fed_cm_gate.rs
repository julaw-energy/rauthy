//! Fork test: FedCM must apply the same client checks as the normal login.
//!
//! `POST /fed_cm/token` hands out an ID token from the browser's own request, with no login page in
//! between. Upstream's version only checked that the client is enabled and that the origin is
//! allowed. Anything a client restricts (group, MFA, required data, access rules) has to be refused
//! here as well, or FedCM becomes the way around it.
//!
//! The test backend must run with `EXPERIMENTAL_FED_CM_ENABLE=true` (the `justfile` sets it).

use crate::common::{USERNAME, check_status, get_auth_headers, get_backend_url};
use rauthy_api_types::oidc::LogoutRequest;
use reqwest::header::{COOKIE, ORIGIN};
use serde_json::{Value, json};
use std::error::Error;

mod common;
mod fed_cm_common;

use fed_cm_common::*;

/// every client this test creates, so the cleanup can remove them
const CLIENTS: &[&str] = &[
    "fedcm_gate_silent",
    "fedcm_gate_ok",
    "fedcm_gate_group",
    "fedcm_gate_mfa",
    "fedcm_gate_rule",
    "fedcm_gate_fields",
    "fedcm_gate_off",
    "fedcm_gate_conf_no",
    "fedcm_gate_conf_ok",
    "fedcm_gate_redirect",
];


/// One test, on purpose: one login, then one client per restriction.
#[tokio::test]
async fn test_fed_cm_applies_client_checks() -> Result<(), Box<dyn Error>> {
    let _cleanup = Cleanup(CLIENTS);
    // a run that was killed could have left clients behind
    for id in CLIENTS {
        let _ = reqwest::Client::new()
            .delete(format!("{}/clients/{id}", get_backend_url()))
            .headers(get_auth_headers().await?)
            .send()
            .await;
    }

    let user_id = my_user_id().await?;
    make_client("fedcm_gate_ok", |_| {}).await?;
    make_client("fedcm_gate_group", |c| {
        c["restrict_group_prefix"] = json!("some_other_group_");
    })
    .await?;
    make_client("fedcm_gate_mfa", |c| c["force_mfa"] = json!(true)).await?;
    make_client("fedcm_gate_rule", |_| {}).await?;
    make_client("fedcm_gate_fields", |_| {}).await?;
    make_client("fedcm_gate_off", |c| c["enabled"] = json!(false)).await?;
    // `fedcm_gate_conf_ok` is the one the test backend lists in `FED_CM_CONFIDENTIAL_CLIENT_IDS`
    make_client("fedcm_gate_conf_no", |_| {}).await?;
    make_client("fedcm_gate_conf_ok", |_| {}).await?;
    // no allowed origin: the origin is then taken from the redirect URI
    make_client("fedcm_gate_redirect", |c| c["allowed_origins"] = json!([])).await?;

    let cookie = fed_cm_cookie().await?;

    // 1. the baseline: without a restriction FedCM issues a token. Without this, the refusals
    // below could pass for the wrong reason (a broken request).
    let res = fed_cm_token(&cookie, "fedcm_gate_ok", &user_id, ORIGIN_OK).await?;
    assert_eq!(res.0, 200, "baseline: {}", res.1);
    let body: Value = serde_json::from_str(&res.1)?;
    let token = body["token"].as_str().expect("an ID token");
    let claims = common::decode_claims(token);
    assert_eq!(claims["aud"], "fedcm_gate_ok");
    assert_eq!(claims["sub"], user_id.as_str());
    // the client asked for `email` by default, so the token carries it
    assert_eq!(claims["email"], USERNAME);
    // what the session really did: the nonce of the app, this session's id, a password login (the
    // test user has no second factor) and the time this session was opened
    assert_eq!(claims["nonce"], NONCE);
    assert!(
        claims["sid"].as_str().is_some_and(|s| !s.is_empty()),
        "the token carries the session id: {claims}"
    );
    assert_eq!(claims["amr"], json!(["pwd"]));
    let now = chrono::Utc::now().timestamp();
    let auth_time = claims["auth_time"].as_i64().expect("auth_time");
    assert!(
        (now - 300..=now).contains(&auth_time),
        "auth_time is the start of this session, not an old login: {auth_time} vs {now}"
    );

    // the nonce is mandatory: without one, a leaked token could be replayed to the app
    let res = fed_cm_token_nonce(&cookie, "fedcm_gate_ok", &user_id, ORIGIN_OK, None).await?;
    assert_refused(&res, 400, "no nonce");
    let res = fed_cm_token_nonce(&cookie, "fedcm_gate_ok", &user_id, ORIGIN_OK, Some("")).await?;
    assert_refused(&res, 400, "an empty nonce");

    // 2. the origin is checked against the client, and an unknown account is refused
    let res = fed_cm_token(&cookie, "fedcm_gate_ok", &user_id, "http://evil.example").await?;
    assert_refused(&res, 403, "an origin the client does not list");
    let res = fed_cm_token(&cookie, "fedcm_gate_ok", "someoneelse", ORIGIN_OK).await?;
    assert_refused(&res, 401, "an account that is not the session's");

    // 3. a group the user is not in
    let res = fed_cm_token(&cookie, "fedcm_gate_group", &user_id, ORIGIN_OK).await?;
    assert_refused(&res, 403, "restrict_group_prefix");

    // 4. force_mfa: this session was opened with the password only
    let res = fed_cm_token(&cookie, "fedcm_gate_mfa", &user_id, ORIGIN_OK).await?;
    assert_refused(&res, 406, "force_mfa");

    // 5. an access rule the user does not meet, and the same client without the rule
    put_for(
        "fedcm_gate_rule",
        "access_rules",
        json!({"rules": [{"field": "email", "op": "ends_with", "value": "@nowhere.example",
                          "message": "not for you"}]}),
    )
    .await?;
    let res = fed_cm_token(&cookie, "fedcm_gate_rule", &user_id, ORIGIN_OK).await?;
    assert_refused(&res, 403, "an access rule");
    put_for("fedcm_gate_rule", "access_rules", json!({"rules": []})).await?;
    let res = fed_cm_token(&cookie, "fedcm_gate_rule", &user_id, ORIGIN_OK).await?;
    assert_eq!(res.0, 200, "without the rule: {}", res.1);

    // 6. data the client requires and the profile lacks: refused, not skipped
    put_for(
        "fedcm_gate_fields",
        "required_fields",
        json!({"fields": ["country"]}),
    )
    .await?;
    let res = fed_cm_token(&cookie, "fedcm_gate_fields", &user_id, ORIGIN_OK).await?;
    assert_refused(&res, 403, "a required field the user has not filled in");

    // 7. a disabled client (upstream already checked this, it must stay that way)
    let res = fed_cm_token(&cookie, "fedcm_gate_off", &user_id, ORIGIN_OK).await?;
    assert_eq!(res.0, 401, "a disabled client: {}", res.1);
    assert!(!res.1.contains("\"token\""));

    // 7b. a confidential client gets no FedCM token unless the server operator lists it
    let res = fed_cm_token(&cookie, "fedcm_gate_conf_no", &user_id, ORIGIN_OK).await?;
    assert_refused(&res, 403, "a confidential client the operator did not list");
    let res = fed_cm_token(&cookie, "fedcm_gate_conf_ok", &user_id, ORIGIN_OK).await?;
    assert_eq!(
        res.0, 200,
        "a confidential client the operator listed: {}",
        res.1
    );

    // 7c. the origin is taken from the redirect URI when no origin is listed, exactly and not by
    // prefix: `http://localhost:30` and `http://localhost` are prefixes of `http://localhost:3000`
    let res = fed_cm_token(&cookie, "fedcm_gate_redirect", &user_id, ORIGIN_OK).await?;
    assert_eq!(res.0, 200, "the origin of the redirect URI: {}", res.1);
    for origin in [
        "http://localhost:30",
        "http://localhost",
        "http://localhost:30000",
    ] {
        let res = fed_cm_token(&cookie, "fedcm_gate_redirect", &user_id, origin).await?;
        assert_refused(&res, 403, &format!("the prefix {origin} of a redirect URI"));
    }

    // 7b. silent sign-in: the browser may skip the prompt only for a client the admin listed in
    // `silent_client_ids` AND that this user has signed in to before. Others keep the prompt.
    make_client("fedcm_gate_silent", |_| {}).await?;
    let approved = approved_clients(&cookie).await?;
    assert!(
        !approved.iter().any(|c| c.starts_with(PREFIX)),
        "nothing is approved before the first sign-in: {approved:?}"
    );
    let res = fed_cm_token(&cookie, "fedcm_gate_silent", &user_id, ORIGIN_OK).await?;
    assert_eq!(res.0, 200, "{}", res.1);
    let res = fed_cm_token(&cookie, "fedcm_gate_ok", &user_id, ORIGIN_OK).await?;
    assert_eq!(res.0, 200, "{}", res.1);
    let approved = approved_clients(&cookie).await?;
    assert!(
        approved.contains(&"fedcm_gate_silent".to_string()),
        "a trusted client the user signed in to is approved: {approved:?}"
    );
    assert!(
        !approved.contains(&"fedcm_gate_ok".to_string()),
        "a client that is not trusted is never approved: {approved:?}"
    );

    // 7b2. revoking the app in "Connected apps" ends silent sign-in for it, until the next sign-in
    let res = reqwest::Client::new()
        .delete(format!(
            "{}/users/{user_id}/consents?client_id=fedcm_gate_silent",
            get_backend_url()
        ))
        .headers(get_auth_headers().await?)
        .send()
        .await?;
    check_status(res, 200).await?;
    let approved = approved_clients(&cookie).await?;
    assert!(
        !approved.contains(&"fedcm_gate_silent".to_string()),
        "a revoked app is not approved any more: {approved:?}"
    );

    // 7c. newer browsers send the nonce inside `params`; it counts, and empty params do not
    for (params, want) in [
        (r#"{"nonce":"fromparams1"}"#, 200),
        (r#"{"nonce":""}"#, 400),
    ] {
        let res = reqwest::Client::new()
            .post(format!("{}/fed_cm/token", get_backend_url()))
            .header(COOKIE, &cookie)
            .header("sec-fetch-dest", "webidentity")
            .header(ORIGIN, ORIGIN_OK)
            .form(&[
                ("client_id", "fedcm_gate_ok"),
                ("account_id", user_id.as_str()),
                ("params", params),
                ("disclosure_text_shown", "true"),
            ])
            .send()
            .await?;
        assert_eq!(res.status().as_u16(), want, "params {params}");
    }

    // 8. the token's `sid` is the session: an RP-initiated logout with it as `id_token_hint` ends
    // that session, and the FedCM cookie of the same session stops working with it
    let res = fed_cm_token(&cookie, "fedcm_gate_ok", &user_id, ORIGIN_OK).await?;
    assert_eq!(res.0, 200, "before the logout: {}", res.1);
    let hint = serde_json::from_str::<Value>(&res.1)?["token"]
        .as_str()
        .unwrap()
        .to_string();
    let res = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?
        .post(format!("{}/oidc/logout", get_backend_url()))
        .header("sec-fetch-site", "cross-site")
        .form(&LogoutRequest {
            id_token_hint: Some(hint),
            post_logout_redirect_uri: Some(LOGGED_OUT.to_string()),
            state: None,
            logout_token: None,
        })
        .send()
        .await?;
    assert!(
        res.status().is_success() || res.status().is_redirection(),
        "logout: {} {}",
        res.status(),
        res.text().await?
    );
    let res = fed_cm_token(&cookie, "fedcm_gate_ok", &user_id, ORIGIN_OK).await?;
    assert_refused(&res, 401, "the FedCM cookie after the logout");

    // 9. no session cookie at all
    let res = reqwest::Client::new()
        .post(format!("{}/fed_cm/token", get_backend_url()))
        .header("sec-fetch-dest", "webidentity")
        .header(ORIGIN, ORIGIN_OK)
        .form(&[
            ("client_id", "fedcm_gate_ok"),
            ("account_id", user_id.as_str()),
            ("nonce", NONCE),
            ("disclosure_text_shown", "true"),
        ])
        .send()
        .await?;
    assert_eq!(res.status(), 401);

    Ok(())
}
