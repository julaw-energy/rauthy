use crate::common::{
    check_status, get_auth_headers, get_backend_url, get_solved_pow, session_headers,
};
use rauthy_api_types::users::NewUserRegistrationRequest;
use reqwest::Method;
use reqwest::header::HeaderMap;
use serde_json::{Value, json};
use std::error::Error;

mod common;

async fn send(
    method: Method,
    path: &str,
    headers: HeaderMap,
    body: Option<Value>,
) -> Result<reqwest::Response, Box<dyn Error>> {
    let url = format!("{}{path}", get_backend_url());
    let mut req = reqwest::Client::new()
        .request(method, &url)
        .headers(headers);
    if let Some(body) = body {
        req = req.json(&body);
    }
    Ok(req.send().await?)
}

/// Puts the defaults for new users back when the test ends, also when an assertion panics
struct Cleanup(HeaderMap);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let admin = self.0.clone();
        let _ = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("a runtime for the cleanup")
                .block_on(async {
                    let _ = send(
                        Method::PUT,
                        "/settings/new_users",
                        admin,
                        Some(json!({"roles": [], "groups": []})),
                    )
                    .await;
                });
        })
        .join();
    }
}

/// One test, on purpose: both settings are global and the user is registered for real.
#[tokio::test]
async fn test_new_user_defaults_and_reminders() -> Result<(), Box<dyn Error>> {
    let (admin, _) = session_headers().await;
    let _cleanup = Cleanup(admin.clone());
    run(&admin).await
}

