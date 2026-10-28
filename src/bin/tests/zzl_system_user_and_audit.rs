use crate::common::{
    USERNAME, check_status, get_auth_headers, get_backend_url, get_solved_pow, session_headers,
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

#[tokio::test]
async fn test_system_user_and_audit_log() -> Result<(), Box<dyn Error>> {
    let (admin, _) = session_headers().await;
    let api = get_auth_headers().await?;

    // the system user exists, and is not able to log in
    let res = send(Method::GET, "/users", api.clone(), None).await?;
    let users: Value = check_status(res, 200).await?.json().await?;
    let users = users.as_array().unwrap();
    let system = users
        .iter()
        .find(|u| u["id"] == "system")
        .expect("the system user exists");
    assert_eq!(system["email"], "system@rauthy.invalid");
    let res = send(Method::GET, "/users/system", api.clone(), None).await?;
    let detail: Value = check_status(res, 200).await?.json().await?;
    assert_eq!(detail["enabled"], false, "it cannot log in");
    let admin_id = users.iter().find(|u| u["email"] == USERNAME).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // nobody can remove or change it
    let res = send(Method::DELETE, "/users/system", api.clone(), None).await?;
    assert_eq!(res.status(), 403, "the system user cannot be deleted");

    // an admin who changes a setting is the actor of that entry
    let res = send(
        Method::PUT,
        "/settings/new_users",
        admin.clone(),
        Some(json!({"roles": [], "groups": []})),
    )
    .await?;
    check_status(res, 200).await?;

    let res = send(
        Method::GET,
        "/events/audit?target_id=new_users&limit=5",
        admin.clone(),
        None,
    )
    .await?;
    let entries: Value = check_status(res, 200).await?.json().await?;
    let first = &entries.as_array().unwrap()[0];
    assert_eq!(first["actor"], admin_id.as_str());
    assert_eq!(
        first["actor_label"], USERNAME,
        "the actor is named by email"
    );
    assert_eq!(first["action"], "update");
    assert_eq!(first["target_type"], "settings");
    assert_eq!(first["target_id"], "new_users");
    assert_eq!(first["details"], "PUT /settings/new_users");
    let at = first["at"].as_i64().unwrap();

    // newest first, and paging back with `before` leaves the newest out
    let res = send(Method::GET, "/events/audit?limit=200", admin.clone(), None).await?;
    let all: Value = check_status(res, 200).await?.json().await?;
    let all = all.as_array().unwrap();
    assert!(
        all.windows(2)
            .all(|w| w[0]["at"].as_i64() >= w[1]["at"].as_i64())
    );
    let res = send(
        Method::GET,
        &format!("/events/audit?target_id=new_users&before={at}"),
        admin.clone(),
        None,
    )
    .await?;
    let older: Value = check_status(res, 200).await?.json().await?;
    assert!(
        older
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["at"].as_i64().unwrap() < at)
    );

    // creating something: the new thing is named in the entry, and the summary says who created it
    let res = send(
        Method::POST,
        "/roles",
        admin.clone(),
        Some(json!({"role": "audit_test_role"})),
    )
    .await?;
    let role: Value = check_status(res, 200).await?.json().await?;
    let role_id = role["id"].as_str().unwrap().to_string();
    let res = send(
        Method::GET,
        &format!("/events/audit/summary?target_id={role_id}"),
        admin.clone(),
        None,
    )
    .await?;
    let sum: Value = check_status(res, 200).await?.json().await?;
    assert_eq!(sum["created_by"], admin_id.as_str());
    assert_eq!(sum["created_by_label"], USERNAME);
    assert_eq!(sum["modified_by"], admin_id.as_str());
    assert!(sum["created_at"].as_i64().unwrap() > 0);
    // something that has no entries has no creator
    let res = send(
        Method::GET,
        "/events/audit/summary?target_id=nothing_here",
        admin.clone(),
        None,
    )
    .await?;
    let none: Value = check_status(res, 200).await?.json().await?;
    assert!(none["created_by"].is_null() && none["modified_by"].is_null());
    let res = send(
        Method::DELETE,
        &format!("/roles/{role_id}"),
        admin.clone(),
        None,
    )
    .await?;
    check_status(res, 200).await?;
    let res = send(
        Method::GET,
        &format!("/events/audit?target_id={role_id}"),
        admin.clone(),
        None,
    )
    .await?;
    let entries: Value = check_status(res, 200).await?.json().await?;
    let actions: Vec<&str> = entries
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["action"].as_str().unwrap())
        .collect();
    assert_eq!(
        actions,
        ["delete", "create"],
        "newest first, reads are not recorded"
    );

    // a user who is deleted is still named, until the personal data is wiped
    let email = "audit.gone@example.com";
    let reg = NewUserRegistrationRequest {
        email: email.to_string(),
        preferred_username: None,
        family_name: Some("Gone".to_string()),
        given_name: Some("Soon".to_string()),
        user_values: None,
        pow: get_solved_pow().await,
        redirect_uri: None,
    };
    let res = send(
        Method::POST,
        "/users/register",
        HeaderMap::new(),
        Some(serde_json::to_value(&reg)?),
    )
    .await?;
    assert!(res.status().is_success());
    let res = send(Method::GET, "/users", api.clone(), None).await?;
    let list: Value = check_status(res, 200).await?.json().await?;
    let gone_id = list
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == email)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let res = send(
        Method::DELETE,
        &format!("/users/{gone_id}"),
        api.clone(),
        None,
    )
    .await?;
    check_status(res, 204).await?;
    let entries = audit_for(&admin, &gone_id).await?;
    assert_eq!(entries[0]["action"], "delete");
    assert_eq!(entries[0]["actor"], admin_id.as_str());
    assert_eq!(
        entries[0]["target_label"], email,
        "named from the kept copy"
    );

    let res = send(
        Method::DELETE,
        &format!("/events/audit/users/{gone_id}"),
        admin.clone(),
        None,
    )
    .await?;
    check_status(res, 204).await?;
    let entries = audit_for(&admin, &gone_id).await?;
    assert!(
        entries[0]["target_label"].is_null(),
        "wiped, only the id is left"
    );
    let res = send(
        Method::DELETE,
        "/events/audit/users/no_such_user",
        admin.clone(),
        None,
    )
    .await?;
    assert_eq!(res.status(), 404);

    // bad parameters and no session
    for bad in ["limit=0", "limit=201", "before=-1"] {
        let res = send(
            Method::GET,
            &format!("/events/audit?{bad}"),
            admin.clone(),
            None,
        )
        .await?;
        assert_eq!(res.status(), 400, "{bad}");
    }
    let res = send(Method::GET, "/events/audit", HeaderMap::new(), None).await?;
    assert_eq!(res.status(), 401);
    Ok(())
}

async fn audit_for(admin: &HeaderMap, target: &str) -> Result<Vec<Value>, Box<dyn Error>> {
    let res = send(
        Method::GET,
        &format!("/events/audit?target_id={target}"),
        admin.clone(),
        None,
    )
    .await?;
    let v: Value = check_status(res, 200).await?.json().await?;
    Ok(v.as_array().unwrap().clone())
}
