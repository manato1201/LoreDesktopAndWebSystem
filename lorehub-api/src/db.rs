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
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};

use crate::state::AppState;

/// Opens (creating if missing) the SQLite database at `path` and applies
/// every migration under `./migrations` (see that directory — `kv_store`,
/// then the VCS schema). `path == ":memory:"` is treated specially: it's
/// what every integration test uses (see `tests/mod.rs::test_app_with_
/// config`) for a private, disposable database, and WAL journaling is both
/// meaningless and unsupported in the way this function would otherwise
/// enable it for an in-memory SQLite database, so WAL is only turned on for
/// a real file-backed `path`.
///
/// `PRAGMA foreign_keys = ON` is enabled unconditionally (both real and
/// `:memory:` connections) — SQLite defaults this pragma to OFF per
/// connection, which would otherwise silently allow orphaned rows across the
/// new VCS tables' `repo_slug` foreign keys (all declared `ON DELETE
/// CASCADE` — see `migrations/0001_vcs_schema.sql`).
pub async fn connect(path: &str) -> SqlitePool {
    let is_memory = path == ":memory:";

    let mut options = SqliteConnectOptions::from_str(&format!("sqlite://{path}"))
        .expect("invalid sqlite path")
        .create_if_missing(true)
        .foreign_keys(true);

    if !is_memory {
        options = options.journal_mode(SqliteJournalMode::Wal);
    }

    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .expect("failed to open sqlite database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run database migrations");

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

/// Like [`load_blob`], but treats a value that fails to deserialize into
/// `T` as absent (`None`) instead of panicking. Used for fields whose
/// on-disk shape changed across a refactor (e.g. `tree`/`commits`/
/// `branches` going from a bare `Vec` to a per-repo-slug `HashMap`) so an
/// old `lorehub.db` doesn't crash the server on startup — the caller falls
/// back to fresh defaults for that field instead.
async fn load_blob_lenient<T: DeserializeOwned>(pool: &SqlitePool, key: &str) -> Option<T> {
    let row = sqlx::query("SELECT value FROM kv_store WHERE key = ?1")
        .bind(key)
        .fetch_optional(pool)
        .await
        .expect("query state blob");

    row.and_then(|r| {
        let value: String = r.get("value");
        serde_json::from_str(&value).ok()
    })
}

/// `None` means this is a first run (no prior save) — the caller should
/// seed fresh demo data and call [`save_all`].
///
/// `org_members` is used as the "has this instance ever been saved before"
/// sentinel (via the early-return `?`) — it's been present in every save
/// since long before this VCS redesign, so its absence reliably means a
/// fresh `lorehub.db`. This used to be `repositories`, but that field moved
/// out of the `kv_store` blob scheme entirely once `repositories` became a
/// real SQL table (see `repo_store.rs`) — by the time this function runs,
/// `main()` has already called [`migrate_or_seed_repositories`], so the SQL
/// table itself is never empty here, but whether *this* particular
/// `kv_store` blob save exists is still the right signal for whether to
/// reload the rest of `AppState` versus building it fresh via
/// `state::seed()`.
pub async fn load_state(pool: &SqlitePool) -> Option<AppState> {
    let org_members: Vec<crate::models::OrgMember> = load_blob(pool, "org_members").await?;

    // The SQL `repositories` table's `is_seeded` column (see `repo_store.rs`)
    // is the replacement for the old in-memory `AppState.seeded_repo_slugs` —
    // used only as input to the tree/commits/branches lenient fallback below.
    let seeded_repo_slugs = crate::repo_store::seeded_slugs(pool).await;

    // Older saves predate the tree/commits/branches per-repo-slug map
    // refactor and still have the bare-`Vec` shape on disk for these three
    // keys; `load_blob_lenient` swallows that deserialize failure and we
    // fall back to a fresh per-slug clone of the demo dataset so existing
    // seeded repos keep showing the same Code/Commits/Branches content they
    // did before the refactor.
    let tree = load_blob_lenient(pool, "tree")
        .await
        .unwrap_or_else(|| crate::state::seeded_tree(&seeded_repo_slugs));
    let commits = load_blob_lenient(pool, "commits")
        .await
        .unwrap_or_else(|| crate::state::seeded_commits(&seeded_repo_slugs));
    let branches = load_blob_lenient(pool, "branches")
        .await
        .unwrap_or_else(|| crate::state::seeded_branches(&seeded_repo_slugs));

    Some(AppState {
        tree,
        file_contents: load_blob(pool, "file_contents").await.unwrap_or_default(),
        image_content: load_blob(pool, "image_content").await.unwrap_or_default(),
        image_content_before: load_blob(pool, "image_content_before")
            .await
            .unwrap_or_default(),
        audio_content: load_blob(pool, "audio_content").await.unwrap_or_default(),
        commits,
        branches,
        current_branch: load_blob(pool, "current_branch").await.unwrap_or_default(),
        pending_changes: load_blob(pool, "pending_changes").await.unwrap_or_default(),
        pull_requests: load_blob(pool, "pull_requests").await.unwrap_or_default(),
        access_entries: load_blob(pool, "access_entries").await.unwrap_or_default(),
        org_members,
        storage: load_blob(pool, "storage")
            .await
            .unwrap_or(crate::models::StorageUsage {
                used_label: "0 GB".into(),
                total_label: "5 TB".into(),
                used_percent: 0,
            }),
        audit_log: load_blob(pool, "audit_log").await.unwrap_or_default(),
        credentials: load_blob(pool, "credentials").await.unwrap_or_default(),
        // Older saves predate the dual-token refactor and have `sessions` as
        // a bare `token -> email` map rather than `token -> SessionEntry`
        // (`{ email, expires_at }`). `load_blob_lenient` swallows that
        // deserialize failure so an old save doesn't crash the server on
        // startup; those sessions are simply dropped and every existing
        // browser session will need to log in again, which is an acceptable
        // one-time cost for closing a real security gap (tokens that used to
        // never expire). `refresh_tokens` is an entirely new field with no
        // legacy shape to fall back from, so a plain missing-key default
        // (empty map) is enough.
        sessions: load_blob_lenient(pool, "sessions")
            .await
            .unwrap_or_default(),
        refresh_tokens: load_blob(pool, "refresh_tokens").await.unwrap_or_default(),
        // Brand-new fields with no legacy on-disk shape — plain `load_blob`
        // (not `load_blob_lenient`) is fine since there's no old shape to
        // fall back from.
        invites: load_blob(pool, "invites").await.unwrap_or_default(),
        password_resets: load_blob(pool, "password_resets").await.unwrap_or_default(),
    })
}

pub async fn save_all(pool: &SqlitePool, state: &AppState) {
    // `repositories` (and, historically, `seeded_repo_slugs`) are
    // deliberately NOT saved here anymore — that data lives in the real SQL
    // `repositories` table (see `repo_store.rs`), written directly by
    // `repo_store::create`/`update`/`delete` as each mutation happens, not
    // batched through this whole-`AppState` blob dump.
    save_blob(pool, "tree", &state.tree).await;
    save_blob(pool, "file_contents", &state.file_contents).await;
    save_blob(pool, "image_content", &state.image_content).await;
    save_blob(pool, "image_content_before", &state.image_content_before).await;
    save_blob(pool, "audio_content", &state.audio_content).await;
    save_blob(pool, "commits", &state.commits).await;
    save_blob(pool, "branches", &state.branches).await;
    save_blob(pool, "current_branch", &state.current_branch).await;
    save_blob(pool, "pending_changes", &state.pending_changes).await;
    save_blob(pool, "pull_requests", &state.pull_requests).await;
    save_blob(pool, "access_entries", &state.access_entries).await;
    save_blob(pool, "org_members", &state.org_members).await;
    save_blob(pool, "storage", &state.storage).await;
    save_blob(pool, "audit_log", &state.audit_log).await;
    save_blob(pool, "credentials", &state.credentials).await;
    save_blob(pool, "sessions", &state.sessions).await;
    save_blob(pool, "refresh_tokens", &state.refresh_tokens).await;
    save_blob(pool, "invites", &state.invites).await;
    save_blob(pool, "password_resets", &state.password_resets).await;
}

/// Gets the SQL `repositories` table (see `repo_store.rs`) into a valid
/// starting state on startup, exactly once. Must be called after migrations
/// have run and before [`load_state`]/`state::seed()` (see `main`) — `main`
/// calls this unconditionally on every startup; it's a no-op whenever the
/// table already has rows (an already-migrated or already-running real
/// instance), so it never overwrites existing data.
///
/// Two first-time cases:
/// - **Pre-Phase-2 `lorehub.db`**: the old `kv_store` still has a
///   `"repositories"` blob (the pre-this-redesign bare-`Vec<Repository>`
///   shape). Its identity fields (slug/name/organization/description/
///   visibility) are imported as-is; every imported repo is marked
///   `is_seeded = true`; `size_label`/`locked_file_count`/`updated_at`
///   likewise carry over from the old blob (they're just display strings/
///   counters, not something this phase recomputes). This is a one-time
///   identity migration — none of the pre-Phase-2 `tree`/`commits`/`branches`
///   blobs are touched here at all (those still load through the existing
///   `kv_store` path in [`load_state`]).
/// - **Genuinely fresh install**: no `repositories` blob either. Seeds the
///   same 6 demo repositories `state::seed()` has always built in memory
///   (`state::demo_repositories()`), also marked `is_seeded = true` — same
///   semantics as the old `AppState.seeded_repo_slugs` every one of those six
///   slugs used to be a member of.
pub async fn migrate_or_seed_repositories(pool: &SqlitePool) {
    if !crate::repo_store::list(pool).await.is_empty() {
        return;
    }

    let created_at = crate::auth::current_unix_time();

    let source: Vec<crate::models::Repository> =
        match load_blob::<Vec<crate::models::Repository>>(pool, "repositories").await {
            Some(legacy) => legacy,
            None => crate::state::demo_repositories(),
        };

    for repo in source {
        crate::repo_store::create(
            pool,
            crate::repo_store::NewRepository {
                slug: repo.slug,
                name: repo.name,
                organization: repo.organization,
                description: repo.description,
                visibility: repo.visibility,
                is_seeded: true,
                size_label: repo.size_label,
                locked_file_count: repo.locked_file_count,
                updated_at: repo.updated_at,
                created_at,
            },
        )
        .await;
    }
}