async fn run(admin: &HeaderMap) -> Result<(), Box<dyn Error>> {
    const GET: Method = Method::GET;
    const POST: Method = Method::POST;
    const PUT: Method = Method::PUT;
    const DELETE: Method = Method::DELETE;

    // -------------------------------------------------------------------------------------
    // roles and groups for new users
    // -------------------------------------------------------------------------------------
    let res = send(GET, "/settings/new_users", admin.clone(), None).await?;
    let body: Value = check_status(res, 200).await?.json().await?;
    assert_eq!(body["values"], json!({"roles": [], "groups": []}));
    let roles = body["available_roles"].as_array().unwrap();
    assert!(
        roles.contains(&json!("bs_role_1")),
        "existing roles are offered"
    );
    assert!(
        !roles.contains(&json!("rauthy_admin")),
        "rauthy_admin is never offered"
    );
    assert!(
        body["available_groups"]
            .as_array()
            .unwrap()
            .contains(&json!("bs_group_1"))
    );

    for bad in [
        json!({"roles": ["rauthy_admin"], "groups": []}),
        json!({"roles": ["rauthy_admin:sales"], "groups": []}),
        json!({"roles": ["no_such_role"], "groups": []}),
        json!({"roles": [], "groups": ["no_such_group"]}),
    ] {
        let res = send(PUT, "/settings/new_users", admin.clone(), Some(bad.clone())).await?;
        assert_eq!(res.status(), 400, "{bad} must be refused");
    }

    let res = send(
        PUT,
        "/settings/new_users",
        admin.clone(),
        Some(json!({"roles": ["bs_role_1", "bs_role_1"], "groups": ["bs_group_1"]})),
    )
    .await?;
    let body: Value = check_status(res, 200).await?.json().await?;
    assert_eq!(
        body["values"],
        json!({"roles": ["bs_role_1"], "groups": ["bs_group_1"]}),
        "saved without the duplicate"
    );

    // a user who registers gets them
    let email = "defaults.test@example.com";
    let reg = NewUserRegistrationRequest {
        email: email.to_string(),
        preferred_username: None,
        family_name: Some("Person".to_string()),
        given_name: Some("New".to_string()),
        user_values: None,
        pow: get_solved_pow().await,
        redirect_uri: None,
    };
    let res = send(
        POST,
        "/users/register",
        HeaderMap::new(),
        Some(serde_json::to_value(&reg)?),
    )
    .await?;
    assert!(res.status().is_success(), "registration: {}", res.status());

    let api = get_auth_headers().await?;
    let res = send(GET, "/users", api.clone(), None).await?;
    let users: Value = check_status(res, 200).await?.json().await?;
    let id = users
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == email)
        .expect("the registered user exists")["id"]
        .as_str()
        .unwrap()
        .to_string();
    let res = send(GET, &format!("/users/{id}"), api.clone(), None).await?;
    let user: Value = check_status(res, 200).await?.json().await?;
    assert_eq!(user["roles"], json!(["bs_role_1"]), "the default role");
    assert_eq!(user["groups"], json!(["bs_group_1"]), "the default group");

    // with no defaults a new user gets nothing
    let res = send(
        PUT,
        "/settings/new_users",
        admin.clone(),
        Some(json!({"roles": [], "groups": []})),
    )
    .await?;
    check_status(res, 200).await?;
    let email2 = "defaults.test2@example.com";
    let reg2 = NewUserRegistrationRequest {
        email: email2.to_string(),
        pow: get_solved_pow().await,
        ..reg
    };
    let res = send(
        POST,
        "/users/register",
        HeaderMap::new(),
        Some(serde_json::to_value(&reg2)?),
    )
    .await?;
    assert!(
        res.status().is_success(),
        "registration 2: {}",
        res.status()
    );
    let res = send(GET, "/users", api.clone(), None).await?;
    let users: Value = check_status(res, 200).await?.json().await?;
    let id2 = users
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == email2)
        .expect("the second user exists")["id"]
        .as_str()
        .unwrap()
        .to_string();
    let res = send(GET, &format!("/users/{id2}"), api.clone(), None).await?;
    let user2: Value = check_status(res, 200).await?.json().await?;
    assert_eq!(user2["roles"], json!([]));
    assert!(user2["groups"].is_null() || user2["groups"] == json!([]));

    for uid in [&id, &id2] {
        let res = send(DELETE, &format!("/users/{uid}"), api.clone(), None).await?;
        check_status(res, 204).await?;
    }

    // -------------------------------------------------------------------------------------
    // the reminder for users who have not logged in
    // -------------------------------------------------------------------------------------
    let res = send(GET, "/settings/reminders", admin.clone(), None).await?;
    let body: Value = check_status(res, 200).await?.json().await?;
    let v = &body["values"];
    assert_eq!(v["enabled"], false, "off until an admin turns it on");
    assert_eq!(v["after_days"], 30, "30 days by default");
    assert_eq!(v["max_reminders"], 3);
    assert!(!v["subject"].as_str().unwrap().is_empty());
    assert!(v["body"].as_str().unwrap().contains("{{link}}"));
    // the two accounts of the initial migrations date from 2023. Other tests of the suite log in
    // as one of them, so only "at least one" is certain.
    assert!(
        body["due_now"].as_u64().unwrap() >= 1,
        "an old default account is due: {}",
        body["due_now"]
    );

    let good = json!({
        "enabled": false, "after_days": 45, "max_reminders": 2,
        "subject": "We miss you at {{app_name}}", "body": "Hi {{name}}, {{days}} days. {{link}}"
    });
    let with = |k: &str, val: Value| {
        let mut m = good.clone();
        m[k] = val;
        m
    };
    for bad in [
        with("after_days", json!(0)),
        with("after_days", json!(3651)),
        with("max_reminders", json!(0)),
        with("max_reminders", json!(11)),
        with("subject", json!("")),
        with("subject", json!("two\nlines")),
        with("subject", json!("a {{link}} in the subject")),
        with("body", json!("")),
        with("body", json!("Hi {{nope}}")),
        with("body", json!("x".repeat(5001))),
    ] {
        let res = send(PUT, "/settings/reminders", admin.clone(), Some(bad.clone())).await?;
        assert_eq!(res.status(), 400, "{bad} must be refused");
    }

    let res = send(
        PUT,
        "/settings/reminders",
        admin.clone(),
        Some(good.clone()),
    )
    .await?;
    let body: Value = check_status(res, 200).await?.json().await?;
    assert_eq!(body["values"], good);
    let res = send(GET, "/settings/reminders", admin.clone(), None).await?;
    let body: Value = check_status(res, 200).await?.json().await?;
    assert_eq!(body["values"], good, "what was saved is what is read");

    // the number of days decides who is due: those 2023 accounts are about 3.5 years old
    let res = send(
        PUT,
        "/settings/reminders",
        admin.clone(),
        Some(with("after_days", json!(3000))),
    )
    .await?;
    let body: Value = check_status(res, 200).await?.json().await?;
    assert_eq!(body["due_now"], 0, "nobody was away for 3000 days");
    let res = send(
        PUT,
        "/settings/reminders",
        admin.clone(),
        Some(with("after_days", json!(1000))),
    )
    .await?;
    let body: Value = check_status(res, 200).await?.json().await?;
    assert!(
        body["due_now"].as_u64().unwrap() >= 1,
        "away for more than 1000 days"
    );
    let res = send(
        PUT,
        "/settings/reminders",
        admin.clone(),
        Some(good.clone()),
    )
    .await?;
    check_status(res, 200).await?;

    // the test mail goes to the admin and to nobody else
    let res = send(POST, "/settings/reminders/test", admin.clone(), None).await?;
    let body: Value = check_status(res, 200).await?.json().await?;
    assert!(body["message"].as_str().unwrap().contains(common::USERNAME));

    // only an admin may read or change it
    for (m, p) in [(GET, "/settings/reminders"), (GET, "/settings/new_users")] {
        let res = send(m, p, HeaderMap::new(), None).await?;
        assert_eq!(res.status(), 401, "{p} needs a session");
    }

    // back to the defaults
    let res = send(
        PUT,
        "/settings/reminders",
        admin.clone(),
        Some(json!({
            "enabled": false, "after_days": 30, "max_reminders": 3,
            "subject": v["subject"], "body": v["body"]
        })),
    )
    .await?;
    check_status(res, 200).await?;
    Ok(())
}
