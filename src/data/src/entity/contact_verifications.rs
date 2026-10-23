use crate::database::DB;
use chrono::Utc;
use hiqlite::macros::{FromRow, params};
use rauthy_common::is_hiqlite;
use rauthy_common::utils::get_rand_between;
use rauthy_derive::FromPgRow;
use rauthy_error::{ErrorResponse, ErrorResponseType};
use serde::Deserialize;

/// Seconds a code stays valid
const CODE_TTL: i64 = 10 * 60;
/// Wrong guesses allowed per code
const MAX_ATTEMPTS: i64 = 5;
/// Minimum seconds between two codes for the same user and kind
const RESEND_COOLDOWN: i64 = 60;

/// Verification state of one contact value of a user.
///
/// `kind` is `email` / `phone` for the user's own values, and `attr:<name>` for a custom
/// attribute of type `verified_email` / `verified_phone`.
///
/// `value` is what a pending code was sent to / what has been verified. A verification therefore
/// only counts while the user's current value is still the same one.
#[derive(Debug, Deserialize, FromRow, FromPgRow)]
pub struct ContactVerification {
    pub user_id: String,
    pub kind: String,
    pub value: String,
    pub verified_at: Option<i64>,
    pub code_hash: Option<String>,
    pub code_exp: Option<i64>,
    pub attempts: i64,
    pub last_sent: i64,
}

fn hash_code(user_id: &str, kind: &str, code: &str) -> String {
    hex::encode(hmac_sha256::Hash::hash(
        format!("{user_id}:{kind}:{code}").as_bytes(),
    ))
}

impl ContactVerification {
    async fn find(user_id: &str, kind: &str) -> Result<Option<Self>, ErrorResponse> {
        let sql = "SELECT * FROM contact_verifications WHERE user_id = $1 AND kind = $2";

        let res = if is_hiqlite() {
            DB::hql()
                .query_map_optional(sql, params!(user_id, kind))
                .await?
        } else {
            DB::pg_query_opt(sql, &[&user_id, &kind]).await?
        };
        Ok(res)
    }

    /// `true` if `value` is the one that has been verified for this user and kind.
    pub async fn is_verified(
        user_id: &str,
        kind: &str,
        value: &str,
    ) -> Result<bool, ErrorResponse> {
        Ok(Self::find(user_id, kind)
            .await?
            .is_some_and(|v| v.verified_at.is_some() && v.value == value))
    }

    /// Creates a fresh code for `value` and returns it in plain text so it can be sent out.
    /// Only the hash is stored.
    pub async fn start(user_id: &str, kind: &str, value: &str) -> Result<String, ErrorResponse> {
        let now = Utc::now().timestamp();

        if let Some(existing) = Self::find(user_id, kind).await? {
            if existing.value == value && existing.verified_at.is_some() {
                return Err(ErrorResponse::new(
                    ErrorResponseType::BadRequest,
                    "This value is already verified",
                ));
            }
            let next = existing.last_sent + RESEND_COOLDOWN;
            if existing.value == value && next > now {
                return Err(ErrorResponse::new(
                    ErrorResponseType::TooManyRequests(next),
                    "Please wait a moment before requesting another code",
                ));
            }
        }

        let code = format!("{:06}", get_rand_between(0, 999_999));
        let hash = hash_code(user_id, kind, &code);
        let exp = now + CODE_TTL;
        let kind_s = kind;

        let sql = r#"
INSERT INTO contact_verifications (user_id, kind, value, verified_at, code_hash, code_exp, attempts, last_sent)
VALUES ($1, $2, $3, NULL, $4, $5, 0, $6)
ON CONFLICT (user_id, kind)
DO UPDATE SET value = $3, verified_at = NULL, code_hash = $4, code_exp = $5, attempts = 0, last_sent = $6"#;

        if is_hiqlite() {
            DB::hql()
                .execute(sql, params!(user_id, kind_s, value, hash, exp, now))
                .await?;
        } else {
            DB::pg_execute(sql, &[&user_id, &kind_s, &value, &hash, &exp, &now]).await?;
        }

        Ok(code)
    }

