use crate::oidc::GrantType;
use rauthy_common::constants::CLIENT_CLAIMS_MAX_LEN;
use rauthy_common::regex::{
    RE_ATTR, RE_CITY, RE_CLIENT_URI, RE_CODE_CHALLENGE_METHOD, RE_CONTACT, RE_GROUPS,
    RE_LINUX_HOSTNAME, RE_ORIGIN, RE_RESOURCE, RE_ROLES_SCOPES,
};
use std::borrow::Cow;
use std::str::FromStr;
use validator::ValidationError;

#[inline]
pub fn validate_vec_attr(value: &[String]) -> Result<(), ValidationError> {
    if value.is_empty() {
        return Err(ValidationError::new(
            "'validate_vec_attr' cannot be empty when provided",
        ));
    }
    for v in value {
        if !RE_ATTR.is_match(v) {
            return Err(ValidationError::new("^[a-z0-9-_/]{2,128}$"));
        }
    }
    Ok(())
}

/// Custom claims attached to a client and emitted into `client_credentials`
/// tokens. Must be a JSON object (it is emitted nested under the `custom` claim)
/// and is capped to keep tokens small, since they are frequently stored in
/// cookies. The cap applies to the client's custom JSON only, not the full token.
#[inline]
pub fn validate_claims(value: &serde_json::Value) -> Result<(), ValidationError> {
    if !value.is_object() {
        return Err(ValidationError::new("`claims` must be a JSON object"));
    }

    let len = serde_json::to_vec(value)
        .map(|v| v.len())
        .unwrap_or(usize::MAX);
    if len > CLIENT_CLAIMS_MAX_LEN {
        return Err(
            ValidationError::new("claims_max_len").with_message(Cow::Owned(format!(
                "`claims` must not exceed {CLIENT_CLAIMS_MAX_LEN} serialized characters"
            ))),
        );
    }

    Ok(())
}

#[inline]
pub fn validate_vec_challenge(value: &[String]) -> Result<(), ValidationError> {
    if value.is_empty() {
        return Err(ValidationError::new(
            "'challenges' cannot be empty when provided",
        ));
    }
    for v in value {
        if !RE_CODE_CHALLENGE_METHOD.is_match(v) {
            return Err(ValidationError::new("^(plain|S256)$"));
        }
    }
    Ok(())
}

#[inline]
pub fn validate_vec_contact(value: &[String]) -> Result<(), ValidationError> {
    for v in value {
        if !RE_CONTACT.is_match(v) {
            return Err(ValidationError::new("^[a-zA-Z0-9\\+.@/-]{0,48}$"));
        }
    }
    Ok(())
}

#[inline]
pub fn validate_vec_linux_hostname(value: &[String]) -> Result<(), ValidationError> {
    for v in value {
        if !RE_LINUX_HOSTNAME.is_match(v) {
            return Err(ValidationError::new(
                "^[a-zA-Z0-9][a-zA-Z0-9-.]*[a-zA-Z0-9]$",
            ));
        }
    }
    Ok(())
}

#[inline]
pub fn validate_vec_origin(value: &[String]) -> Result<(), ValidationError> {
    for v in value {
        if !RE_ORIGIN.get().unwrap().is_match(v) {
            return Err(ValidationError::new("^(http|https)://[a-z0-9.:-]+$"));
        }
    }
    Ok(())
}

#[inline]
pub fn validate_vec_uri(value: &[String]) -> Result<(), ValidationError> {
    for v in value {
        if !RE_CLIENT_URI.is_match(v) {
            return Err(ValidationError::new(
                "valid URI with a non-empty host part, without # or ,",
            ));
        }
    }
    Ok(())
}

/// RFC 8707 resource indicators (client `allowed_resources` / `default_aud`). Same as a URI
/// but a fragment (`#`) is not allowed, since a resource indicator MUST be an absolute URI
/// without a fragment (RFC 8707 §2). Matches the request-side `resource` validation.
#[inline]
pub fn validate_vec_resource(value: &[String]) -> Result<(), ValidationError> {
    for v in value {
        if !RE_RESOURCE.is_match(v) {
            return Err(ValidationError::new("^[a-zA-Z0-9,.:/_\\-&?=~!$'()*+%@]+$"));
        }
    }
    Ok(())
}

/// Grant-type validation for ephemeral (CIMD) client documents. Strict by default: an
/// advertised grant type Rauthy does not support is rejected. An operator can opt into
/// accepting such a document by enabling `ephemeral_clients.ignore_unknown_auth_flows`,
/// which strips the unknown grant types in `Client::ephemeral_from_url` *before* this
/// validation runs, so the sanitized list passes here.
///
/// This is the one grant-type field that stays a `Vec<String>` rather than a `Vec<GrantType>`:
/// the document is deserialized from a remote source, so an unknown grant type has to survive
/// deserialization long enough to be stripped. Making it an enum would reject the document at
/// `serde` level and defeat `ignore_unknown_auth_flows` entirely. `GrantType` is still the
/// single source of truth for what "supported" means.
#[inline]
pub fn validate_vec_grant_type(value: &[String]) -> Result<(), ValidationError> {
    for v in value {
        if GrantType::from_str(v).is_err() {
            return Err(ValidationError::new(
                "^(authorization_code|client_credentials|urn:ietf:params:oauth:grant-type:device_code|urn:ietf:params:oauth:grant-type:token-exchange|password|refresh_token)$",
            ));
        }
    }
    Ok(())
}

