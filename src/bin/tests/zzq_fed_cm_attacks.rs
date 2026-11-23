//! Fork test: attacks against the FedCM endpoints, one per way a real attacker would try.
//!
//! Each case sends something a hostile page, a stolen or forged cookie, or a script would send,
//! and checks that no ID token comes back. A case that passes with a 200 means the defence is
//! missing, so every case also has a control (the same request without the attack) in
//! `zzp_fed_cm_gate`, which proves the request itself is fine.
//!
//! The test backend must run with `EXPERIMENTAL_FED_CM_ENABLE=true` (the `justfile` sets it).

use crate::common::get_backend_url;
use fed_cm_common::*;
use reqwest::header::{COOKIE, HeaderMap, ORIGIN};
use serde_json::{Value, json};
use std::error::Error;

mod common;
mod fed_cm_common;

const CLIENTS: &[&str] = &["fedcm_atk_a", "fedcm_atk_b"];

/// A request to `/fed_cm/token` that lets the test choose every header and form field
async fn raw_token(
    cookie: Option<&str>,
    headers: &[(&str, &str)],
    form: &[(&str, &str)],
) -> Result<(u16, HeaderMap, String), Box<dyn Error>> {
    let mut req = reqwest::Client::new().post(format!("{}/fed_cm/token", get_backend_url()));
    if let Some(cookie) = cookie {
        req = req.header(COOKIE, cookie);
    }
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let res = req.form(form).send().await?;
    let status = res.status().as_u16();
    let headers = res.headers().clone();
    Ok((status, headers, res.text().await?))
}

/// The headers of a request the browser would send
fn browser<'a>(origin: &'a str) -> Vec<(&'a str, &'a str)> {
    vec![("sec-fetch-dest", "webidentity"), ("origin", origin)]
}

fn form<'a>(client: &'a str, account: &'a str, nonce: &'a str) -> Vec<(&'a str, &'a str)> {
    vec![
        ("client_id", client),
        ("account_id", account),
        ("nonce", nonce),
        ("disclosure_text_shown", "true"),
    ]
}

/// no token, and the status is one of the refusals (never a 200 or a server error)
fn no_token(res: &(u16, HeaderMap, String), why: &str) {
    assert!(
        (400..500).contains(&res.0),
        "{why}: expected a client error, got {}: {}",
        res.0,
        res.2
    );
    assert!(
        !res.2.contains("\"token\""),
        "{why}: a token was handed out: {}",
        res.2
    );
}

