//! SQL-backed CRUD for the `repositories` table (see
//! `migrations/0001_vcs_schema.sql`).
//!
//! This replaces the old in-memory `AppState.repositories: Vec<Repository>` /
//! `AppState.seeded_repo_slugs: HashSet<String>` pair — see `state.rs` (which
//! no longer carries either field) and `db::migrate_or_seed_repositories`
//! (which is what actually gets the first rows into this table, either by
//! importing a pre-Phase-2 `kv_store` blob or by seeding fresh demo data).
//!
//! No `AppState` lock is needed for anything in this module: the
//! `repositories` table is the source of truth on its own, read/written
//! directly against the `SqlitePool` (`ctx.db`).
//!
//! Row <-> `Repository` mapping is done by hand (`sqlx::Row::get`, matching
//! the rest of this codebase's existing `db.rs` style) rather than via
//! `sqlx::query_as!`/`#[derive(sqlx::FromRow)]` — those require the `macros`
//! Cargo feature, which isn't enabled (see `Cargo.toml`), and pulling it in
//! for one table isn't worth the extra dependency surface. `Visibility`
//! similarly gets plain `&str` conversion helpers below instead of a
//! `sqlx::Type` impl, for the same reason.

use std::collections::HashSet;

use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use crate::models::{Repository, Visibility};

fn visibility_to_str(v: Visibility) -> &'static str {
    match v {
        Visibility::Private => "private",
        Visibility::Internal => "internal",
        Visibility::Public => "public",
    }
}

/// Inverse of [`visibility_to_str`]. The `visibility` column has a `CHECK
/// (visibility IN ('private','internal','public'))` constraint (see the
/// migration), so any row that made it into the table is guaranteed to hold
/// one of these three values — an unrecognized value here would mean the
/// schema constraint itself was bypassed, which `unreachable!` reflects
/// rather than silently defaulting to some guessed variant.
fn visibility_from_str(s: &str) -> Visibility {
    match s {
        "private" => Visibility::Private,
        "internal" => Visibility::Internal,
        "public" => Visibility::Public,
        other => unreachable!("repositories.visibility CHECK constraint allowed {other:?}"),
    }
}

fn row_to_repository(row: &SqliteRow) -> Repository {
    Repository {
        slug: row.get("slug"),
        name: row.get("name"),
        organization: row.get("organization"),
        description: row.get("description"),
        updated_at: row.get("updated_at"),
        size_label: row.get("size_label"),
        locked_file_count: row.get::<i64, _>("locked_file_count") as u32,
        visibility: visibility_from_str(&row.get::<String, _>("visibility")),
    }
}

/// `SELECT * FROM repositories ORDER BY created_at` — stable, predictable
/// ordering matching the old `Vec`-backed store's insertion order (repos are
/// only ever inserted with a fresh, increasing `created_at`, and never
/// reordered in place).
pub async fn list(pool: &SqlitePool) -> Vec<Repository> {
    sqlx::query("SELECT * FROM repositories ORDER BY created_at")
        .fetch_all(pool)
        .await
        .expect("query repositories")
        .iter()
        .map(row_to_repository)
        .collect()
}

pub async fn get(pool: &SqlitePool, slug: &str) -> Option<Repository> {
    sqlx::query("SELECT * FROM repositories WHERE slug = ?1")
        .bind(slug)
        .fetch_optional(pool)
        .await
        .expect("query repository")
        .as_ref()
        .map(row_to_repository)
}

/// Needed by every other VCS handler's "does this repo exist" guard before
/// touching tree/commits/branches/uploads/pending/pull-requests for a slug.
pub async fn exists(pool: &SqlitePool, slug: &str) -> bool {
    sqlx::query("SELECT 1 FROM repositories WHERE slug = ?1")
        .bind(slug)
        .fetch_optional(pool)
        .await
        .expect("query repository existence")
        .is_some()
}

