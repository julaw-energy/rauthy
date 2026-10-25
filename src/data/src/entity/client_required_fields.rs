use crate::database::DB;
use crate::entity::user_attr::{UserAttrConfigEntity, UserAttrValueEntity};
use crate::entity::users::User;
use crate::entity::users_values::UserValues;
use hiqlite::macros::{FromRow, params};
use rauthy_common::is_hiqlite;
use rauthy_derive::FromPgRow;
use rauthy_error::{ErrorResponse, ErrorResponseType};
use serde::Deserialize;

/// Built-in user fields a client can require. Custom attributes are `attr:<name>`.
pub const STANDARD_FIELDS: &[&str] = &[
    "given_name",
    "family_name",
    "preferred_username",
    "email_verified",
    "birthdate",
    "gender",
    "tz",
    "phone",
    "street",
    "zip",
    "city",
    "country",
];

/// Raw `client_required_fields` row, used by the DB migration tooling
#[derive(Debug, Deserialize, FromRow, FromPgRow)]
pub struct ClientRequiredFieldsRow {
    pub client_id: String,
    pub fields: String,
}

/// The user fields and attributes a client needs to have a value before a login for it may
/// complete. Users who lack some are sent to their account page first.
pub struct ClientRequiredFields;

impl ClientRequiredFields {
    pub async fn find(client_id: &str) -> Result<Vec<String>, ErrorResponse> {
        let row: Option<ClientRequiredFieldsRow> = if is_hiqlite() {
            DB::hql()
                .query_map_optional(
                    "SELECT client_id, fields FROM client_required_fields WHERE client_id = $1",
                    params!(client_id),
                )
                .await?
        } else {
            DB::pg_query_opt(
                "SELECT client_id, fields FROM client_required_fields WHERE client_id = $1",
                &[&client_id],
            )
            .await?
        };

        Ok(row
            .map(|r| {
                r.fields
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default())
    }

    /// Replaces the list. An empty list removes the row.
    pub async fn save(client_id: &str, fields: Vec<String>) -> Result<Vec<String>, ErrorResponse> {
        let attrs = UserAttrConfigEntity::find_all_as_set().await?;
        let mut clean: Vec<String> = Vec::with_capacity(fields.len());
        for f in fields {
            let f = f.trim().to_string();
            let known = match f.strip_prefix("attr:") {
                Some(name) => attrs.contains(name),
                None => STANDARD_FIELDS.contains(&f.as_str()),
            };
            if !known {
                return Err(ErrorResponse::new(
                    ErrorResponseType::BadRequest,
                    format!("unknown field '{f}'"),
                ));
            }
            if !clean.contains(&f) {
                clean.push(f);
            }
        }

        if clean.is_empty() {
            let sql = "DELETE FROM client_required_fields WHERE client_id = $1";
            if is_hiqlite() {
                DB::hql().execute(sql, params!(client_id)).await?;
            } else {
                DB::pg_execute(sql, &[&client_id]).await?;
            }
        } else {
            let joined = clean.join(",");
            let sql = r#"
INSERT INTO client_required_fields (client_id, fields)
VALUES ($1, $2)
ON CONFLICT (client_id) DO UPDATE SET fields = $2"#;
            if is_hiqlite() {
                DB::hql().execute(sql, params!(client_id, joined)).await?;
            } else {
                DB::pg_execute(sql, &[&client_id, &joined]).await?;
            }
        }

        Ok(clean)
    }

    /// The required fields of `client_id` that `user` has no value for.
    pub async fn missing_for(user: &User, client_id: &str) -> Result<Vec<String>, ErrorResponse> {
        let required = Self::find(client_id).await?;
        if required.is_empty() {
            return Ok(Vec::new());
        }

        let uv = UserValues::find(&user.id).await?.unwrap_or_default();
        let filled = |v: &Option<String>| v.as_deref().is_some_and(|s| !s.trim().is_empty());

        let mut attr_values = None;
        let mut missing = Vec::new();
        for f in required {
            let ok = match f.as_str() {
                "given_name" => !user.given_name.trim().is_empty(),
                "family_name" => user
                    .family_name
                    .as_deref()
                    .is_some_and(|s| !s.trim().is_empty()),
                "preferred_username" => filled(&uv.preferred_username),
                "email_verified" => user.email_verified,
                "birthdate" => filled(&uv.birthdate),
                "gender" => filled(&uv.gender),
                "tz" => filled(&uv.tz),
                "phone" => filled(&uv.phone),
                "street" => filled(&uv.street),
                "zip" => filled(&uv.zip),
                "city" => filled(&uv.city),
                "country" => filled(&uv.country),
                other => match other.strip_prefix("attr:") {
                    Some(name) => {
                        if attr_values.is_none() {
                            attr_values = Some(UserAttrValueEntity::find_for_user(&user.id).await?);
                        }
                        attr_values
                            .as_ref()
                            .and_then(|vals| vals.iter().find(|v| v.key == name))
                            .and_then(|v| {
                                serde_json::from_slice::<serde_json::Value>(&v.value).ok()
                            })
                            .is_some_and(|v| match v {
                                serde_json::Value::Null => false,
                                serde_json::Value::String(s) => !s.trim().is_empty(),
                                serde_json::Value::Array(a) => !a.is_empty(),
                                _ => true,
                            })
                    }
                    None => true,
                },
            };
            if !ok {
                missing.push(f);
            }
        }

        Ok(missing)
    }
}
