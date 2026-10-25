//! The database changes of this fork, kept apart from upstream's migrations.
//!
//! Hiqlite refuses to start when the migrations it has applied are not exactly a prefix of the
//! built-in ones (and refinery behaves the same for Postgres). If our changes shared upstream's
//! numbering, every migration that upstream adds would collide with one of ours. So ours live in
//! `migrations_fork/` and run after upstream's, tracked in `_fork_migrations`.
//!
//! The first version of this fork put them into upstream's numbering as 36-43 (Hiqlite) and
//! 31-38 (Postgres). A database that has them there is adopted once: they are recorded here and
//! removed from upstream's table, so upstream's numbers 36+ are free again.

use crate::database::DB;
use hiqlite::macros::{FromRow, embed::*, params};
use rauthy_common::is_hiqlite;
use rauthy_derive::FromPgRow;
use rauthy_error::{ErrorResponse, ErrorResponseType};
use std::ops::DerefMut;
use tracing::{info, warn};

#[derive(Embed)]
#[folder = "../../migrations_fork/hiqlite"]
struct ForkHiqlite;

#[derive(Embed)]
#[folder = "../../migrations_fork/postgres"]
struct ForkPostgres;

/// Where our first 8 migrations were numbered in upstream's table, `(name, hiqlite id, postgres
/// version)`. Their position here is the id they have now (1 to 8).
const LEGACY: [(&str, i64, i32); 8] = [
    ("user_gender", 36, 31),
    ("user_clients", 37, 32),
    ("user_attr_types", 38, 33),
    ("email_jobs_channel", 39, 34),
    ("contact_verifications", 40, 35),
    ("settings", 41, 36),
    ("client_required_fields", 42, 37),
    ("attr_verify_channels", 43, 38),
];

#[derive(Debug, Clone)]
struct Mig {
    id: i64,
    name: String,
    hash: String,
    sql: String,
}

#[derive(Debug, FromRow, FromPgRow)]
struct Tracked {
    id: i64,
    name: String,
    hash: String,
}

fn load<T: RustEmbed>() -> Vec<Mig> {
    let mut out: Vec<Mig> = T::iter()
        .filter(|f| f.ends_with(".sql"))
        .map(|f| {
            let stem = f.trim_end_matches(".sql");
            let (id, name) = stem
                .split_once('_')
                .unwrap_or_else(|| panic!("Invalid fork migration file name '{f}'"));
            let id: i64 = id
                .parse()
                .unwrap_or_else(|_| panic!("Invalid fork migration id in '{f}'"));
            let file = T::get(&f).expect("embedded fork migration");
            Mig {
                id,
                name: name.to_string(),
                hash: hex::encode(hmac_sha256::Hash::hash(file.data.as_ref())),
                sql: String::from_utf8_lossy(file.data.as_ref()).to_string(),
            }
        })
        .collect();
    out.sort_by_key(|m| m.id);
    for (i, m) in out.iter().enumerate() {
        assert_eq!(
            m.id,
            i as i64 + 1,
            "The fork migrations must be numbered 1, 2, 3 ... without gaps"
        );
    }
    out
}

