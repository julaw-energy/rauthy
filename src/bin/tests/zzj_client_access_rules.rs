use crate::common::{
    CLIENT_ID, PASSWORD, USERNAME, check_status, cookie_csrf_headers_from_res, get_auth_headers,
    get_backend_url, get_solved_pow, session_headers,
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

async fn put_required(fields: Value) -> Result<reqwest::Response, Box<dyn Error>> {
    let url = format!("{}/clients/{CLIENT_ID}/required_fields", get_backend_url());
    Ok(reqwest::Client::new()
        .put(&url)
        .headers(get_auth_headers().await?)
        .json(&json!({ "fields": fields }))
        .send()
        .await?)
}

/// The test user changes their own profile values, the way the account page does
async fn set_country(country: Option<&str>) -> Result<(), Box<dyn Error>> {
    let (session, _) = session_headers().await;
    let list = reqwest::Client::new()
        .get(format!("{}/users", get_backend_url()))
        .headers(get_auth_headers().await?)
        .send()
        .await?;
    let users: Value = check_status(list, 200).await?.json().await?;
    let me = users
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == USERNAME)
        .expect("the test user exists");
    let id = me["id"].as_str().unwrap();
    let values = country.map_or(json!({}), |c| json!({ "country": c }));
    let res = reqwest::Client::new()
        .put(format!("{}/users/{id}/self", get_backend_url()))
        .headers(session)
        .json(&json!({"email": USERNAME, "user_values": values}))
        .send()
        .await?;
    check_status(res, 200).await?;
    Ok(())
}

const GROUP_CLIENT: &str = "access_rules_group_test";

/// A client of its own that only a group may use: the shared one has settings no update accepts,
/// and the rules and required data of this one stay apart from the other checks
async fn create_group_client(prefix: &str) -> Result<(), Box<dyn Error>> {
    let url = format!("{}/clients", get_backend_url());
    let res = reqwest::Client::new()
        .post(&url)
        .headers(get_auth_headers().await?)
        .json(&json!({
            "id": GROUP_CLIENT,
            "name": "Access rules group test",
            "confidential": false,
            "redirect_uris": ["http://localhost:3000/oidc/callback"],
        }))
        .send()
        .await?;
    let mut client: Value = check_status(res, 200).await?.json().await?;
    client["restrict_group_prefix"] = json!(prefix);
    client["challenges"] = json!(["S256", "plain"]);
    // the test user has no second factor: if MFA were asked first, they would be told to set one up
    client["force_mfa"] = json!(true);
    let res = reqwest::Client::new()
        .put(format!("{url}/{GROUP_CLIENT}"))
        .headers(get_auth_headers().await?)
        .json(&client)
        .send()
        .await?;
    check_status(res, 200).await?;
    Ok(())
}

async fn delete_group_client() {
    if let Ok(headers) = get_auth_headers().await {
        let _ = reqwest::Client::new()
            .delete(format!("{}/clients/{GROUP_CLIENT}", get_backend_url()))
            .headers(headers)
            .send()
            .await;
    }
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
    Ok(authorize_login().await?.0)
}

/// The same login, with what the server said besides the status
async fn authorize_login() -> Result<(u16, String), Box<dyn Error>> {
    authorize_login_for(CLIENT_ID).await
}

