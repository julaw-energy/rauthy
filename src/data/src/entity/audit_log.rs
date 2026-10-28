//! Who did what, in a log that is only ever added to.
//!
//! The actor is the id of the user who did it, or [`SYSTEM_USER_ID`] for what the background tasks
//! do on their own. The system user exists as a real user (disabled, without a password or passkey,
//! and `User::save` / `User::delete` refuse to touch it), so it shows up in the user list and
//! nobody can log in as it.

use crate::database::DB;
use crate::entity::audit_users::AuditUser;
use crate::entity::users::User;
use chrono::Utc;
use hiqlite::macros::{FromRow, params};
use rauthy_common::is_hiqlite;
use rauthy_common::utils::new_store_id;
use rauthy_derive::FromPgRow;
use rauthy_error::{ErrorResponse, ErrorResponseType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info};

/// The user that owns what the system does on its own. System users have numbers as ids (101, 102,
/// ...) and `<number>@system.invalid` as email: no other user can have a name that starts with a
/// number, or an email in the reserved domain (see `check_not_system_email`).
pub const SYSTEM_USER_ID: &str = "101";

/// The email domain of the system users. `.invalid` can never be registered, so no mailbox exists.
pub const SYSTEM_EMAIL_DOMAIN: &str = "system.invalid";

/// Ids of system users are short numbers. The ids of other users are 24 random characters.
pub fn is_system_user_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 8 && id.bytes().all(|b| b.is_ascii_digit())
}

/// System users are read only: they cannot be changed or deleted by anyone
pub fn refuse_system_user_id(id: &str) -> Result<(), ErrorResponse> {
    if is_system_user_id(id) {
        return Err(ErrorResponse::new(
            ErrorResponseType::Forbidden,
            "System users cannot be changed or deleted",
        ));
    }
    Ok(())
}

/// Refuses an email in the domain reserved for system users (and its subdomains)
pub fn check_not_system_email(email: &str) -> Result<(), ErrorResponse> {
    let domain = email.rsplit('@').next().unwrap_or_default().to_lowercase();
    let reserved =
        domain == SYSTEM_EMAIL_DOMAIN || domain.ends_with(&format!(".{SYSTEM_EMAIL_DOMAIN}"));
    if reserved && email.contains('@') {
        return Err(ErrorResponse::new(
            ErrorResponseType::BadRequest,
            format!("The domain @{SYSTEM_EMAIL_DOMAIN} is reserved for system users"),
        ));
    }
    Ok(())
}

/// Only the longest `details` we keep, so an entry stays small
const MAX_DETAILS: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, FromPgRow)]
pub struct AuditEntry {
    pub id: String,
    /// Unix timestamp in milliseconds, so entries of the same second keep their order
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

#[derive(Debug, Clone)]
pub struct AuditSummary {
    pub created: Option<AuditEntry>,
    pub modified: Option<AuditEntry>,
}

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
        // the same line goes to the log, so a log collector has the audit trail too
        info!(
            target: "audit",
            actor,
            action,
            target_type,
            target_id = target_id.unwrap_or_default(),
            details = details.as_deref().unwrap_or_default(),
            "audit"
        );
        let id = new_store_id();
        let at = Utc::now().timestamp_millis();
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

    /// The newest entries first, older than `before` (a timestamp in milliseconds). With `target_id`,
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

    /// Who created the thing, and who changed it last. An entry with the action `create` is
    /// needed for the first one: what existed before the audit log has no creator on record.
    pub async fn summary(target_id: &str) -> Result<AuditSummary, ErrorResponse> {
        let sql = "SELECT id, at, actor, action, target_type, target_id, details FROM audit_log \
            WHERE target_id = $1 AND action = 'create' ORDER BY at ASC LIMIT 1";
        let created: Option<AuditEntry> = if is_hiqlite() {
            DB::hql()
                .query_map_optional(sql, params!(target_id))
                .await?
        } else {
            DB::pg_query_opt(sql, &[&target_id]).await?
        };
        let modified = Self::find(Some(target_id), i64::MAX, 1).await?.pop();
        Ok(AuditSummary { created, modified })
    }

    /// A readable name for each id: the email of the user, "System", or the name of an API key.
    /// A user who was deleted is named from the copy kept for the audit log, and one whose
    /// personal data was wiped has none. Ids without a name are left out, show the id then.
    pub async fn labels<'a>(ids: impl Iterator<Item = &'a str>) -> HashMap<String, String> {
        let mut out = HashMap::new();
        for id in ids {
            if out.contains_key(id) {
                continue;
            }
            let label = if is_system_user_id(id) {
                // named by what it does, not by its `<number>@system.invalid` email
                match User::find(id.to_string()).await {
                    Ok(u) => Some(format!("System: {}", u.family_name.unwrap_or(u.given_name))),
                    Err(_) => Some(format!("System user {id}")),
                }
            } else if let Some(name) = id.strip_prefix("api_key:") {
                Some(format!("API key {name}"))
            } else if let Ok(user) = User::find(id.to_string()).await {
                Some(user.email)
            } else {
                match AuditUser::find(id).await {
                    Ok(Some(u)) if !u.anonymized => u.email,
                    _ => None,
                }
            };
            if let Some(label) = label {
                out.insert(id.to_string(), label);
            }
        }
        out
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_ids_are_short_numbers() {
        for yes in ["101", "102", "1", "99999999"] {
            assert!(is_system_user_id(yes), "{yes}");
        }
        for no in [
            "",
            "system",
            "10a",
            "123456789",
            "a1b2c3d4e5f6g7h8i9j0k1l2",
            " 101",
            "-1",
        ] {
            assert!(!is_system_user_id(no), "{no}");
        }
        assert!(is_system_user_id(SYSTEM_USER_ID));
    }

    #[test]
    fn the_system_email_domain_is_reserved() {
        for bad in [
            "101@system.invalid",
            "anything@system.invalid",
            "Someone@SYSTEM.INVALID",
            "a@mail.system.invalid",
        ] {
            assert!(check_not_system_email(bad).is_err(), "{bad}");
        }
        for ok in [
            "a@example.com",
            "a@system.com",
            "a@notsystem.invalid",
            "system.invalid@example.com",
            "a@other.invalid",
        ] {
            assert!(check_not_system_email(ok).is_ok(), "{ok}");
        }
    }
}
