//! Persistence for `AppState`.
//!
//! Rather than fully normalizing every nested type (`TreeNode`,
//! `PullRequest`'s comments/diff files, etc.) into relational tables, each
//! top-level field of `AppState` is stored as one JSON blob in a single
//! `kv_store` table, keyed by field name. This is a deliberate scope
//! trade-off for a demo backend: it gets genuine restart-survival with a
//! small, low-risk change (`AppState` and every handler's read path stay
//! exactly as they were), at the cost of not being able to query/index
//! individual rows in SQL. A production version would normalize the
//! frequently-queried pieces (org_members, access_entries, sessions,
//! audit_log at least) into real tables.
use std::str::FromStr;

use serde::Serialize;
use serde::de::DeserializeOwned;
use sqlx::Row;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};

use crate::state::AppState;

pub async fn connect(path: &str) -> SqlitePool {
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{path}"))
        .expect("invalid sqlite path")
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .expect("failed to open sqlite database");

    sqlx::query("CREATE TABLE IF NOT EXISTS kv_store (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
        .execute(&pool)
        .await
        .expect("failed to create kv_store table");

    pool
}

pub async fn save_blob<T: Serialize>(pool: &SqlitePool, key: &str, value: &T) {
    let json = serde_json::to_string(value).expect("serialize state blob");
    sqlx::query(
        "INSERT INTO kv_store (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(json)
    .execute(pool)
    .await
    .expect("persist state blob");
}

async fn load_blob<T: DeserializeOwned>(pool: &SqlitePool, key: &str) -> Option<T> {
    let row = sqlx::query("SELECT value FROM kv_store WHERE key = ?1")
        .bind(key)
        .fetch_optional(pool)
        .await
        .expect("query state blob");

    row.map(|r| {
        let value: String = r.get("value");
        serde_json::from_str(&value).expect("deserialize state blob")
    })
}

/// `None` means this is a first run (no prior save) — the caller should
/// seed fresh demo data and call [`save_all`].
pub async fn load_state(pool: &SqlitePool) -> Option<AppState> {
    Some(AppState {
        repositories: load_blob(pool, "repositories").await?,
        tree: load_blob(pool, "tree").await.unwrap_or_default(),
        file_contents: load_blob(pool, "file_contents").await.unwrap_or_default(),
        image_content: load_blob(pool, "image_content").await.unwrap_or_default(),
        image_content_before: load_blob(pool, "image_content_before")
            .await
            .unwrap_or_default(),
        audio_content: load_blob(pool, "audio_content").await.unwrap_or_default(),
        commits: load_blob(pool, "commits").await.unwrap_or_default(),
        branches: load_blob(pool, "branches").await.unwrap_or_default(),
        pull_requests: load_blob(pool, "pull_requests").await.unwrap_or_default(),
        access_entries: load_blob(pool, "access_entries").await.unwrap_or_default(),
        org_members: load_blob(pool, "org_members").await.unwrap_or_default(),
        storage: load_blob(pool, "storage")
            .await
            .unwrap_or(crate::models::StorageUsage {
                used_label: "0 GB".into(),
                total_label: "5 TB".into(),
                used_percent: 0,
            }),
        audit_log: load_blob(pool, "audit_log").await.unwrap_or_default(),
        credentials: load_blob(pool, "credentials").await.unwrap_or_default(),
        sessions: load_blob(pool, "sessions").await.unwrap_or_default(),
    })
}

pub async fn save_all(pool: &SqlitePool, state: &AppState) {
    save_blob(pool, "repositories", &state.repositories).await;
    save_blob(pool, "tree", &state.tree).await;
    save_blob(pool, "file_contents", &state.file_contents).await;
    save_blob(pool, "image_content", &state.image_content).await;
    save_blob(pool, "image_content_before", &state.image_content_before).await;
    save_blob(pool, "audio_content", &state.audio_content).await;
    save_blob(pool, "commits", &state.commits).await;
    save_blob(pool, "branches", &state.branches).await;
    save_blob(pool, "pull_requests", &state.pull_requests).await;
    save_blob(pool, "access_entries", &state.access_entries).await;
    save_blob(pool, "org_members", &state.org_members).await;
    save_blob(pool, "storage", &state.storage).await;
    save_blob(pool, "audit_log", &state.audit_log).await;
    save_blob(pool, "credentials", &state.credentials).await;
    save_blob(pool, "sessions", &state.sessions).await;
}
