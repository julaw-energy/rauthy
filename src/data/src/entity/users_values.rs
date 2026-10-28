use crate::database::{Cache, DB};
use hiqlite::macros::{FromRow, params};
use rauthy_api_types::users::{UserValuesRequest, UserValuesResponse};
use rauthy_common::constants::{CACHE_TTL_USER, IDX_USERS_VALUES};
use rauthy_common::is_hiqlite;
use rauthy_derive::FromPgRow;
use rauthy_error::{ErrorResponse, ErrorResponseType};
use serde::{Deserialize, Serialize};

/// The shortest username there is
pub const PREFERRED_USERNAME_MIN: usize = 4;

#[derive(Debug, Default, Clone, Serialize, Deserialize, FromRow, FromPgRow)]
pub struct UserValues {
    pub id: String,
    pub birthdate: Option<String>,
    pub gender: Option<String>,
    pub phone: Option<String>,
    pub street: Option<String>,
    pub zip: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub preferred_username: Option<String>,
    pub tz: Option<String>,
}

impl UserValues {
    #[inline(always)]
    fn cache_idx(user_id: &str) -> String {
        format!("{IDX_USERS_VALUES}_{user_id}")
    }

    /// A username has at least 4 characters and does not start with a number: names like `101` are
    /// for system users. The configured pattern (`user_values.preferred_username`) is checked in
    /// addition, this is what no configuration can allow.
    pub fn check_preferred_username(name: &str) -> Result<(), ErrorResponse> {
        if name.chars().count() < PREFERRED_USERNAME_MIN {
            return Err(ErrorResponse::new(
                ErrorResponseType::BadRequest,
                format!("The username needs at least {PREFERRED_USERNAME_MIN} characters"),
            ));
        }
        if name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return Err(ErrorResponse::new(
                ErrorResponseType::BadRequest,
                "The username must not start with a number, those are reserved for system users",
            ));
        }
        Ok(())
    }

    /// A field the admin hid is not shown to the user, so the page cannot send it. A self-update must
    /// keep what is stored for it instead of clearing it, whatever the request says.
    pub fn keep_hidden(
        req: &mut UserValuesRequest,
        current: Option<&UserValues>,
        cfg: &crate::settings::UserValuesEffective,
    ) {
        use crate::rauthy_config::UserValueConfigValue::Hidden;
        macro_rules! keep {
            ($($f:ident),*) => {$(
                if cfg.$f == Hidden {
                    req.$f = current.and_then(|c| c.$f.clone());
                }
            )*};
        }
        keep!(birthdate, gender, phone, street, zip, city, country, tz);
    }

    /// CAUTION: Does also set the `preferred_username`. This should only be used with open registration.
    pub async fn insert(
        user_id: String,
        values: UserValuesRequest,
        preferred_username: Option<String>,
    ) -> Result<(), ErrorResponse> {
        if let Some(name) = &preferred_username {
            Self::check_preferred_username(name)?;
        }
        let sql = r#"
INSERT INTO
users_values (id, birthdate, gender, phone, street, zip, city, country, preferred_username, tz)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"#;

        if is_hiqlite() {
            DB::hql()
                .execute(
                    sql,
                    params!(
                        user_id,
                        values.birthdate,
                        values.gender,
                        values.phone,
                        values.street,
                        values.zip,
                        values.city,
                        values.country,
                        preferred_username,
                        values.tz
                    ),
                )
                .await
                .map_err(|err| {
                    let err = ErrorResponse::from(err);
                    if err.message.contains("UNIQUE") {
                        ErrorResponse::new(
                            ErrorResponseType::NotAccepted,
                            "UNIQUE constraint on: 'preferred_username'",
                        )
                    } else {
                        err
                    }
                })?
        } else {
            DB::pg_execute(
                sql,
                &[
                    &user_id,
                    &values.birthdate,
                    &values.gender,
                    &values.phone,
                    &values.street,
                    &values.zip,
                    &values.city,
                    &values.country,
                    &preferred_username,
                    &values.tz,
                ],
            )
            .await
            .map_err(|err| {
                if err.message.contains("UNIQUE") {
                    ErrorResponse::new(
                        ErrorResponseType::NotAccepted,
                        "UNIQUE constraint on: 'preferred_username'",
                    )
                } else {
                    err
                }
            })?
        };

        Ok(())
    }

    pub async fn find(user_id: &str) -> Result<Option<Self>, ErrorResponse> {
        let idx = Self::cache_idx(user_id);
        let client = DB::hql();

        let opt: Option<Option<Self>> = client.get(Cache::User, &idx).await?;
        if let Some(slf) = opt {
            return Ok(slf);
        }

        let sql = "SELECT * FROM users_values WHERE id = $1";
        let slf = if is_hiqlite() {
            client.query_map_optional(sql, params!(user_id)).await?
        } else {
            DB::pg_query_opt(sql, &[&user_id]).await?
        };

        client.put(Cache::User, idx, &slf, CACHE_TTL_USER).await?;

        Ok(slf)
    }

    pub async fn find_preferred_username(user_id: &str) -> Result<Option<String>, ErrorResponse> {
        let idx = Self::cache_idx(user_id);
        let client = DB::hql();

        let opt: Option<Option<Self>> = client.get(Cache::User, &idx).await?;
        if let Some(slf) = opt {
            return Ok(slf.and_then(|s| s.preferred_username));
        }

        let sql = "SELECT preferred_username FROM users_values WHERE id = $1";
        let slf = if is_hiqlite() {
            let mut rows = client.query_raw(sql, params!(user_id)).await?;
            if rows.is_empty() {
                None
            } else {
                rows[0].try_get("preferred_username").ok()
            }
        } else {
            let rows = DB::pg_query_rows(sql, &[&user_id], 0).await?;
            if rows.is_empty() {
                None
            } else {
                rows[0].try_get("preferred_username").ok()
            }
        };

        Ok(slf)
    }

    /// CAUTION: Does NOT update the `preferred_username`, which has dedicated functions for
    /// additional validation.
    pub async fn upsert(
        user_id: String,
        values: UserValuesRequest,
    ) -> Result<Option<Self>, ErrorResponse> {
        crate::entity::audit_log::refuse_system_user_id(&user_id)?;
        let idx = Self::cache_idx(&user_id);
        let sql = r#"
INSERT INTO
users_values (id, birthdate, gender, phone, street, zip, city, country, tz)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
ON CONFLICT(id) DO UPDATE
SET birthdate = $2, gender = $3, phone = $4, street = $5, zip = $6, city = $7, country = $8, tz = $9
RETURNING *"#;

        let values: Self = if is_hiqlite() {
            DB::hql()
                .execute_returning_map_one(
                    sql,
                    params!(
                        user_id,
                        values.birthdate,
                        values.gender,
                        values.phone,
                        values.street,
                        values.zip,
                        values.city,
                        values.country,
                        values.tz
                    ),
                )
                .await?
        } else {
            DB::pg_query_one(
                sql,
                &[
                    &user_id,
                    &values.birthdate,
                    &values.gender,
                    &values.phone,
                    &values.street,
                    &values.zip,
                    &values.city,
                    &values.country,
                    &values.tz,
                ],
            )
            .await?
        };

        let slf = Some(values);
        DB::hql()
            .put(Cache::User, idx, &slf, CACHE_TTL_USER)
            .await?;

        Ok(slf)
    }

    pub async fn upsert_preferred_username(
        user_id: String,
        preferred_username: String,
    ) -> Result<Option<Self>, ErrorResponse> {
        crate::entity::audit_log::refuse_system_user_id(&user_id)?;
        // a name that was accepted before may be saved again, only a new one is held to the rule
        if Self::find_preferred_username(&user_id).await?.as_deref() != Some(&preferred_username) {
            Self::check_preferred_username(&preferred_username)?;
        }
        let idx = Self::cache_idx(&user_id);
        let sql = r#"
INSERT INTO
users_values (id, preferred_username)
VALUES ($1, $2)
ON CONFLICT(id) DO UPDATE
SET preferred_username = $2
RETURNING *"#;

        let values: Self = if is_hiqlite() {
            DB::hql()
                .execute_returning_map_one(sql, params!(user_id, preferred_username))
                .await
                .map_err(|err| {
                    let err = ErrorResponse::from(err);
                    if err.message.contains("UNIQUE") {
                        ErrorResponse::new(
                            ErrorResponseType::NotAccepted,
                            "'preferred_username' is not available",
                        )
                    } else {
                        err
                    }
                })?
        } else {
            DB::pg_query_one(sql, &[&user_id, &preferred_username])
                .await
                .map_err(|err| {
                    if err.message.contains("UNIQUE") {
                        ErrorResponse::new(
                            ErrorResponseType::NotAccepted,
                            "'preferred_username' is not available",
                        )
                    } else {
                        err
                    }
                })?
        };

        let slf = Some(values);
        DB::hql()
            .put(Cache::User, idx, &slf, CACHE_TTL_USER)
            .await?;

        Ok(slf)
    }

    pub async fn delete(user_id: String) -> Result<(), ErrorResponse> {
        let cache_idx = Self::cache_idx(&user_id);
        let sql = "DELETE FROM users_values WHERE id = $1";

        if is_hiqlite() {
            DB::hql().execute(sql, params!(user_id)).await?;
        } else {
            DB::pg_execute(sql, &[&user_id]).await?;
        }

        DB::hql().delete(Cache::User, cache_idx).await?;

        Ok(())
    }

    pub async fn delete_preferred_username(user_id: String) -> Result<(), ErrorResponse> {
        crate::entity::audit_log::refuse_system_user_id(&user_id)?;
        let idx = Self::cache_idx(&user_id);
        let sql = r#"
UPDATE users_values
SET preferred_username = null
WHERE id = $1
RETURNING *"#;

        let values: Self = if is_hiqlite() {
            DB::hql()
                .execute_returning_map_one(sql, params!(user_id))
                .await?
        } else {
            DB::pg_query_one(sql, &[&user_id]).await?
        };

        DB::hql()
            .put(Cache::User, idx, &Some(values), CACHE_TTL_USER)
            .await?;

        Ok(())
    }

    pub async fn validate_preferred_username_free(
        preferred_username: String,
    ) -> Result<(), ErrorResponse> {
        let sql = "SELECT 1 FROM users_values WHERE preferred_username = $1";

        let is_free = if is_hiqlite() {
            DB::hql()
                .query_raw_one(sql, params!(preferred_username))
                .await
                .is_err()
        } else {
            DB::pg_query_one_row(sql, &[&preferred_username])
                .await
                .is_err()
        };

        if is_free {
            Ok(())
        } else {
            Err(ErrorResponse::new(
                ErrorResponseType::NotAccepted,
                "UNIQUE constraint on: 'preferred_username'",
            ))
        }
    }
}

