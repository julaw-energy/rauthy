use crate::common::{USERNAME, check_status, get_backend_url, session_headers};
use reqwest::header::HeaderMap;
use serde_json::{Value, json};
use std::error::Error;

mod common;

async fn send(
    method: reqwest::Method,
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

/// Puts the profile field settings back when the test ends, also when an assertion panics
struct Restore(Value, HeaderMap);

impl Drop for Restore {
    fn drop(&mut self) {
        let (values, headers) = (self.0.clone(), self.1.clone());
        let _ = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let _ = send(
                        reqwest::Method::PUT,
                        "/settings/user_values",
                        &headers,
                        Some(values),
                    )
                    .await;
                });
        })
        .join();
    }
}

/// A self-update must not clear a profile field the admin hid: the page cannot send it.
#[tokio::test]
async fn test_hidden_profile_field_survives_a_self_update() -> Result<(), Box<dyn Error>> {
    let (admin, _) = session_headers().await;

    // the settings as they are, to put back afterwards
    let res = send(reqwest::Method::GET, "/settings", &admin, None).await?;
    let all: Value = check_status(res, 200).await?.json().await?;
    let mut values = all["user_values"].clone();
    values.as_object_mut().unwrap().remove("from_db");
    let _restore = Restore(values.clone(), admin.clone());

    let set_tz = async |level: &str| -> Result<(), Box<dyn Error>> {
        let mut v = values.clone();
        v["tz"] = json!(level);
        let res = send(
            reqwest::Method::PUT,
            "/settings/user_values",
            &admin,
            Some(v),
        )
        .await?;
        check_status(res, 200).await?;
        Ok(())
    };

    // our own account
    let res = send(reqwest::Method::GET, "/users", &admin, None).await?;
    let users: Value = check_status(res, 200).await?.json().await?;
    let me = users
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == USERNAME)
        .expect("the admin exists");
    let id = me["id"].as_str().unwrap().to_string();
    let email = me["email"].as_str().unwrap().to_string();

    let update = async |user_values: Value| -> Result<Value, Box<dyn Error>> {
        let res = send(
            reqwest::Method::PUT,
            &format!("/users/{id}/self"),
            &admin,
            Some(json!({"email": email, "user_values": user_values})),
        )
        .await?;
        Ok(check_status(res, 200).await?.json().await?)
    };

    // visible: the user sets the timezone, and clears it again by sending none
    set_tz("optional").await?;
    let user = update(json!({"tz": "Africa/Kampala"})).await?;
    assert_eq!(user["user_values"]["tz"], "Africa/Kampala");

    // hidden: the page cannot send it, and the stored one stays
    set_tz("hidden").await?;
    let user = update(json!({"city": "Kampala"})).await?;
    assert_eq!(user["user_values"]["tz"], "Africa/Kampala", "kept");
    assert_eq!(
        user["user_values"]["city"], "Kampala",
        "visible fields change"
    );
    // also a request that tries to change it is ignored
    let user = update(json!({"tz": "Europe/Berlin"})).await?;
    assert_eq!(user["user_values"]["tz"], "Africa/Kampala", "ignored");

    // visible again: now the user decides
    set_tz("optional").await?;
    let user = update(json!({"tz": "Europe/Berlin"})).await?;
    assert_eq!(user["user_values"]["tz"], "Europe/Berlin");
    update(json!({})).await?;
    Ok(())
}
