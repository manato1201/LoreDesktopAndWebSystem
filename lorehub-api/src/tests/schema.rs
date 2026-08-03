//! Schema-level assertions for the new VCS tables added by `migrations/
//! 0001_vcs_schema.sql` (see `db::connect`, which now runs `sqlx::
//! migrate!("./migrations")` on every pool it opens). These are deliberately
//! NOT HTTP tests — nothing reads or writes these tables from a handler yet
//! (that's later phases), so `test_app()`'s HTTP harness isn't the right
//! tool. Instead these go through `test_pool()` and talk to the `SqlitePool`
//! directly.
use sqlx::Row;

use crate::tests::test_pool;

const EXPECTED_TABLES: &[&str] = &[
    "kv_store",
    "repositories",
    "file_blobs",
    "branches",
    "current_branch",
    "commits",
    "commit_files",
    "tree_entries",
    "staged_content",
    "pending_changes",
    "file_locks",
];

#[tokio::test]
async fn every_migrated_table_exists() {
    let pool = test_pool().await;

    for table in EXPECTED_TABLES {
        let row = sqlx::query("SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1")
            .bind(table)
            .fetch_optional(&pool)
            .await
            .expect("query sqlite_master");
        assert!(
            row.is_some(),
            "expected table {table:?} to exist after migrations ran"
        );
    }
}

/// `PRAGMA foreign_keys = ON` (set in `db::connect`) must actually reject a
/// `branches` row whose `repo_slug` doesn't exist in `repositories` — that's
/// the entire point of turning it on. Without the pragma this insert would
/// silently succeed (SQLite defaults foreign key enforcement to OFF per
/// connection), leaving an orphaned row.
#[tokio::test]
async fn foreign_key_violation_is_rejected() {
    let pool = test_pool().await;

    let result = sqlx::query(
        "INSERT INTO branches (repo_slug, name, head_commit_hash, is_default)
         VALUES (?1, ?2, NULL, 0)",
    )
    .bind("does-not-exist")
    .bind("main")
    .execute(&pool)
    .await;

    assert!(
        result.is_err(),
        "inserting a branches row with a nonexistent repo_slug should be rejected by the \
         repo_slug foreign key now that PRAGMA foreign_keys = ON is set"
    );
}

/// Sanity check that the FK constraint machinery itself works (not just
/// enforcement) — the same insert as above succeeds once its `repo_slug`
/// legitimately exists in `repositories`.
#[tokio::test]
async fn insert_succeeds_once_referenced_repository_exists() {
    let pool = test_pool().await;

    sqlx::query(
        "INSERT INTO repositories
             (slug, name, organization, description, visibility, is_seeded,
              size_label, locked_file_count, updated_at, created_at)
         VALUES (?1, 'Demo', 'Nebula Studios', 'desc', 'private', 0, '0 B', 0, 'just now', 0)",
    )
    .bind("demo-repo")
    .execute(&pool)
    .await
    .expect("insert repositories row");

    let result = sqlx::query(
        "INSERT INTO branches (repo_slug, name, head_commit_hash, is_default)
         VALUES (?1, ?2, NULL, 1)",
    )
    .bind("demo-repo")
    .bind("main")
    .execute(&pool)
    .await;

    assert!(
        result.is_ok(),
        "inserting a branches row with a valid repo_slug should succeed"
    );

    let row = sqlx::query("SELECT COUNT(*) as count FROM branches WHERE repo_slug = ?1")
        .bind("demo-repo")
        .fetch_one(&pool)
        .await
        .expect("count branches");
    let count: i64 = row.get("count");
    assert_eq!(count, 1);
}
