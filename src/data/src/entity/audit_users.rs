//! The few details of a deleted user that audit entries need to name them.
//!
//! Audit entries only hold user ids, without foreign keys, so they stay when a user is deleted.
//! When `User::delete` runs, the id, email, names, username and phone are copied here first. An
//! admin can wipe the personal data of such a row on request ([`AuditUser::anonymize`]); the id
//! stays, so entries are still tied together.

use crate::database::DB;
use crate::entity::users::User;
use crate::entity::users_values::UserValues;
use chrono::Utc;
use hiqlite::macros::{FromRow, params};
use rauthy_common::is_hiqlite;
use rauthy_derive::FromPgRow;
use rauthy_error::ErrorResponse;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, FromPgRow)]
pub struct AuditUser {
    pub id: String,
    pub email: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub preferred_username: Option<String>,
    pub phone: Option<String>,
    /// Unix timestamp in seconds
    pub deleted_at: i64,
    pub anonymized: bool,
}

const COLS: &str =
    "id, email, given_name, family_name, preferred_username, phone, deleted_at, anonymized";

impl AuditUser {
    /// Copies what audit entries need before the user is deleted
    pub async fn remember(user: &User) -> Result<(), ErrorResponse> {
        let uv = UserValues::find(&user.id).await?.unwrap_or_default();
        let now = Utc::now().timestamp();
        let sql = r#"
INSERT INTO audit_users (id, email, given_name, family_name, preferred_username, phone, deleted_at, anonymized)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
ON CONFLICT (id) DO UPDATE SET email = excluded.email, given_name = excluded.given_name,
    family_name = excluded.family_name, preferred_username = excluded.preferred_username,
    phone = excluded.phone, deleted_at = excluded.deleted_at, anonymized = excluded.anonymized"#;
        if is_hiqlite() {
            DB::hql()
                .execute(
                    sql,
                    params!(
                        &user.id,
                        &user.email,
                        &user.given_name,
                        &user.family_name,
                        uv.preferred_username,
                        uv.phone,
                        now,
                        false
                    ),
                )
                .await?;
        } else {
            DB::pg_execute(
                sql,
                &[
                    &user.id,
                    &user.email,
                    &user.given_name,
                    &user.family_name,
                    &uv.preferred_username,
                    &uv.phone,
                    &now,
                    &false,
                ],
            )
            .await?;
        }
        Ok(())
    }

    pub async fn find(id: &str) -> Result<Option<Self>, ErrorResponse> {
        let sql = format!("SELECT {COLS} FROM audit_users WHERE id = $1");
        Ok(if is_hiqlite() {
            DB::hql().query_map_optional(sql, params!(id)).await?
        } else {
            DB::pg_query_opt(&sql, &[&id]).await?
        })
    }

    /// Wipes the personal data of a deleted user. Returns `false` if there is no such row.
    pub async fn anonymize(id: &str) -> Result<bool, ErrorResponse> {
        if Self::find(id).await?.is_none() {
            return Ok(false);
        }
        let sql = r#"
UPDATE audit_users
SET email = NULL, given_name = NULL, family_name = NULL, preferred_username = NULL, phone = NULL,
    anonymized = $1
WHERE id = $2"#;
        if is_hiqlite() {
            DB::hql().execute(sql, params!(true, id)).await?;
        } else {
            DB::pg_execute(sql, &[&true, &id]).await?;
        }
        Ok(true)
    }

    /// The deleted users, the most recently deleted first
    pub async fn find_recent(limit: i64) -> Result<Vec<Self>, ErrorResponse> {
        let sql = format!("SELECT {COLS} FROM audit_users ORDER BY deleted_at DESC, id LIMIT $1");
        Ok(if is_hiqlite() {
            DB::hql().query_map(sql, params!(limit)).await?
        } else {
            DB::pg_query(&sql, &[&limit], 0).await?
        })
    }

    pub async fn find_all() -> Result<Vec<Self>, ErrorResponse> {
        let sql = format!("SELECT {COLS} FROM audit_users");
        Ok(if is_hiqlite() {
            DB::hql().query_map(sql, params!()).await?
        } else {
            DB::pg_query(&sql, &[], 0).await?
        })
    }
}
