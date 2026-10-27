use crate::common::{
    CLIENT_ID, PASSWORD, USERNAME, check_status, cookie_csrf_headers_from_res, get_auth_headers,
    get_backend_url, get_solved_pow,
};
use rauthy_api_types::oidc::{GrantType, LoginRequest, TokenRequest};
use rauthy_service::token_set::TokenSet;
use serde_json::{Value, json};
use std::error::Error;

mod common;

async fn put_rules(rules: Value) -> Result<reqwest::Response, Box<dyn Error>> {
    let url = format!("{}/clients/{CLIENT_ID}/access_rules", get_backend_url());
    Ok(reqwest::Client::new()
        .put(&url)
        .headers(get_auth_headers().await?)
        .json(&json!({ "rules": rules }))
        .send()
        .await?)
}

async fn get_rules() -> Result<Value, Box<dyn Error>> {
    let url = format!("{}/clients/{CLIENT_ID}/access_rules", get_backend_url());
    let res = reqwest::Client::new()
        .get(&url)
        .headers(get_auth_headers().await?)
        .send()
        .await?;
    Ok(check_status(res, 200).await?.json().await?)
}

/// The client secret as it is now: an earlier test of the suite rotates the one in `common`
async fn current_secret() -> Result<String, Box<dyn Error>> {
    let url = format!("{}/clients/{CLIENT_ID}/secret", get_backend_url());
    // `POST` reads the secret, `PUT` would rotate it
    let res = reqwest::Client::new()
        .post(&url)
        .headers(get_auth_headers().await?)
        .send()
        .await?;
    let body: Value = check_status(res, 200).await?.json().await?;
    Ok(body["secret"]
        .as_str()
        .expect("the client has a secret")
        .to_string())
}

async fn token_request(req: &TokenRequest) -> Result<reqwest::Response, Box<dyn Error>> {
    let url = format!("{}/oidc/token", get_backend_url());
    Ok(reqwest::Client::new().post(&url).form(req).send().await?)
}

fn password_grant(secret: &str) -> TokenRequest {
    TokenRequest {
        grant_type: GrantType::Password,
        client_id: Some(CLIENT_ID.to_string()),
        client_secret: Some(secret.to_string()),
        username: Some(USERNAME.to_string()),
        password: Some(PASSWORD.to_string()),
        ..Default::default()
    }
}

/// A login the way the browser does it: `POST /authorize` with the credentials. `202` means the
/// login went through and a code was issued.
async fn authorize_status() -> Result<u16, Box<dyn Error>> {
    let challenge_plain = "oDXug9zfYqfz8ejcqMpALRPXfW8QhbKV2AVuScAt8xrLKDAmaRYQ4yRi2uqcH9ys";
    let redirect_uri = "http://localhost:3000/oidc/callback";
    let url = format!(
        "{}/oidc/authorize?client_id={CLIENT_ID}&redirect_uri={redirect_uri}\
        &response_type=code&code_challenge={challenge_plain}",
        get_backend_url()
    );
    let res = check_status(reqwest::get(&url).await?, 200).await?;
    let headers = cookie_csrf_headers_from_res(res).await?;

    let login = LoginRequest {
        email: Some(USERNAME.to_string()),
        password: Some(PASSWORD.to_string()),
        pow: get_solved_pow().await,
        client_id: CLIENT_ID.to_string(),
        redirect_uri: redirect_uri.to_string(),
        scopes: None,
        state: None,
        nonce: Some("MySuperNonce".to_string()),
        code_challenge: Some(challenge_plain.to_string()),
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
    Ok(res.status().as_u16())
}

/// Takes the rules off the client when the test ends, also when an assertion panics: they would
/// break the tests that follow.
struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::thread::spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("a runtime for the cleanup")
                .block_on(async {
                    let _ = put_rules(json!([])).await;
                });
        })
        .join();
    }
}

