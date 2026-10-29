use crate::common::{
    check_status, get_auth_headers, get_backend_url, get_solved_pow, session_headers,
};
use rauthy_api_types::users::NewUserRegistrationRequest;
use reqwest::Method;
use reqwest::header::HeaderMap;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::error::Error;

mod common;

async fn send(
    method: Method,
    path: &str,
    headers: &HeaderMap,
    body: Option<Value>,
) -> Result<reqwest::Response, Box<dyn Error>> {
    let mut req = reqwest::Client::new()
        .request(method, format!("{}{path}", get_backend_url()))
        .headers(headers.clone());
    if let Some(body) = body {
        req = req.json(&body);
    }
    Ok(req.send().await?)
}

fn groups_of(user: &Value) -> BTreeSet<String> {
    user["groups"]
        .as_array()
        .map(|a| a.iter().map(|g| g.as_str().unwrap().to_string()).collect())
        .unwrap_or_default()
}

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}

/// Adding and removing ONE group: no list is sent, other groups stay, repeats change nothing.
#[tokio::test]
async fn test_add_and_remove_one_group() -> Result<(), Box<dyn Error>> {
    let (admin, _) = session_headers().await;
    let api = get_auth_headers().await?;

    // a user and some groups of their own
    let email = "one.group@example.com";
    let reg = NewUserRegistrationRequest {
        email: email.to_string(),
        preferred_username: None,
        family_name: Some("Group".to_string()),
        given_name: Some("One".to_string()),
        user_values: None,
        pow: get_solved_pow().await,
        redirect_uri: None,
    };
    let res = send(
        Method::POST,
        "/users/register",
        &HeaderMap::new(),
        Some(serde_json::to_value(&reg)?),
    )
    .await?;
    assert!(res.status().is_success());
    let list: Value = check_status(send(Method::GET, "/users", &api, None).await?, 200)
        .await?
        .json()
        .await?;
    let id = list
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == email)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let names: Vec<String> = (0..8).map(|i| format!("one_grp_{i}")).collect();
    for n in &names {
        check_status(
            send(Method::POST, "/groups", &admin, Some(json!({ "group": n }))).await?,
            200,
        )
        .await?;
    }
    let url = |g: &str| format!("/users/{id}/groups/{g}");

    // add, add another: the first one stays without being sent again
    let user: Value = check_status(
        send(Method::PUT, &url("one_grp_0"), &admin, None).await?,
        200,
    )
    .await?
    .json()
    .await?;
    assert_eq!(groups_of(&user), set(&["one_grp_0"]));
    let user: Value = check_status(
        send(Method::PUT, &url("one_grp_1"), &admin, None).await?,
        200,
    )
    .await?
    .json()
    .await?;
    assert_eq!(groups_of(&user), set(&["one_grp_0", "one_grp_1"]));
    // the same again changes nothing
    let user: Value = check_status(
        send(Method::PUT, &url("one_grp_1"), &admin, None).await?,
        200,
    )
    .await?
    .json()
    .await?;
    assert_eq!(groups_of(&user), set(&["one_grp_0", "one_grp_1"]));

    // a group that does not exist, a user that does not exist, the system user, nobody logged in
    let res = send(Method::PUT, &url("no_such_group"), &admin, None).await?;
    assert_eq!(res.status(), 404, "unknown group");
    let res = send(
        Method::PUT,
        "/users/no_such_user/groups/one_grp_0",
        &admin,
        None,
    )
    .await?;
    assert_eq!(res.status(), 404, "unknown user");
    let res = send(Method::PUT, "/users/101/groups/one_grp_0", &admin, None).await?;
    assert_eq!(res.status(), 403, "system user");
    let res = send(Method::PUT, &url("one_grp_2"), &HeaderMap::new(), None).await?;
    assert_eq!(res.status(), 401);
    let res = send(Method::DELETE, &url("one_grp_0"), &HeaderMap::new(), None).await?;
    assert_eq!(res.status(), 401);

    // remove one: the other stays; removing it again changes nothing
    let user: Value = check_status(
        send(Method::DELETE, &url("one_grp_0"), &admin, None).await?,
        200,
    )
    .await?
    .json()
    .await?;
    assert_eq!(groups_of(&user), set(&["one_grp_1"]));
    let user: Value = check_status(
        send(Method::DELETE, &url("one_grp_0"), &admin, None).await?,
        200,
    )
    .await?
    .json()
    .await?;
    assert_eq!(groups_of(&user), set(&["one_grp_1"]));
    // a group whose name only starts like another one is not touched
    check_status(
        send(
            Method::POST,
            "/groups",
            &admin,
            Some(json!({ "group": "one_grp_1_long" })),
        )
        .await?,
        200,
    )
    .await?;
    check_status(
        send(Method::PUT, &url("one_grp_1_long"), &admin, None).await?,
        200,
    )
    .await?;
    let user: Value = check_status(
        send(Method::DELETE, &url("one_grp_1"), &admin, None).await?,
        200,
    )
    .await?
    .json()
    .await?;
    assert_eq!(groups_of(&user), set(&["one_grp_1_long"]));
    check_status(
        send(Method::DELETE, &url("one_grp_1_long"), &admin, None).await?,
        200,
    )
    .await?;
    let user: Value = check_status(
        send(Method::GET, &format!("/users/{id}"), &admin, None).await?,
        200,
    )
    .await?
    .json()
    .await?;
    assert!(groups_of(&user).is_empty(), "no groups left");

    // eight changes at the same moment: none of them may overwrite another
    let mut tasks = Vec::new();
    for n in &names {
        let (admin, path) = (admin.clone(), url(n));
        tasks.push(tokio::spawn(async move {
            send(Method::PUT, &path, &admin, None)
                .await
                .unwrap()
                .status()
        }));
    }
    for t in tasks {
        assert_eq!(t.await?, 200);
    }
    let user: Value = check_status(
        send(Method::GET, &format!("/users/{id}"), &admin, None).await?,
        200,
    )
    .await?
    .json()
    .await?;
    assert_eq!(
        groups_of(&user),
        names.iter().cloned().collect::<BTreeSet<_>>(),
        "all eight were added"
    );
    let mut tasks = Vec::new();
    for n in names.iter().take(7) {
        let (admin, path) = (admin.clone(), url(n));
        tasks.push(tokio::spawn(async move {
            send(Method::DELETE, &path, &admin, None)
                .await
                .unwrap()
                .status()
        }));
    }
    for t in tasks {
        assert_eq!(t.await?, 200);
    }
    let user: Value = check_status(
        send(Method::GET, &format!("/users/{id}"), &admin, None).await?,
        200,
    )
    .await?
    .json()
    .await?;
    assert_eq!(
        groups_of(&user),
        set(&["one_grp_7"]),
        "only the last one is left"
    );

    // the audit log shows a change of the user, not its deletion
    let res = send(
        Method::GET,
        &format!("/events/audit?target_id={id}&limit=5"),
        &admin,
        None,
    )
    .await?;
    let entries: Value = check_status(res, 200).await?.json().await?;
    let newest = &entries.as_array().unwrap()[0];
    assert_eq!(newest["action"], "update");
    assert!(newest["details"].as_str().unwrap().contains("/groups/"));

    // clean up
    check_status(
        send(Method::DELETE, &format!("/users/{id}"), &api, None).await?,
        204,
    )
    .await?;
    for n in names.iter().map(String::as_str).chain(["one_grp_1_long"]) {
        let list: Value = check_status(send(Method::GET, "/groups", &admin, None).await?, 200)
            .await?
            .json()
            .await?;
        if let Some(g) = list.as_array().unwrap().iter().find(|g| g["name"] == n) {
            let gid = g["id"].as_str().unwrap();
            send(Method::DELETE, &format!("/groups/{gid}"), &admin, None).await?;
        }
    }
    Ok(())
}
