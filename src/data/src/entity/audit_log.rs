//! Who did what, in a log that is only ever added to.
//!
//! The actor is the id of the user who did it, or [`SYSTEM_USER_ID`] for what the background tasks
//! do on their own. The system user exists as a real user (disabled, without a password or passkey,
//! and `User::save` / `User::delete` refuse to touch it), so it shows up in the user list and
//! nobody can log in as it.

use crate::database::DB;
use chrono::Utc;
use hiqlite::macros::{FromRow, params};
use rauthy_common::is_hiqlite;
use rauthy_common::utils::new_store_id;
use rauthy_derive::FromPgRow;
use rauthy_error::ErrorResponse;
use serde::{Deserialize, Serialize};
use tracing::error;

/// The user that owns what the system does on its own
pub const SYSTEM_USER_ID: &str = "system";

/// Only the longest `details` we keep, so an entry stays small
const MAX_DETAILS: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, FromPgRow)]
pub struct AuditEntry {
    pub id: String,
    /// Unix timestamp in seconds
    pub at: i64,
    pub actor: String,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub details: Option<String>,
}

/// Newest first. Placeholders in ascending order, Hiqlite binds them by their position.
const LIST: &str = "SELECT id, at, actor, action, target_type, target_id, details FROM audit_log \
    WHERE at < $1 ORDER BY at DESC, id DESC LIMIT $2";
const LIST_TARGET: &str = "SELECT id, at, actor, action, target_type, target_id, details \
    FROM audit_log WHERE target_id = $1 AND at < $2 ORDER BY at DESC, id DESC LIMIT $3";

pub struct AuditLog;

impl AuditLog {
    /// Adds an entry
    pub async fn record(
        actor: &str,
        action: &str,
        target_type: &str,
        target_id: Option<&str>,
        details: Option<String>,
    ) -> Result<(), ErrorResponse> {
        let id = new_store_id();
        let at = Utc::now().timestamp();
        let details = details.map(|d| d.chars().take(MAX_DETAILS).collect::<String>());
        let sql = r#"
INSERT INTO audit_log (id, at, actor, action, target_type, target_id, details)
VALUES ($1, $2, $3, $4, $5, $6, $7)"#;
        if is_hiqlite() {
            DB::hql()
                .execute(
                    sql,
                    params!(id, at, actor, action, target_type, target_id, details),
                )
                .await?;
        } else {
            DB::pg_execute(
                sql,
                &[
                    &id,
                    &at,
                    &actor,
                    &action,
                    &target_type,
                    &target_id,
                    &details,
                ],
            )
            .await?;
        }
        Ok(())
    }

    /// Like [`Self::record`], for what has happened already: a failure to write the entry is
    /// logged, it does not undo or fail the action.
    pub async fn record_or_log(
        actor: &str,
        action: &str,
        target_type: &str,
        target_id: Option<&str>,
        details: Option<String>,
    ) {
        if let Err(err) = Self::record(actor, action, target_type, target_id, details).await {
            error!(actor, action, ?err, "Could not write the audit log entry");
        }
    }

    /// The newest entries first, older than `before` (a timestamp in seconds). With `target_id`,
    /// only those about that user, client, ...
    pub async fn find(
        target_id: Option<&str>,
        before: i64,
        limit: i64,
    ) -> Result<Vec<AuditEntry>, ErrorResponse> {
        Ok(match (target_id, is_hiqlite()) {
            (Some(t), true) => {
                DB::hql()
                    .query_map(LIST_TARGET, params!(t, before, limit))
                    .await?
            }
            (Some(t), false) => DB::pg_query(LIST_TARGET, &[&t, &before, &limit], 0).await?,
            (None, true) => DB::hql().query_map(LIST, params!(before, limit)).await?,
            (None, false) => DB::pg_query(LIST, &[&before, &limit], 0).await?,
        })
    }

    pub async fn find_all() -> Result<Vec<AuditEntry>, ErrorResponse> {
        let sql = "SELECT id, at, actor, action, target_type, target_id, details FROM audit_log";
        Ok(if is_hiqlite() {
            DB::hql().query_map(sql, params!()).await?
        } else {
            DB::pg_query(sql, &[], 0).await?
        })
    }
}
