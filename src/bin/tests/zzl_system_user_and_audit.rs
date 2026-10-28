use crate::common::{USERNAME, check_status, get_auth_headers, get_backend_url, session_headers};
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
    assert_eq!(first["action"], "settings_changed");
    assert_eq!(first["target_type"], "settings");
    assert_eq!(first["target_id"], "new_users");
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
