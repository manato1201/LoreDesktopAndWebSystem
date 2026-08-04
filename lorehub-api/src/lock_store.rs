//! SQL-backed CRUD for the `file_locks` table (see
//! `migrations/0001_vcs_schema.sql`).
//!
//! Kept as its own module rather than folded into `vcs_store.rs` because the
//! schema's own comment on this table says why: locks are "independent of
//! commit history" (Perforce-style exclusive locking, not versioned) —
//! unlike branches/commits/tree/pending changes, which all revolve around
//! the same commit-creation transaction, locking has nothing to do with that
//! transaction at all. Mirrors `repo_store.rs`/`blob_meta_store.rs`'s
//! existing one-module-per-concern style.
//!
//! This replaces `AppState::set_lock`'s recursive in-memory tree walk (see
//! `state.rs`, removed in this phase) with a direct table CRUD — `handlers::
//! toggle_lock` no longer needs to hold the tree in memory at all to lock a
//! path.

use std::collections::HashMap;

use sqlx::{Row, SqlitePool};

/// Every currently-locked path in `repo_slug`, as `path -> locked_by`. Used
/// by `vcs_store::build_tree` to overlay `lockedBy` onto matching leaf
/// `TreeNode`s.
pub async fn list_locks(pool: &SqlitePool, repo_slug: &str) -> HashMap<String, String> {
    sqlx::query("SELECT path, locked_by FROM file_locks WHERE repo_slug = ?1")
        .bind(repo_slug)
        .fetch_all(pool)
        .await
        .expect("query file_locks")
        .iter()
        .map(|row| {
            (
                row.get::<String, _>("path"),
                row.get::<String, _>("locked_by"),
            )
        })
        .collect()
}

/// Locks `path` as `locked_by`, overwriting any existing lock on the same
/// path (re-locking as a different user simply reassigns it — matches the
/// pre-redesign in-memory `set_lock`'s unconditional overwrite behavior).
pub async fn lock(pool: &SqlitePool, repo_slug: &str, path: &str, locked_by: &str) {
    sqlx::query(
        "INSERT INTO file_locks (repo_slug, path, locked_by, locked_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(repo_slug, path) DO UPDATE SET
             locked_by = excluded.locked_by,
             locked_at = excluded.locked_at",
    )
    .bind(repo_slug)
    .bind(path)
    .bind(locked_by)
    .bind(crate::auth::current_unix_time())
    .execute(pool)
    .await
    .expect("upsert file_locks row");
}

pub async fn unlock(pool: &SqlitePool, repo_slug: &str, path: &str) {
    sqlx::query("DELETE FROM file_locks WHERE repo_slug = ?1 AND path = ?2")
        .bind(repo_slug)
        .bind(path)
        .execute(pool)
        .await
        .expect("delete file_locks row");
}
