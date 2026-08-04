//! SQL-backed operations against the Phase-1 `file_blobs` and
//! `staged_content` tables (see `migrations/0001_vcs_schema.sql`).
//!
//! `blob_store.rs` owns the pure filesystem I/O for a blob's actual bytes;
//! this module owns the SQL metadata half — which content hashes exist for
//! a repo (`file_blobs`, deduped by `(repo_slug, content_hash)`) and which
//! content hash a path's working copy currently points at
//! (`staged_content`, one row per `(repo_slug, path)`). Kept in its own
//! module rather than folded into `blob_store.rs`, matching this codebase's
//! existing habit of one clear responsibility per module — see
//! `repo_store.rs`'s doc comment for the same reasoning applied to the
//! `repositories` table.
//!
//! No `AppState` lock is needed for anything here: both tables are read/
//! written directly against the `SqlitePool`, the same pattern
//! `repo_store.rs` uses for `repositories`.

use sqlx::{Row, SqlitePool};

/// Inserts a `file_blobs` row for `(repo_slug, content_hash)` if one doesn't
/// already exist. This is the real dedup point: uploading identical content
/// to two different paths (even within the same repo) produces exactly one
/// row here, since the primary key is `(repo_slug, content_hash)` alone —
/// `path` never appears in this table at all.
pub async fn record_blob(pool: &SqlitePool, repo_slug: &str, content_hash: &str, size_bytes: i64) {
    sqlx::query(
        "INSERT INTO file_blobs (repo_slug, content_hash, size_bytes, created_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(repo_slug, content_hash) DO NOTHING",
    )
    .bind(repo_slug)
    .bind(content_hash)
    .bind(size_bytes)
    .bind(crate::auth::current_unix_time())
    .execute(pool)
    .await
    .expect("insert file_blobs row");
}

/// Upserts the `staged_content` row for `(repo_slug, path)` — the "current
/// working-copy pointer" for that path, mirroring what an actual working
/// directory would hold. A re-upload to the same path simply repoints this
/// row at the new content hash/size; the previous blob's `file_blobs` row
/// and on-disk bytes are left exactly as they were (content-addressed
/// storage keeps every version around forever — see `blob_store.rs` — this
/// is deliberate, not a leak).
pub async fn upsert_staged_content(
    pool: &SqlitePool,
    repo_slug: &str,
    path: &str,
    content_hash: &str,
    size_bytes: i64,
) {
    sqlx::query(
        "INSERT INTO staged_content (repo_slug, path, content_hash, size_bytes, uploaded_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(repo_slug, path) DO UPDATE SET
             content_hash = excluded.content_hash,
             size_bytes = excluded.size_bytes,
             uploaded_at = excluded.uploaded_at",
    )
    .bind(repo_slug)
    .bind(path)
    .bind(content_hash)
    .bind(size_bytes)
    .bind(crate::auth::current_unix_time())
    .execute(pool)
    .await
    .expect("upsert staged_content row");
}

/// Returns `(content_hash, size_bytes)` for `path` in `repo_slug`'s working
/// copy, or `None` if nothing has ever been uploaded to that path in this
/// repo. Used by the read path (`handlers::resolve_uploaded_content`) before
/// falling back to the org-wide seeded content maps.
pub async fn get_staged_content(
    pool: &SqlitePool,
    repo_slug: &str,
    path: &str,
) -> Option<(String, i64)> {
    sqlx::query(
        "SELECT content_hash, size_bytes FROM staged_content
         WHERE repo_slug = ?1 AND path = ?2",
    )
    .bind(repo_slug)
    .bind(path)
    .fetch_optional(pool)
    .await
    .expect("query staged_content")
    .map(|row| (row.get("content_hash"), row.get("size_bytes")))
}