/// The statements of a migration file: comments removed, split on `;`
fn statements(sql: &str) -> Vec<String> {
    sql.lines()
        .filter(|l| !l.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n")
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn mig_err(msg: impl Into<String>) -> ErrorResponse {
    ErrorResponse::new(ErrorResponseType::Database, msg.into())
}

/// Checks that what was applied matches the built-in migrations. Returns the ones still to apply.
fn pending(migs: &[Mig], tracked: &[Tracked]) -> Vec<Mig> {
    for (i, t) in tracked.iter().enumerate() {
        match migs.get(i) {
            None => {
                warn!(
                    "Fork migration {}_{} was applied, but this build does not have it. It was \
                    written by a NEWER build.",
                    t.id, t.name
                );
                return Vec::new();
            }
            Some(m) => {
                if m.id != t.id || m.name != t.name {
                    panic!(
                        "Fork migration mismatch: applied {}_{}, built in {}_{}",
                        t.id, t.name, m.id, m.name
                    );
                }
                if m.hash != t.hash {
                    panic!(
                        "HASH mismatch for the fork migration '{}': a migration that was applied \
                        must never be edited",
                        m.name
                    );
                }
            }
        }
    }
    migs.iter().skip(tracked.len()).cloned().collect()
}

pub async fn apply() -> Result<(), ErrorResponse> {
    if is_hiqlite() {
        apply_hiqlite().await
    } else {
        apply_postgres().await
    }
}

// ---- Hiqlite ----------------------------------------------------------------------------

async fn tracked_hiqlite() -> Result<Vec<Tracked>, ErrorResponse> {
    Ok(DB::hql()
        .query_map("SELECT id, name, hash FROM _fork_migrations ORDER BY id ASC", params!())
        .await?)
}

async fn apply_hiqlite() -> Result<(), ErrorResponse> {
    let db = DB::hql();
    db.execute(
        r#"
CREATE TABLE IF NOT EXISTS _fork_migrations
(
    id   INTEGER NOT NULL CONSTRAINT _fork_migrations_pk PRIMARY KEY,
    name TEXT    NOT NULL,
    hash TEXT    NOT NULL,
    ts   INTEGER NOT NULL
) STRICT"#,
        params!(),
    )
    .await?;

    let migs = load::<ForkHiqlite>();
    let mut tracked = tracked_hiqlite().await?;

    if tracked.is_empty() {
        adopt_hiqlite(&migs).await?;
        tracked = tracked_hiqlite().await?;
    }

    let todo = pending(&migs, &tracked);
    if todo.is_empty() {
        info!("All fork migrations have been applied already - nothing to migrate");
        return Ok(());
    }

    for m in todo {
        info!("Applying fork migration {} {}", m.id, m.name);
        let now = chrono::Utc::now().timestamp();
        let mut queries: Vec<(String, hiqlite::Params)> =
            statements(&m.sql).into_iter().map(|s| (s, params!())).collect();
        queries.push((
            "INSERT INTO _fork_migrations (id, name, hash, ts) VALUES ($1, $2, $3, $4)".into(),
            params!(m.id, m.name.clone(), m.hash.clone(), now),
        ));

        let res = db.txn(queries).await;
        let failed = match &res {
            Ok(inner) => inner.iter().any(|r| r.is_err()),
            Err(_) => true,
        };
        if failed {
            // another node may have been faster, which is fine
            if tracked_hiqlite().await?.iter().any(|t| t.id == m.id) {
                continue;
            }
            return Err(mig_err(format!(
                "The fork migration {} {} failed: {:?}",
                m.id, m.name, res
            )));
        }
    }
    Ok(())
}

/// A database that has our first 8 migrations in upstream's table: record them here, free the
/// numbers there.
async fn adopt_hiqlite(migs: &[Mig]) -> Result<(), ErrorResponse> {
    let db = DB::hql();
    let rows: Vec<Tracked> = db
        .query_map(
            "SELECT id, name, hash FROM _migrations WHERE id >= 36 ORDER BY id ASC",
            params!(),
        )
        .await
        .unwrap_or_default();
    let ours: Vec<(usize, &Tracked)> = rows
        .iter()
        .filter_map(|r| {
            LEGACY
                .iter()
                .position(|(n, id, _)| *n == r.name && *id == r.id)
                .map(|i| (i, r))
        })
        .collect();
    if ours.is_empty() {
        return Ok(());
    }
    // all of them or nothing, a partial set would leave the database in a state nobody tested
    if ours.len() != LEGACY.len() {
        return Err(mig_err(
            "The database has only some of the first fork migrations in upstream's migration \
            table. This needs a manual look.",
        ));
    }

    warn!("Adopting the fork migrations 36-43 from upstream's migration table");
    let now = chrono::Utc::now().timestamp();
    let mut queries: Vec<(String, hiqlite::Params)> = Vec::new();
    for (i, r) in &ours {
        let m = &migs[*i];
        queries.push((
            "INSERT INTO _fork_migrations (id, name, hash, ts) VALUES ($1, $2, $3, $4)".into(),
            params!(m.id, m.name.clone(), m.hash.clone(), now),
        ));
        queries.push((
            "DELETE FROM _migrations WHERE id = $1 AND name = $2".into(),
            params!(r.id, r.name.clone()),
        ));
    }
    let res = db.txn(queries).await?;
    if res.iter().any(|r| r.is_err()) {
        return Err(mig_err(format!("Adopting the fork migrations failed: {res:?}")));
    }
    Ok(())
}

// ---- Postgres ---------------------------------------------------------------------------

async fn tracked_postgres() -> Result<Vec<Tracked>, ErrorResponse> {
    DB::pg_query(
        "SELECT id, name, hash FROM _fork_migrations ORDER BY id ASC",
        &[],
        0,
    )
    .await
}

async fn apply_postgres() -> Result<(), ErrorResponse> {
    DB::pg_execute(
        r#"
CREATE TABLE IF NOT EXISTS _fork_migrations
(
    id   BIGINT  NOT NULL CONSTRAINT _fork_migrations_pk PRIMARY KEY,
    name VARCHAR NOT NULL,
    hash VARCHAR NOT NULL,
    ts   BIGINT  NOT NULL
)"#,
        &[],
    )
    .await?;

    let migs = load::<ForkPostgres>();
    let mut tracked = tracked_postgres().await?;

    if tracked.is_empty() {
        adopt_postgres(&migs).await?;
        tracked = tracked_postgres().await?;
    }

    let todo = pending(&migs, &tracked);
    if todo.is_empty() {
        info!("All fork migrations have been applied already - nothing to migrate");
        return Ok(());
    }

    for m in todo {
        info!("Applying fork migration {} {}", m.id, m.name);
        let now = chrono::Utc::now().timestamp();
        let mut cl = DB::pg().await?;
        let cl = cl.deref_mut().deref_mut();
        let tx = cl.transaction().await?;
        tx.batch_execute(&m.sql).await?;
        tx.execute(
            "INSERT INTO _fork_migrations (id, name, hash, ts) VALUES ($1, $2, $3, $4)",
            &[&m.id, &m.name, &m.hash, &now],
        )
        .await?;
        tx.commit().await?;
    }
    Ok(())
}

#[derive(Debug, FromPgRow)]
struct HistoryRow {
    version: i32,
    name: String,
}

async fn adopt_postgres(migs: &[Mig]) -> Result<(), ErrorResponse> {
    let rows: Vec<HistoryRow> = match DB::pg_query(
        "SELECT version, name FROM refinery_schema_history WHERE version >= 31 ORDER BY version ASC",
        &[],
        0,
    )
    .await
    {
        Ok(r) => r,
        Err(_) => return Ok(()),
    };
    let ours: Vec<(usize, &HistoryRow)> = rows
        .iter()
        .filter_map(|r| {
            LEGACY
                .iter()
                .position(|(n, _, v)| *n == r.name && *v == r.version)
                .map(|i| (i, r))
        })
        .collect();
    if ours.is_empty() {
        return Ok(());
    }
    if ours.len() != LEGACY.len() {
        return Err(mig_err(
            "The database has only some of the first fork migrations in the refinery history. \
            This needs a manual look.",
        ));
    }

    warn!("Adopting the fork migrations V31-V38 from the refinery history");
    let now = chrono::Utc::now().timestamp();
    let mut cl = DB::pg().await?;
    let cl = cl.deref_mut().deref_mut();
    let tx = cl.transaction().await?;
    for (i, r) in &ours {
        let m = &migs[*i];
        tx.execute(
            "INSERT INTO _fork_migrations (id, name, hash, ts) VALUES ($1, $2, $3, $4)",
            &[&m.id, &m.name, &m.hash, &now],
        )
        .await?;
        tx.execute(
            "DELETE FROM refinery_schema_history WHERE version = $1 AND name = $2",
            &[&r.version, &r.name],
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_numbered_without_gaps() {
        // `load` asserts it
        assert_eq!(load::<ForkHiqlite>().len(), LEGACY.len());
        assert_eq!(load::<ForkPostgres>().len(), LEGACY.len());
    }

    #[test]
    fn legacy_names_match_the_files() {
        for (idx, (name, _, _)) in LEGACY.iter().enumerate() {
            assert_eq!(load::<ForkHiqlite>()[idx].name, *name);
            assert_eq!(load::<ForkPostgres>()[idx].name, *name);
        }
    }

    #[test]
    fn splits_statements_and_drops_comments() {
        let s = statements("-- a comment; with a semicolon\nCREATE TABLE a (x INT);\n\nALTER TABLE a ADD y INT;\n");
        assert_eq!(s, ["CREATE TABLE a (x INT)", "ALTER TABLE a ADD y INT"]);
    }

    #[test]
    fn pending_returns_the_missing_tail() {
        let m = |id: i64, name: &str| Mig {
            id,
            name: name.into(),
            hash: format!("h{id}"),
            sql: String::new(),
        };
        let migs = vec![m(1, "a"), m(2, "b"), m(3, "c")];
        let t = vec![Tracked {
            id: 1,
            name: "a".into(),
            hash: "h1".into(),
        }];
        let todo = pending(&migs, &t);
        assert_eq!(todo.iter().map(|m| m.id).collect::<Vec<_>>(), [2, 3]);
        assert!(pending(&migs, &[]).len() == 3);
    }
}