/// One test, on purpose: the rules belong to one shared client, tests running in parallel would
/// see each other's rules.
#[tokio::test]
async fn test_client_access_rules() -> Result<(), Box<dyn Error>> {
    let _cleanup = Cleanup;
    let secret = current_secret().await?;
    // start clean, a run that was killed could have left rules behind
    check_status(put_rules(json!([])).await?, 200).await?;
    // no rules to begin with, and the fields and operators a rule can use are listed
    let body = get_rules().await?;
    assert_eq!(body["rules"], json!([]));
    let fields = body["fields"].as_array().unwrap();
    for f in ["country", "age", "email", "roles", "groups", "birthdate"] {
        assert!(fields.contains(&json!(f)), "field {f} is listed");
    }
    let ops = body["ops"].as_array().unwrap();
    for o in ["eq", "gte", "in", "not_in", "contains", "is_set", "not_set"] {
        assert!(ops.contains(&json!(o)), "operator {o} is listed");
    }

    // rules that must not be saved
    for bad in [
        json!([{"field": "nope", "op": "eq", "value": "x"}]),
        json!([{"field": "attr:nope", "op": "eq", "value": "x"}]),
        json!([{"field": "country", "op": "teleport", "value": "x"}]),
        json!([{"field": "country", "op": "in", "value": "UG"}]),
        json!([{"field": "country", "op": "in", "value": []}]),
        json!([{"field": "age", "op": "gte", "value": "old"}]),
        json!([{"field": "age", "op": "gte"}]),
        json!([{"field": "country", "op": "is_set", "value": "x"}]),
        json!(
            (0..21)
                .map(|_| json!({"field": "country", "op": "is_set"}))
                .collect::<Vec<_>>()
        ),
    ] {
        let res = put_rules(bad.clone()).await?;
        assert_eq!(res.status(), 400, "{bad} must be refused");
    }
    assert_eq!(get_rules().await?["rules"], json!([]), "nothing was saved");

    // a rule the user meets: the login works
    let res =
        put_rules(json!([{"field": "email", "op": "ends_with", "value": "@localhost"}])).await?;
    check_status(res, 200).await?;
    let res = token_request(&password_grant(&secret)).await?;
    let ts: TokenSet = check_status(res, 200).await?.json().await?;
    let refresh_token = ts.refresh_token.expect("a refresh token");
    assert_eq!(
        authorize_status().await?,
        202,
        "the browser login works too"
    );

    // rules the user does not meet: the login is refused
    for unmet in [
        json!([{"field": "email", "op": "ends_with", "value": "@nowhere.invalid"}]),
        // the test user has no birthdate, so no age: a missing value never meets a rule
        json!([{"field": "age", "op": "gte", "value": 18}]),
        json!([{"field": "country", "op": "in", "value": ["UG", "KE", "TZ"]}]),
        json!([{"field": "roles", "op": "contains", "value": "no_such_role"}]),
        // all rules must hold: one that is met does not make up for one that is not
        json!([
            {"field": "email", "op": "ends_with", "value": "@localhost"},
            {"field": "country", "op": "in", "value": ["UG"]}
        ]),
    ] {
        let res = put_rules(unmet.clone()).await?;
        check_status(res, 200).await?;
        let res = token_request(&password_grant(&secret)).await?;
        assert_eq!(res.status(), 403, "{unmet} must refuse the login");
        assert_eq!(
            authorize_status().await?,
            403,
            "{unmet} must refuse the browser login"
        );
    }

    // ... and so is a refresh, while the rule stays unmet
    let refresh = TokenRequest {
        grant_type: GrantType::RefreshToken,
        client_id: Some(CLIENT_ID.to_string()),
        client_secret: Some(secret.clone()),
        refresh_token: Some(refresh_token),
        ..Default::default()
    };
    let res = token_request(&refresh).await?;
    assert_eq!(res.status(), 403, "a refresh is checked too");

    // rules the user meets in several ways
    for met in [
        json!([{"field": "age", "op": "not_set"}]),
        json!([{"field": "country", "op": "not_set"}]),
        json!([{"field": "email", "op": "in", "value": [USERNAME, "someone@else.org"]}]),
        json!([{"field": "email", "op": "contains", "value": "INIT_ADMIN"}]),
        json!([{"field": "email_verified", "op": "is_set"}]),
        json!([
            {"field": "email", "op": "ends_with", "value": "@localhost"},
            {"field": "age", "op": "not_set"}
        ]),
    ] {
        let res = put_rules(met.clone()).await?;
        check_status(res, 200).await?;
        let res = token_request(&password_grant(&secret)).await?;
        assert_eq!(res.status(), 200, "{met} must let the login through");
        assert_eq!(
            authorize_status().await?,
            202,
            "{met} must let the browser login through"
        );
    }

    // the refresh token was not used up by the refusal: with the rules gone it works
    let res = put_rules(json!([])).await?;
    check_status(res, 200).await?;
    let res = token_request(&refresh).await?;
    assert_eq!(res.status(), 200, "without rules the refresh works again");

    Ok(())
}