/// Every slug currently marked `is_seeded = 1` — i.e. repositories that
/// should get a clone of the shared demo tree/commit/branch dataset (see
/// `state::seeded_tree`/`seeded_commits`/`seeded_branches`). Mirrors what the
/// old in-memory `AppState.seeded_repo_slugs` tracked; used by
/// `db::load_state`'s lenient tree/commits/branches fallback.
pub async fn seeded_slugs(pool: &SqlitePool) -> HashSet<String> {
    sqlx::query("SELECT slug FROM repositories WHERE is_seeded = 1")
        .fetch_all(pool)
        .await
        .expect("query seeded repository slugs")
        .iter()
        .map(|row| row.get::<String, _>("slug"))
        .collect()
}

/// Every field needed to insert a new `repositories` row. Used both by
/// `handlers::create_repository` (a real user-created repo: `is_seeded =
/// false`) and by `db::migrate_or_seed_repositories` (demo/imported repos:
/// `is_seeded = true`) — one insert path for both callers rather than two
/// slightly-different ones.
pub struct NewRepository {
    pub slug: String,
    pub name: String,
    pub organization: String,
    pub description: String,
    pub visibility: Visibility,
    pub is_seeded: bool,
    pub size_label: String,
    pub locked_file_count: u32,
    pub updated_at: String,
    pub created_at: i64,
}

pub async fn create(pool: &SqlitePool, new: NewRepository) -> Repository {
    sqlx::query(
        "INSERT INTO repositories
            (slug, name, organization, description, visibility, is_seeded,
             size_label, locked_file_count, updated_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )
    .bind(&new.slug)
    .bind(&new.name)
    .bind(&new.organization)
    .bind(&new.description)
    .bind(visibility_to_str(new.visibility))
    .bind(new.is_seeded)
    .bind(&new.size_label)
    .bind(new.locked_file_count as i64)
    .bind(&new.updated_at)
    .bind(new.created_at)
    .execute(pool)
    .await
    .expect("insert repository");

    Repository {
        slug: new.slug,
        name: new.name,
        organization: new.organization,
        description: new.description,
        updated_at: new.updated_at,
        size_label: new.size_label,
        locked_file_count: new.locked_file_count,
        visibility: new.visibility,
    }
}

/// Partial update, mirroring `handlers::UpdateRepositoryRequest`: every field
/// left `None` is left untouched.
#[derive(Default)]
pub struct RepositoryUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub visibility: Option<Visibility>,
}

/// Applies `patch` to the repository at `slug` and stamps `updated_at`.
/// Returns `None` (no-op, nothing written) if `slug` doesn't exist.
pub async fn update(
    pool: &SqlitePool,
    slug: &str,
    patch: RepositoryUpdate,
    updated_at: &str,
) -> Option<Repository> {
    let mut repo = get(pool, slug).await?;

    if let Some(name) = patch.name {
        repo.name = name;
    }
    if let Some(description) = patch.description {
        repo.description = description;
    }
    if let Some(visibility) = patch.visibility {
        repo.visibility = visibility;
    }
    repo.updated_at = updated_at.to_string();

    sqlx::query(
        "UPDATE repositories
         SET name = ?1, description = ?2, visibility = ?3, updated_at = ?4
         WHERE slug = ?5",
    )
    .bind(&repo.name)
    .bind(&repo.description)
    .bind(visibility_to_str(repo.visibility))
    .bind(&repo.updated_at)
    .bind(slug)
    .execute(pool)
    .await
    .expect("update repository");

    Some(repo)
}

/// Deletes the repository row at `slug`. Thanks to every VCS table's
/// `repo_slug` foreign key being declared `ON DELETE CASCADE` (see the
/// migration) and `PRAGMA foreign_keys = ON` being set per-connection (see
/// `db::connect`), this single statement atomically removes every related
/// row across every VCS table too. Returns whether a row was actually
/// deleted.
pub async fn delete(pool: &SqlitePool, slug: &str) -> bool {
    let result = sqlx::query("DELETE FROM repositories WHERE slug = ?1")
        .bind(slug)
        .execute(pool)
        .await
        .expect("delete repository");
    result.rows_affected() > 0
}
