//! Fork file. The checks a user must pass before ANY way of signing in hands out a token for a client.
//!
//! The browser login (`finish_authorize`) runs these checks inline, in its own order, because it can
//! also send the user to set up MFA or to complete the profile. Ways of signing in that have no page
//! to send the user to (FedCM) call this instead. They fail closed: whatever the normal login would
//! have turned into a "do this first" step becomes a refusal here, and the app falls back to the
//! normal login, which can handle it.
//!
//! Keep the order and the rules in line with `finish_authorize`, so that no way of signing in is
//! weaker than another.

use rauthy_data::entity::client_access_rules::ClientAccessRules;
use rauthy_data::entity::client_required_fields::ClientRequiredFields;
use rauthy_data::entity::clients::Client;
use rauthy_data::entity::sessions::Session;
use rauthy_data::entity::users::User;
use rauthy_error::{ErrorResponse, ErrorResponseType};

/// `session_used_mfa` is whether the session the user signs in with was itself opened with a second
/// factor. Having one set up is not enough: a session opened with the password only must not satisfy
/// `force_mfa` just because the account also has a passkey.
pub async fn validate_user_for_client(
    client: &Client,
    user: &User,
    session_used_mfa: bool,
) -> Result<(), ErrorResponse> {
    client.validate_enabled()?;
    user.check_enabled()?;
    user.check_expired()?;

    // 1. the group the client is restricted to
    client.validate_user_groups(user)?;

    // 2. MFA. The `rauthy` client itself is handled by the admin / account checks
    if client.id != "rauthy" && client.force_mfa && !session_used_mfa {
        return Err(ErrorResponse::new(
            ErrorResponseType::MfaRequired,
            "MFA is required for this client",
        ));
    }

    // 3. the data the client requires. The normal login sends the user to fill it in; without a
    // page to do that, we refuse, which is the safe side: the access rules below must never run
    // (or be skipped) on a profile the client never saw complete.
    if client.id != "rauthy"
        && !ClientRequiredFields::missing_for(user, &client.id)
            .await?
            .is_empty()
    {
        return Err(ErrorResponse::new(
            ErrorResponseType::Forbidden,
            "This application needs more data in your profile. Please sign in the normal way once to complete it",
        ));
    }

    // 4. rules on the user's values and attributes
    ClientAccessRules::validate_user(user, &client.id).await?;

    Ok(())
}

/// When the session was opened, in seconds since the epoch: the `auth_time` of a token that is issued
/// without a new login. Same calculation as the `max_age` check of the authorize endpoint.
pub fn session_auth_time(session: &Session) -> i64 {
    let lifetime = rauthy_data::settings::lifetimes()
        .session_lifetime
        .as_secs() as i64;
    // a lifetime that was lowered since would put the start in the future
    (session.exp - lifetime).min(chrono::Utc::now().timestamp())
}

/// A confidential client proves itself with a secret when it exchanges a code. FedCM hands the token
/// to the browser page, with nothing that proves the receiver is the client's backend. So FedCM is
/// for public clients, and a confidential one only gets it when the server operator lists it in
/// `[fedcm] confidential_client_ids`. A client's own admin cannot switch it on for themselves.
pub fn validate_fed_cm_client(
    client: &Client,
    confidential_allowed: &[String],
) -> Result<(), ErrorResponse> {
    if client.confidential && !confidential_allowed.iter().any(|id| id == &client.id) {
        return Err(ErrorResponse::new(
            ErrorResponseType::Forbidden,
            "This client is confidential and not allowed to use FedCM. The server operator can \
            allow it with `[fedcm] confidential_client_ids`",
        ));
    }
    Ok(())
}

/// `scheme://host[:port]` of an `http(s)` URI, lowercased, without the default port. `None` for
/// anything else, also for a URI with user info (`https://a.com@evil.com`).
pub fn origin_of_uri(uri: &str) -> Option<String> {
    let (scheme, rest) = uri.split_once("://")?;
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next()?.to_ascii_lowercase();
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    let default_port = if scheme == "https" { ":443" } else { ":80" };
    let authority = authority.strip_suffix(default_port).unwrap_or(&authority);
    Some(format!("{scheme}://{authority}"))
}

/// Whether `uri` is on exactly the origin `origin`. A prefix is not enough: `https://a.co` is a
/// prefix of `https://a.com/client.json`, and `http://localhost:80` of `http://localhost:8099/`.
pub fn same_origin(uri: &str, origin: &str) -> bool {
    match (origin_of_uri(uri), origin_of_uri(origin)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_of_uri_normalizes() {
        assert_eq!(
            origin_of_uri("https://A.com/x?y#z").as_deref(),
            Some("https://a.com")
        );
        assert_eq!(
            origin_of_uri("https://a.com:443/x").as_deref(),
            Some("https://a.com")
        );
        assert_eq!(
            origin_of_uri("http://a.com:80").as_deref(),
            Some("http://a.com")
        );
        assert_eq!(
            origin_of_uri("https://a.com:8443").as_deref(),
            Some("https://a.com:8443")
        );
        assert_eq!(
            origin_of_uri("http://[::1]:80/").as_deref(),
            Some("http://[::1]")
        );
        for bad in [
            "null",
            "",
            "ftp://a.com",
            "myapp://cb",
            "https://",
            "https://u@a.com/",
        ] {
            assert_eq!(origin_of_uri(bad), None, "{bad}");
        }
    }

    #[test]
    fn same_origin_is_not_a_prefix_match() {
        assert!(same_origin("https://a.com/client.json", "https://a.com"));
        assert!(same_origin("https://a.com:443/cb", "https://a.com"));
        assert!(same_origin("HTTPS://A.COM/cb", "https://a.com"));
        // the prefix cases that `starts_with` let through
        assert!(!same_origin("https://a.com/client.json", "https://a.co"));
        assert!(!same_origin(
            "http://localhost:8099/",
            "http://localhost:80"
        ));
        assert!(!same_origin("http://localhost:8099/", "http://localhost"));
        assert!(!same_origin(
            "https://app.example.com.evil.net/cb",
            "https://app.example.com"
        ));
        assert!(!same_origin("https://a.com@evil.com/", "https://a.com"));
        // another scheme or port is another origin
        assert!(!same_origin("http://a.com/", "https://a.com"));
        assert!(!same_origin("https://a.com:8443/", "https://a.com"));
        // `null` (sandboxed pages) and non-web origins never match
        assert!(!same_origin("https://a.com/", "null"));
        assert!(!same_origin("null", "null"));
    }
}