async fn authorize_login_for(client_id: &str) -> Result<(u16, String), Box<dyn Error>> {
    let challenge_plain = "oDXug9zfYqfz8ejcqMpALRPXfW8QhbKV2AVuScAt8xrLKDAmaRYQ4yRi2uqcH9ys";
    let redirect_uri = "http://localhost:3000/oidc/callback";
    let url = format!(
        "{}/oidc/authorize?client_id={client_id}&redirect_uri={redirect_uri}\
        &response_type=code&code_challenge={challenge_plain}",
        get_backend_url()
    );
    let res = check_status(reqwest::get(&url).await?, 200).await?;
    let headers = cookie_csrf_headers_from_res(res).await?;

    let login = LoginRequest {
        email: Some(USERNAME.to_string()),
        password: Some(PASSWORD.to_string()),
        pow: get_solved_pow().await,
        client_id: client_id.to_string(),
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
    let status = res.status().as_u16();
    Ok((status, res.text().await?))
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
                    let _ = put_required(json!([])).await;
                    let _ = set_country(None).await;
                    delete_group_client().await;
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
    let kind_of = |name: &str| {
        fields
            .iter()
            .find(|f| f["name"] == name)
            .unwrap_or_else(|| panic!("field {name} is listed"))["kind"]
            .as_str()
            .unwrap()
            .to_string()
    };
    // the kind tells the UI which input to offer
    for (name, kind) in [
        ("country", "country"),
        ("age", "number"),
        ("email_verified", "bool"),
        ("birthdate", "date"),
        ("gender", "choice"),
        ("email", "text"),
        ("roles", "list"),
        ("groups", "list"),
    ] {
        assert_eq!(kind_of(name), kind, "the kind of {name}");
    }
    let gender = fields.iter().find(|f| f["name"] == "gender").unwrap();
    assert_eq!(gender["options"], json!(["female", "male", "other"]));
    let roles = fields.iter().find(|f| f["name"] == "roles").unwrap();
    assert!(
        roles["options"]
            .as_array()
            .unwrap()
            .contains(&json!("rauthy_admin")),
        "roles lists the existing roles"
    );
    let ops = body["ops"].as_array().unwrap();
    for o in [
        "eq", "gte", "between", "in", "not_in", "contains", "is_set", "not_set",
    ] {
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
        json!([{"field": "age", "op": "between", "value": [65, 18]}]),
        json!([{"field": "age", "op": "between", "value": [18]}]),
        json!([{"field": "country", "op": "is_set", "value": "x"}]),
        json!([{"field": "country", "op": "is_set", "message": "two\nlines"}]),
        json!([{"field": "country", "op": "is_set", "message": "x".repeat(201)}]),
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

    // the user hears the message of the first rule that fails, or the general one
    let refusal = |res: reqwest::Response| async move {
        assert_eq!(res.status(), 403);
        let body: Value = res.json().await.unwrap();
        body["message"].as_str().unwrap().to_string()
    };
    let res = put_rules(json!([
        {"field": "email", "op": "ends_with", "value": "@localhost"},
        {"field": "country", "op": "in", "value": ["UG"], "message": "Only for people in Uganda"},
        {"field": "age", "op": "gte", "value": 18, "message": "You must be 18 or older"}
    ]))
    .await?;
    check_status(res, 200).await?;
    assert_eq!(
        get_rules().await?["rules"][1]["message"],
        "Only for people in Uganda",
        "the message is stored"
    );
    let said = refusal(token_request(&password_grant(&secret)).await?).await;
    assert_eq!(
        said, "Only for people in Uganda",
        "the first failing rule speaks"
    );
    let res = put_rules(json!([
        {"field": "country", "op": "in", "value": ["UG"]}
    ]))
    .await?;
    check_status(res, 200).await?;
    let said = refusal(token_request(&password_grant(&secret)).await?).await;
    assert_eq!(
        said, "Your account does not meet the requirements to access this application",
        "without a message of its own the general one is used"
    );
    // restore the rule the refresh check below relies on
    let res =
        put_rules(json!([{"field": "email", "op": "ends_with", "value": "@nowhere.invalid"}]))
            .await?;
    check_status(res, 200).await?;

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
        // a true / false field is compared with true or false, not with text
        json!([{"field": "email_verified", "op": "in", "value": [true, false]}]),
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

    // The required user data comes first. The test user has only an email and a name, like a user
    // who signed in with Google. The client requires the country and a rule is set on it: the user
    // must be sent to fill it in (205), not be refused (403) for a value never asked for.
    set_country(None).await?;
    check_status(put_required(json!(["country"])).await?, 200).await?;
    let res = put_rules(json!([{"field": "country", "op": "in", "value": ["UG"]}])).await?;
    check_status(res, 200).await?;
    assert_eq!(
        authorize_status().await?,
        205,
        "a missing required field comes before the rules"
    );
    // once the country is there, the rule decides
    set_country(Some("DE")).await?;
    assert_eq!(authorize_status().await?, 403, "now the rule refuses");
    set_country(Some("UG")).await?;
    assert_eq!(authorize_status().await?, 202, "and now it lets in");
    // The group of the client comes first of all. A user outside of it is turned away at once, also
    // when data is missing and a rule would not be met: they are not sent to fill in a profile for an
    // application they may not use.
    set_country(None).await?;
    delete_group_client().await;
    create_group_client("no_such_group_prefix").await?;
    let url = format!("{}/clients/{GROUP_CLIENT}", get_backend_url());
    let auth = get_auth_headers().await?;
    let put = |path: &'static str, body: Value| {
        let (url, auth) = (format!("{url}/{path}"), auth.clone());
        async move {
            let res = reqwest::Client::new()
                .put(url)
                .headers(auth)
                .json(&body)
                .send()
                .await?;
            check_status(res, 200).await?;
            Ok::<(), Box<dyn Error>>(())
        }
    };
    put("required_fields", json!({"fields": ["country"]})).await?;
    put(
        "access_rules",
        json!({"rules": [{"field": "country", "op": "in", "value": ["UG"]}]}),
    )
    .await?;
    let (status, body) = authorize_login_for(GROUP_CLIENT).await?;
    assert_eq!(
        status, 403,
        "the group check comes before the required data"
    );
    assert!(
        body.contains("permission to view this application"),
        "the user is told about the application: {body}"
    );
    delete_group_client().await;

    // the grants that cannot ask the user for anything keep checking the rules
    set_country(None).await?;
    let res = token_request(&password_grant(&secret)).await?;
    assert_eq!(
        res.status(),
        403,
        "the password grant has no form to fill in"
    );
    check_status(put_required(json!([])).await?, 200).await?;
    assert_eq!(
        authorize_status().await?,
        403,
        "nothing is required, so the rule refuses at once"
    );

    // a user without groups: positive rules fail, negations hold
    let url = format!("{}/users", get_backend_url());
    let users: Value = check_status(
        reqwest::Client::new()
            .get(&url)
            .headers(get_auth_headers().await?)
            .send()
            .await?,
        200,
    )
    .await?
    .json()
    .await?;
    let me = users
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == USERNAME)
        .unwrap();
    let has_groups = me["groups"].as_array().is_some_and(|g| !g.is_empty());
    for (rule, passes) in [
        (
            json!({"field": "groups", "op": "not_in", "value": ["no_such_group"]}),
            true,
        ),
        (
            json!({"field": "groups", "op": "ne", "value": "no_such_group"}),
            true,
        ),
        (
            json!({"field": "groups", "op": "in", "value": ["no_such_group"]}),
            false,
        ),
        (
            json!({"field": "groups", "op": "contains", "value": "no_such_group"}),
            false,
        ),
    ] {
        check_status(put_rules(json!([rule.clone()])).await?, 200).await?;
        let res = token_request(&password_grant(&secret)).await?;
        let want = if passes { 200 } else { 403 };
        assert_eq!(res.status(), want, "{rule} (user has groups: {has_groups})");
    }
    // a user without a country is not in a country they must not be in
    check_status(
        put_rules(json!([{"field": "country", "op": "not_in", "value": ["RU"]}])).await?,
        200,
    )
    .await?;
    let res = token_request(&password_grant(&secret)).await?;
    assert_eq!(res.status(), 200, "no country is not RU");

    Ok(())
}