    /// Checks `code` against the pending one for `value`. Every wrong guess is counted, after
    /// [`MAX_ATTEMPTS`] of them the code is dead and a new one must be requested.
    pub async fn confirm(
        user_id: &str,
        kind: &str,
        value: &str,
        code: &str,
    ) -> Result<(), ErrorResponse> {
        let invalid = || {
            ErrorResponse::new(
                ErrorResponseType::BadRequest,
                "Invalid or expired code, request a new one",
            )
        };

        let Some(row) = Self::find(user_id, kind).await? else {
            return Err(invalid());
        };
        let now = Utc::now().timestamp();
        let (Some(hash), Some(exp)) = (&row.code_hash, row.code_exp) else {
            return Err(invalid());
        };
        if row.value != value || exp < now || row.attempts >= MAX_ATTEMPTS {
            return Err(invalid());
        }

        let kind_s = kind;
        let ok = constant_time_eq::constant_time_eq(
            hash.as_bytes(),
            hash_code(user_id, kind, code).as_bytes(),
        );

        if ok {
            let sql = r#"
UPDATE contact_verifications
SET verified_at = $1, code_hash = NULL, code_exp = NULL, attempts = 0
WHERE user_id = $2 AND kind = $3"#;
            // keep the placeholders in order of appearance: hiqlite binds them positionally
            if is_hiqlite() {
                DB::hql()
                    .execute(sql, params!(now, user_id, kind_s))
                    .await?;
            } else {
                DB::pg_execute(sql, &[&now, &user_id, &kind_s]).await?;
            }
            Ok(())
        } else {
            let sql = r#"
UPDATE contact_verifications SET attempts = attempts + 1
WHERE user_id = $1 AND kind = $2"#;
            if is_hiqlite() {
                DB::hql().execute(sql, params!(user_id, kind_s)).await?;
            } else {
                DB::pg_execute(sql, &[&user_id, &kind_s]).await?;
            }
            Err(invalid())
        }
    }

    /// Admin override: mark `value` as verified (or drop the verification).
    pub async fn set_verified(
        user_id: &str,
        kind: &str,
        value: &str,
        verified: bool,
    ) -> Result<(), ErrorResponse> {
        let kind_s = kind;
        if verified {
            let now = Utc::now().timestamp();
            let sql = r#"
INSERT INTO contact_verifications (user_id, kind, value, verified_at, attempts, last_sent)
VALUES ($1, $2, $3, $4, 0, 0)
ON CONFLICT (user_id, kind)
DO UPDATE SET value = $3, verified_at = $4, code_hash = NULL, code_exp = NULL, attempts = 0"#;
            if is_hiqlite() {
                DB::hql()
                    .execute(sql, params!(user_id, kind_s, value, now))
                    .await?;
            } else {
                DB::pg_execute(sql, &[&user_id, &kind_s, &value, &now]).await?;
            }
        } else {
            let sql = "DELETE FROM contact_verifications WHERE user_id = $1 AND kind = $2";
            if is_hiqlite() {
                DB::hql().execute(sql, params!(user_id, kind_s)).await?;
            } else {
                DB::pg_execute(sql, &[&user_id, &kind_s]).await?;
            }
        }
        Ok(())
    }

    pub async fn find_all() -> Result<Vec<Self>, ErrorResponse> {
        let sql = "SELECT * FROM contact_verifications";
        let res = if is_hiqlite() {
            DB::hql().query_map(sql, params!()).await?
        } else {
            DB::pg_query(sql, &[], 0).await?
        };
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_hash_is_bound_to_user_and_kind() {
        let a = hash_code("u1", "phone", "123456");
        assert_eq!(a, hash_code("u1", "phone", "123456"));
        assert_ne!(a, hash_code("u2", "phone", "123456"));
        assert_ne!(a, hash_code("u1", "email", "123456"));
        assert_ne!(a, hash_code("u1", "phone", "123457"));
    }
}
