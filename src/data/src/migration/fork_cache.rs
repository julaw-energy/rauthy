//! Drops the cache once whenever the build changes.
//!
//! The cache is persisted on disk (`cache_storage_disk`) and the cached structs are stored in a
//! binary format. After an upgrade a struct may have another shape, and the old entries can not be
//! read any more ("invalid length 26, expected struct Client with 27 elements"). Everything cached
//! is only a copy of the database, so it is safe to drop.
//!
//! `RAUTHY_VERSION` does not tell our releases apart (the crate version stays the same between
//! several of them), so the image sets `RAUTHY_BUILD_ID` to its own version.

use crate::database::DB;
use crate::entity::config::ConfigEntity;
use hiqlite::macros::params;
use rauthy_common::constants::RAUTHY_VERSION;
use rauthy_common::is_hiqlite;
use tracing::{info, warn};

const MARKER_ID: &str = "fork_build_id";

fn build_id() -> String {
    std::env::var("RAUTHY_BUILD_ID")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| RAUTHY_VERSION.to_string())
}

async fn stored() -> Option<String> {
    let sql = "SELECT * FROM config WHERE id = $1";
    let config: ConfigEntity = if is_hiqlite() {
        DB::hql()
            .query_as_optional(sql, params!(MARKER_ID))
            .await
            .ok()??
    } else {
        DB::pg_query_opt(sql, &[&MARKER_ID]).await.ok()??
    };
    String::from_utf8(config.data).ok()
}

async fn store(id: &str) -> Result<(), rauthy_error::ErrorResponse> {
    let sql =
        "INSERT INTO config (id, data) VALUES ($1, $2) ON CONFLICT(id) DO UPDATE SET data = $2";
    let data = id.as_bytes().to_vec();
    if is_hiqlite() {
        DB::hql().execute(sql, params!(MARKER_ID, data)).await?;
    } else {
        DB::pg_execute(sql, &[&MARKER_ID, &data]).await?;
    }
    Ok(())
}

/// Never fails the start: at worst a stale entry is dropped later, when it can not be read.
pub async fn clear_if_build_changed() {
    let now = build_id();
    let before = stored().await;
    if before.as_deref() == Some(now.as_str()) {
        return;
    }

    info!(
        "Build changed from {} to {now}, clearing the cache",
        before.as_deref().unwrap_or("an unknown one")
    );
    if let Err(err) = DB::hql().clear_cache_all().await {
        warn!(?err, "Could not clear the cache after the build changed");
        return;
    }
    if let Err(err) = store(&now).await {
        warn!(?err, "Could not store the build id");
    }
}
