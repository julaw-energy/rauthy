use crate::database::DB;
use hiqlite::macros::{FromRow, params};
use rauthy_common::is_hiqlite;
use rauthy_derive::FromPgRow;
use rauthy_error::ErrorResponse;
use serde::Deserialize;

/// Raw `event_read_state` row, used by the DB migration tooling.
/// Every query that maps into it must select both columns, a missing one panics at runtime.
#[derive(Debug, Deserialize, FromRow, FromPgRow)]
pub struct EventReadStateRow {
    pub user_id: String,
    pub read_until: i64,
}

/// How far a user has read the events, as a timestamp in milliseconds like the events have. The
/// bell in the admin UI counts what is newer. The marker only ever moves forward, so a tab with
/// an older view cannot bring back what another device already marked as read.
pub struct EventReadState;

impl EventReadState {
    pub async fn find_all() -> Result<Vec<EventReadStateRow>, ErrorResponse> {
        let sql = "SELECT user_id, read_until FROM event_read_state";
        Ok(if is_hiqlite() {
            DB::hql().query_map(sql, params!()).await?
        } else {
            DB::pg_query(sql, &[], 0).await?
        })
    }

    /// `0` if the user never read any
    pub async fn find(user_id: &str) -> Result<i64, ErrorResponse> {
        let sql = "SELECT user_id, read_until FROM event_read_state WHERE user_id = $1";
        let row: Option<EventReadStateRow> = if is_hiqlite() {
            DB::hql().query_map_optional(sql, params!(user_id)).await?
        } else {
            DB::pg_query_opt(sql, &[&user_id]).await?
        };
        Ok(row.map(|r| r.read_until).unwrap_or_default())
    }

    /// Moves the marker forward to `read_until`, never back. Returns the marker after the update.
    pub async fn advance(user_id: &str, read_until: i64) -> Result<i64, ErrorResponse> {
        if is_hiqlite() {
            let sql = r#"
INSERT INTO event_read_state (user_id, read_until)
VALUES ($1, $2)
ON CONFLICT (user_id) DO UPDATE SET read_until = MAX(read_until, excluded.read_until)"#;
            DB::hql().execute(sql, params!(user_id, read_until)).await?;
        } else {
            let sql = r#"
INSERT INTO event_read_state (user_id, read_until)
VALUES ($1, $2)
ON CONFLICT (user_id) DO UPDATE
SET read_until = GREATEST(event_read_state.read_until, EXCLUDED.read_until)"#;
            DB::pg_execute(sql, &[&user_id, &read_until]).await?;
        }
        Self::find(user_id).await
    }
}