impl From<UserValues> for UserValuesResponse {
    fn from(value: UserValues) -> Self {
        Self {
            birthdate: value.birthdate,
            gender: value.gender,
            phone: value.phone,
            street: value.street,
            zip: value.zip,
            city: value.city,
            country: value.country,
            preferred_username: value.preferred_username,
            tz: value.tz,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_fields_keep_what_is_stored() {
        use crate::rauthy_config::UserValueConfigValue::{Hidden, Optional};
        let cfg = crate::settings::UserValuesEffective {
            given_name: Optional,
            family_name: Optional,
            birthdate: Optional,
            gender: Optional,
            street: Optional,
            zip: Optional,
            city: Optional,
            country: Optional,
            phone: Optional,
            tz: Hidden,
            revalidate_during_login: false,
            name_order_reverse: false,
        };
        let current = UserValues {
            tz: Some("Africa/Kampala".into()),
            city: Some("Old".into()),
            ..Default::default()
        };
        // the page sends no tz (hidden) and a new city (visible)
        let mut req = UserValuesRequest {
            city: Some("New".into()),
            ..Default::default()
        };
        UserValues::keep_hidden(&mut req, Some(&current), &cfg);
        assert_eq!(req.tz.as_deref(), Some("Africa/Kampala"), "hidden: kept");
        assert_eq!(
            req.city.as_deref(),
            Some("New"),
            "visible: the request wins"
        );
        // a request that tries to set a hidden field does not get through either
        let mut req = UserValuesRequest {
            tz: Some("UTC".into()),
            ..Default::default()
        };
        UserValues::keep_hidden(&mut req, Some(&current), &cfg);
        assert_eq!(req.tz.as_deref(), Some("Africa/Kampala"));
        // nothing stored, nothing to keep
        let mut req = UserValuesRequest {
            tz: Some("UTC".into()),
            ..Default::default()
        };
        UserValues::keep_hidden(&mut req, None, &cfg);
        assert_eq!(req.tz, None);
    }

    #[test]
    fn usernames_have_four_characters_and_no_leading_number() {
        for ok in ["abcd", "batman", "a123", "a_b-c", "jörg"] {
            assert!(UserValues::check_preferred_username(ok).is_ok(), "{ok}");
        }
        for bad in ["", "abc", "ab", "101", "1abc", "9lives", "0000"] {
            assert!(UserValues::check_preferred_username(bad).is_err(), "{bad}");
        }
    }
}
