use crate::database::DB;
use chrono::Utc;
use hiqlite::macros::{FromRow, params};
use rauthy_common::is_hiqlite;
use rauthy_derive::FromPgRow;
use rauthy_error::ErrorResponse;
use serde::Deserialize;

/// Raw `user_clients` row, used by the DB migration tooling
#[derive(Debug, Deserialize, FromRow, FromPgRow)]
pub struct UserClientRow {
    pub user_id: String,
    pub client_id: String,
    pub first_at: i64,
    pub last_at: i64,
    pub scope: Option<String>,
    pub logins: i64,
}

/// Audit record of the clients a user has received tokens for.
#[derive(Debug, FromRow, FromPgRow)]
pub struct UserClient {
    pub client_id: String,
    pub client_name: Option<String>,
    pub first_at: i64,
    pub last_at: i64,
    pub scope: Option<String>,
    pub logins: i64,
}

impl UserClient {
    /// Records that `user_id` has just been issued tokens for `client_id`.
    pub async fn touch(user_id: &str, client_id: &str, scope: &str) -> Result<(), ErrorResponse> {
        let now = Utc::now().timestamp();
        let sql = r#"
INSERT INTO user_clients (user_id, client_id, first_at, last_at, scope, logins)
VALUES ($1, $2, $3, $3, $4, 1)
ON CONFLICT (user_id, client_id)
DO UPDATE SET last_at = $3, scope = $4, logins = user_clients.logins + 1"#;

        if is_hiqlite() {
            DB::hql()
                .execute(sql, params!(user_id, client_id, now, scope))
                .await?;
        } else {
            DB::pg_execute(sql, &[&user_id, &client_id, &now, &scope]).await?;
        }

        Ok(())
    }

    pub async fn find_for_user(user_id: &str) -> Result<Vec<Self>, ErrorResponse> {
        let sql = r#"
SELECT uc.client_id, c.name AS client_name, uc.first_at, uc.last_at, uc.scope, uc.logins
FROM user_clients uc
JOIN clients c ON c.id = uc.client_id
WHERE uc.user_id = $1
ORDER BY uc.last_at DESC"#;

        let res = if is_hiqlite() {
            DB::hql().query_map(sql, params!(user_id)).await?
        } else {
            DB::pg_query(sql, &[&user_id], 0).await?
        };

        Ok(res)
    }
}