#[tokio::test]
async fn test_fed_cm_attacks() -> Result<(), Box<dyn Error>> {
    let _cleanup = Cleanup(CLIENTS);
    for id in CLIENTS {
        let _ = reqwest::Client::new()
            .delete(format!("{}/clients/{id}", get_backend_url()))
            .headers(crate::common::get_auth_headers().await?)
            .send()
            .await;
    }
    let user_id = my_user_id().await?;
    make_client("fedcm_atk_a", |_| {}).await?;
    make_client("fedcm_atk_b", |_| {}).await?;
    let cookie = fed_cm_cookie().await?;

    // control: the request below works, so the refusals that follow are about the attack
    let res = raw_token(
        Some(&cookie),
        &browser(ORIGIN_OK),
        &form("fedcm_atk_a", &user_id, "ctrl1"),
    )
    .await?;
    assert_eq!(res.0, 200, "control: {}", res.2);

    // 1. a script cannot be the browser: `Sec-Fetch-Dest` is set by the browser and no page can
    // forge it, so every FedCM endpoint refuses a request without it, even with a valid cookie
    let no_dest = [("origin", ORIGIN_OK)];
    let res = raw_token(
        Some(&cookie),
        &no_dest,
        &form("fedcm_atk_a", &user_id, "n1"),
    )
    .await?;
    no_token(&res, "token without Sec-Fetch-Dest");
    for (path, method) in [
        ("fed_cm/accounts", "GET"),
        ("fed_cm/config", "GET"),
        ("fed_cm/client_meta?client_id=fedcm_atk_a", "GET"),
    ] {
        let res = reqwest::Client::new()
            .request(method.parse()?, format!("{}/{path}", get_backend_url()))
            .header(COOKIE, &cookie)
            .send()
            .await?;
        let status = res.status().as_u16();
        let body = res.text().await?;
        assert!(
            (400..500).contains(&status),
            "{path} without Sec-Fetch-Dest: {status} {body}"
        );
        assert!(!body.contains("approved_clients"), "{path} leaked: {body}");
    }
    // the status endpoint is also used by the login page itself, so it has no `Sec-Fetch-Dest`
    // rule, but a page of another site must not learn from it whether someone is signed in
    for site in ["cross-site", "same-site"] {
        let res = reqwest::Client::new()
            .get(format!("{}/fed_cm/status", get_backend_url()))
            .header(COOKIE, &cookie)
            .header("sec-fetch-site", site)
            .send()
            .await?;
        assert_eq!(res.status(), 401, "status for a {site} request");
    }
    // a wrong value is as bad as none
    let res = raw_token(
        Some(&cookie),
        &[("sec-fetch-dest", "empty"), ("origin", ORIGIN_OK)],
        &form("fedcm_atk_a", &user_id, "n2"),
    )
    .await?;
    no_token(&res, "token with Sec-Fetch-Dest: empty");

    // 2. forged, damaged and foreign cookies
    let (name, value) = cookie.split_once('=').unwrap();
    let flipped = {
        let mut v: Vec<char> = value.chars().collect();
        let i = v.len() / 2;
        v[i] = if v[i] == 'A' { 'B' } else { 'A' };
        v.into_iter().collect::<String>()
    };
    let forged = [
        ("a flipped character", format!("{name}={flipped}")),
        (
            "a truncated value",
            format!("{name}={}", &value[..value.len() / 2]),
        ),
        ("an empty value", format!("{name}=")),
        (
            "random text",
            format!("{name}=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),
        ),
        (
            "a session id as plain text",
            format!("{name}=jqEDK3wvJsqe8AUzZo4N6Sds5k7maU9X"),
        ),
        // the cookie without the `__Host-` prefix is a different cookie that an attacker on a
        // sibling subdomain could set; the server must not read it
        ("no __Host- prefix", format!("RauthySessionFedCM={value}")),
        ("a different name", format!("Other={value}")),
    ];
    for (what, c) in forged {
        let res = raw_token(
            Some(&c),
            &browser(ORIGIN_OK),
            &form("fedcm_atk_a", &user_id, "n3"),
        )
        .await?;
        no_token(&res, what);
        assert_eq!(res.0, 401, "{what}: {}", res.2);
    }

    // 2b. a cookie value that is too short to hold the encryption header. `panic = abort` turned
    // this into one request that stopped the whole server, on every endpoint that reads a cookie.
    for value in [
        "",
        "A",
        "AA",
        "AAAA",
        "AAAAAAA",
        "AAAAAAAA",
        "AAAAAAAAAAAA",
        "-",
        "====",
        "%00",
    ] {
        for name in ["__Host-RauthySessionFedCM", "__Host-RauthySession"] {
            let res = reqwest::Client::new()
                .get(format!("{}/oidc/sessioninfo", get_backend_url()))
                .header(COOKIE, format!("{name}={value}"))
                .send()
                .await?;
            assert!(
                matches!(res.status().as_u16(), 400..=499),
                "short cookie {name}={value:?}: {}",
                res.status()
            );
        }
        let res = raw_token(
            Some(&format!("__Host-RauthySessionFedCM={value}")),
            &browser(ORIGIN_OK),
            &form("fedcm_atk_a", &user_id, "short"),
        )
        .await?;
        no_token(&res, &format!("short cookie value {value:?}"));
    }
    // the server is still there
    let res = raw_token(
        Some(&cookie),
        &browser(ORIGIN_OK),
        &form("fedcm_atk_a", &user_id, "alive"),
    )
    .await?;
    assert_eq!(
        res.0, 200,
        "the server must survive short cookies: {}",
        res.2
    );

    // 3. origins a hostile page could have. A browser sets `Origin` itself, so these are the
    // origins of other sites that try to get a token for this client.
    for origin in [
        "null",
        "https://evil.example",
        "http://localhost:3000.evil.example",
        "http://localhost:3000@evil.example",
        "http://evil.example#http://localhost:3000",
        "http://evil.example/http://localhost:3000",
        "http://localhost:30000",
        "http://localhost:300",
        "http://localhost:3001",
        "https://localhost:3000",
        "http://xlocalhost:3000",
        "http://localhost",
        "localhost:3000",
        "",
    ] {
        let res = raw_token(
            Some(&cookie),
            &browser(origin),
            &form("fedcm_atk_a", &user_id, "n4"),
        )
        .await?;
        no_token(&res, &format!("origin {origin:?}"));
    }
    // no `Origin` at all
    let res = raw_token(
        Some(&cookie),
        &[("sec-fetch-dest", "webidentity")],
        &form("fedcm_atk_a", &user_id, "n5"),
    )
    .await?;
    no_token(&res, "no Origin header");

    // 4. the nonce: it is what stops a leaked token from being replayed, so it must be real
    let long = "a".repeat(5000);
    let long_params = format!(r#"{{"nonce":"{}"}}"#, "a".repeat(600));
    for (what, fields) in [
        ("a 5000 character nonce", vec![("nonce", long.as_str())]),
        ("a control character", vec![("nonce", "a\nb")]),
        ("an empty nonce", vec![("nonce", "")]),
        ("params that are no JSON", vec![("params", "{nonce:abc")]),
        ("params without a nonce", vec![("params", r#"{"x":1}"#)]),
        ("a number as nonce", vec![("params", r#"{"nonce":12345}"#)]),
        ("a list as nonce", vec![("params", r#"{"nonce":["a"]}"#)]),
        ("a null nonce", vec![("params", r#"{"nonce":null}"#)]),
        ("an empty params nonce", vec![("params", r#"{"nonce":""}"#)]),
        (
            "a 600 character params nonce",
            vec![("params", long_params.as_str())],
        ),
        ("params that are an array", vec![("params", r#"["nonce"]"#)]),
        ("no nonce at all", vec![]),
    ] {
        let mut f = vec![
            ("client_id", "fedcm_atk_a"),
            ("account_id", user_id.as_str()),
            ("disclosure_text_shown", "true"),
        ];
        f.extend(fields);
        let res = raw_token(Some(&cookie), &browser(ORIGIN_OK), &f).await?;
        no_token(&res, what);
    }

    // 5. client ids an attacker picks
    let long_id = "c".repeat(300);
    for (what, id) in [
        ("an unknown client", "fedcm_atk_does_not_exist"),
        ("a quote", "x' OR '1'='1"),
        ("a SQL comment", "fedcm_atk_a'--"),
        ("a path", "../../clients/fedcm_atk_a"),
        ("an overlong id", long_id.as_str()),
        ("an empty id", ""),
        ("a client name that only starts alike", "fedcm_atk_"),
        // an ephemeral client: its id is a URL the server would have to fetch
        ("an ephemeral client", "http://localhost:3000/client.json"),
    ] {
        let res = raw_token(
            Some(&cookie),
            &browser(ORIGIN_OK),
            &form(id, &user_id, "n6"),
        )
        .await?;
        assert_ne!(res.0, 200, "{what}: {}", res.2);
        assert!(
            !res.2.contains("\"token\""),
            "{what}: a token was handed out: {}",
            res.2
        );
    }

    // 6. other accounts: ids of users that are not the one of this session
    for account in ["", "admin", "0", "za9UxpH7XVxqrtpEbThoqvn2' OR '1'='1", "*"] {
        let res = raw_token(
            Some(&cookie),
            &browser(ORIGIN_OK),
            &form("fedcm_atk_a", account, "n7"),
        )
        .await?;
        no_token(&res, &format!("account {account:?}"));
    }

    // 7. what a legitimate token is bound to: one client, the nonce asked for, no refresh or
    // access token that would outlive it, and an answer only the allowed origin can read
    let a = raw_token(
        Some(&cookie),
        &browser(ORIGIN_OK),
        &form("fedcm_atk_a", &user_id, "bind-a"),
    )
    .await?;
    let b = raw_token(
        Some(&cookie),
        &browser(ORIGIN_OK),
        &form("fedcm_atk_b", &user_id, "bind-b"),
    )
    .await?;
    assert_eq!(a.0, 200);
    assert_eq!(b.0, 200);
    let body_a: Value = serde_json::from_str(&a.2)?;
    let claims_a = common::decode_claims(body_a["token"].as_str().unwrap());
    let body_b: Value = serde_json::from_str(&b.2)?;
    let claims_b = common::decode_claims(body_b["token"].as_str().unwrap());
    assert_eq!(claims_a["aud"], "fedcm_atk_a");
    assert_eq!(claims_b["aud"], "fedcm_atk_b");
    assert_eq!(claims_a["nonce"], "bind-a");
    assert_eq!(claims_b["nonce"], "bind-b");
    for k in ["access_token", "refresh_token"] {
        assert!(body_a.get(k).is_none(), "{k} must not be in a FedCM answer");
    }
    assert_eq!(
        a.1.get("access-control-allow-origin")
            .map(|v| v.to_str().unwrap()),
        Some(ORIGIN_OK),
        "only the origin of the client may read the answer"
    );
    assert!(
        a.1.get("set-cookie").is_none(),
        "no cookie is set by the token"
    );
    let exp = claims_a["exp"].as_i64().unwrap();
    let iat = claims_a["iat"].as_i64().unwrap();
    assert!(
        exp - iat <= 3600,
        "an ID token for FedCM lives {} s",
        exp - iat
    );

    // 8. the wrong kind of request
    let res = reqwest::Client::new()
        .post(format!("{}/fed_cm/token", get_backend_url()))
        .header(COOKIE, &cookie)
        .header("sec-fetch-dest", "webidentity")
        .header(ORIGIN, ORIGIN_OK)
        .json(&json!({"client_id": "fedcm_atk_a", "account_id": user_id, "nonce": "j1"}))
        .send()
        .await?;
    let status = res.status().as_u16();
    let body = res.text().await?;
    assert!((400..500).contains(&status), "a JSON body: {status} {body}");
    assert!(!body.contains("\"token\""));
    let res = reqwest::Client::new()
        .get(format!(
            "{}/fed_cm/token?client_id=fedcm_atk_a&account_id={user_id}&nonce=g1",
            get_backend_url()
        ))
        .header(COOKIE, &cookie)
        .header("sec-fetch-dest", "webidentity")
        .header(ORIGIN, ORIGIN_OK)
        .send()
        .await?;
    assert!(res.status().as_u16() >= 400, "a GET to the token endpoint");
    assert!(!res.text().await?.contains("\"token\""));

    // 9. a burst of bad requests does not break the endpoint, and a good one still works
    for i in 0..40 {
        let nonce = format!("burst{i}");
        let res = raw_token(
            Some("__Host-RauthySessionFedCM=AAAA"),
            &browser(ORIGIN_OK),
            &form("fedcm_atk_a", &user_id, &nonce),
        )
        .await?;
        // a rate limit (429) is as good as a refusal
        assert!(res.0 == 401 || res.0 == 429, "burst: {}", res.0);
    }
    let res = raw_token(
        Some(&cookie),
        &browser(ORIGIN_OK),
        &form("fedcm_atk_a", &user_id, "after-burst"),
    )
    .await?;
    // the burst came from the same IP, so a limit may apply to the good request too
    assert!(
        res.0 == 200 || res.0 == 429,
        "after the burst: {} {}",
        res.0,
        res.2
    );

    Ok(())
}