// validate_vec_groups, _roles and _scopes do the same thing but are 3 functions just to
// be clear in the validation fields above that it does not create confusion, even if they
// all use the same `RE_GROUPS` regex.
#[inline]
pub fn validate_vec_groups(value: &[String]) -> Result<(), ValidationError> {
    for v in value {
        if !RE_GROUPS.is_match(v) {
            return Err(ValidationError::new("^[a-zA-Z0-9-_/,:*\\s]{2,64}$"));
        }
    }
    Ok(())
}

#[inline]
pub fn validate_vec_roles(value: &[String]) -> Result<(), ValidationError> {
    for v in value {
        if !RE_ROLES_SCOPES.is_match(v) {
            return Err(ValidationError::new("^[a-zA-Z0-9-_/,:*.]{2,64}$"));
        }
    }
    Ok(())
}

#[inline]
pub fn validate_vec_scopes(value: &[String]) -> Result<(), ValidationError> {
    for v in value {
        if !RE_ROLES_SCOPES.is_match(v) {
            return Err(ValidationError::new("^[a-zA-Z0-9-_/,:*.]{2,64}$"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn client_claims_must_be_an_object() {
        assert!(validate_claims(&json!({"tenant": "acme", "tier": 3})).is_ok());
        assert!(validate_claims(&json!({})).is_ok());

        // anything that is not a JSON object is rejected
        assert!(validate_claims(&json!("a string")).is_err());
        assert!(validate_claims(&json!(42)).is_err());
        assert!(validate_claims(&json!(true)).is_err());
        assert!(validate_claims(&json!(["a", "b"])).is_err());
        assert!(validate_claims(&serde_json::Value::Null).is_err());
    }

    #[test]
    fn grant_type_validator_rejects_unknown_by_default() {
        // A spec-valid client (e.g. claude.ai) may advertise grant types Rauthy does not
        // implement. The ephemeral path rejects them by default and only accepts such a
        // document once they are stripped upstream, gated by
        // `ephemeral_clients.ignore_unknown_auth_flows`.
        //
        // The admin / DCR / bootstrap side no longer needs a validator at all: those fields
        // are `Vec<GrantType>`, so `serde` rejects an unknown grant type at deserialization.
        // This is the one path where that cannot work, because the document is remote and the
        // unknown value has to survive long enough to be stripped.
        let with_unknown = [
            "authorization_code",
            "refresh_token",
            "urn:ietf:params:oauth:grant-type:jwt-bearer",
        ]
        .map(String::from);
        assert!(validate_vec_grant_type(&with_unknown).is_err());

        // An all-supported list passes, including the extended-grant URNs.
        let all_known = [
            "authorization_code",
            "refresh_token",
            "urn:ietf:params:oauth:grant-type:device_code",
            "urn:ietf:params:oauth:grant-type:token-exchange",
        ]
        .map(String::from);
        assert!(validate_vec_grant_type(&all_known).is_ok());

        // No empty-check here: an ephemeral document may legitimately omit `grant_types`.
        let empty: [String; 0] = [];
        assert!(validate_vec_grant_type(&empty).is_ok());
    }

    #[test]
    fn grant_type_validator_tracks_the_enum() {
        // `GrantType` is the single source of truth. If a variant is ever added, it must pass
        // the ephemeral validator too, otherwise the two drift apart silently.
        for gt in [
            GrantType::AuthorizationCode,
            GrantType::ClientCredentials,
            GrantType::Password,
            GrantType::RefreshToken,
            GrantType::DeviceCode,
            GrantType::TokenExchange,
        ] {
            assert!(validate_vec_grant_type(&[gt.to_string()]).is_ok(), "{gt}");
        }
    }

    #[test]
    fn client_claims_respect_the_size_cap() {
        // a value just under the 1024 serialized-byte cap is accepted
        let ok = json!({ "k": "a".repeat(1000) });
        assert!(serde_json::to_vec(&ok).unwrap().len() <= 1024);
        assert!(validate_claims(&ok).is_ok());

        // a value over the cap is rejected
        let too_big = json!({ "k": "a".repeat(1024) });
        assert!(serde_json::to_vec(&too_big).unwrap().len() > 1024);
        assert!(validate_claims(&too_big).is_err());
    }
}

/// ISO 3166-1 alpha-2 codes plus the few extra ones `libphonenumber` offers (`AC`, `TA`, `XK`).
/// Sorted for `binary_search`.
const COUNTRY_CODES: &[&str] = &[
    "AC", "AD", "AE", "AF", "AG", "AI", "AL", "AM", "AO", "AQ", "AR", "AS", "AT", "AU", "AW", "AX",
    "AZ", "BA", "BB", "BD", "BE", "BF", "BG", "BH", "BI", "BJ", "BL", "BM", "BN", "BO", "BQ", "BR",
    "BS", "BT", "BV", "BW", "BY", "BZ", "CA", "CC", "CD", "CF", "CG", "CH", "CI", "CK", "CL", "CM",
    "CN", "CO", "CR", "CU", "CV", "CW", "CX", "CY", "CZ", "DE", "DJ", "DK", "DM", "DO", "DZ", "EC",
    "EE", "EG", "EH", "ER", "ES", "ET", "FI", "FJ", "FK", "FM", "FO", "FR", "GA", "GB", "GD", "GE",
    "GF", "GG", "GH", "GI", "GL", "GM", "GN", "GP", "GQ", "GR", "GS", "GT", "GU", "GW", "GY", "HK",
    "HM", "HN", "HR", "HT", "HU", "ID", "IE", "IL", "IM", "IN", "IO", "IQ", "IR", "IS", "IT", "JE",
    "JM", "JO", "JP", "KE", "KG", "KH", "KI", "KM", "KN", "KP", "KR", "KW", "KY", "KZ", "LA", "LB",
    "LC", "LI", "LK", "LR", "LS", "LT", "LU", "LV", "LY", "MA", "MC", "MD", "ME", "MF", "MG", "MH",
    "MK", "ML", "MM", "MN", "MO", "MP", "MQ", "MR", "MS", "MT", "MU", "MV", "MW", "MX", "MY", "MZ",
    "NA", "NC", "NE", "NF", "NG", "NI", "NL", "NO", "NP", "NR", "NU", "NZ", "OM", "PA", "PE", "PF",
    "PG", "PH", "PK", "PL", "PM", "PN", "PR", "PS", "PT", "PW", "PY", "QA", "RE", "RO", "RS", "RU",
    "RW", "SA", "SB", "SC", "SD", "SE", "SG", "SH", "SI", "SJ", "SK", "SL", "SM", "SN", "SO", "SR",
    "SS", "ST", "SV", "SX", "SY", "SZ", "TA", "TC", "TD", "TF", "TG", "TH", "TJ", "TK", "TL", "TM",
    "TN", "TO", "TR", "TT", "TV", "TW", "TZ", "UA", "UG", "UM", "US", "UY", "UZ", "VA", "VC", "VE",
    "VG", "VI", "VN", "VU", "WF", "WS", "XK", "YE", "YT", "ZA", "ZM", "ZW",
];

/// A country is stored as its ISO 3166-1 alpha-2 code. Values that are not exactly two letters
/// are still accepted as free text (legacy data from before codes were used).
pub fn validate_country(value: &str) -> Result<(), ValidationError> {
    if value.len() == 2 {
        if COUNTRY_CODES.binary_search(&value).is_ok() {
            return Ok(());
        }
        return Err(ValidationError::new(
            "unknown ISO 3166-1 alpha-2 country code",
        ));
    }
    if value.chars().count() > 48 || !RE_CITY.is_match(value) {
        return Err(ValidationError::new("[a-zA-Z0-9À-ÿ-\\s]{0,48}"));
    }
    Ok(())
}

/// `YYYY-MM-DD`, a real calendar date, not before 1900 and not in the future.
pub fn validate_birthdate(value: &str) -> Result<(), ValidationError> {
    let err = || ValidationError::new("birthdate must be a real date between 1900-01-01 and today");
    let date = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| err())?;
    let min = chrono::NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();
    if date < min || date > chrono::Utc::now().date_naive() {
        return Err(err());
    }
    Ok(())
}

#[cfg(test)]
mod tests_user_values {
    use super::*;

    #[test]
    fn country_codes_sorted_and_complete() {
        assert!(COUNTRY_CODES.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(COUNTRY_CODES.len(), 249 + 3);
        assert!(validate_country("UG").is_ok());
        assert!(validate_country("ZZ").is_err());
        assert!(validate_country("uG").is_err());
        assert!(validate_country("Uganda").is_ok());
    }

    #[test]
    fn birthdate() {
        assert!(validate_birthdate("1999-08-16").is_ok());
        assert!(validate_birthdate("2024-02-29").is_ok());
        assert!(validate_birthdate("2025-02-29").is_err());
        assert!(validate_birthdate("2025-13-45").is_err());
        assert!(validate_birthdate("1899-12-31").is_err());
        assert!(validate_birthdate("2999-01-01").is_err());
    }
}

/// A user-defined pattern for a custom user attribute
pub fn validate_attr_regex(value: &str) -> Result<(), ValidationError> {
    if value.len() > 256 {
        return Err(ValidationError::new("regex: max 256 characters"));
    }
    if regex::RegexBuilder::new(value)
        .size_limit(100_000)
        .build()
        .is_err()
    {
        return Err(ValidationError::new("regex: invalid expression"));
    }
    Ok(())
}

pub fn validate_attr_options(value: &[String]) -> Result<(), ValidationError> {
    if value.len() > 64 || value.iter().any(|v| v.is_empty() || v.chars().count() > 64) {
        return Err(ValidationError::new(
            "options: max 64 entries, 1-64 characters each",
        ));
    }
    Ok(())
}
