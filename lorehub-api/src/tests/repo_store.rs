//! Tests for the SQL-backed `repositories` table wiring added in this phase
//! (see `repo_store.rs`) that go beyond what `tests/repositories.rs` already
//! covers via plain HTTP behavior: slug-collision handling now backed by
//! SQL, a direct-SQL check that `DELETE` really removes the row, and a
//! direct-SQL proof that the `ON DELETE CASCADE` foreign keys (see
//! `migrations/0001_vcs_schema.sql`) actually fire through the real
//! `DELETE FROM repositories` handlers now issue (see
//! `handlers::delete_repository`).
use axum::http::StatusCode;
use sqlx::Row;

use super::{
    DEMO_EMAIL, body_json, json_request, login_cookie, plain_request, send, test_app,
    test_app_with_state,
};

#[tokio::test]
async fn colliding_slugified_names_get_distinct_slugs() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let make_request = || {
        json_request(
            "POST",
            "/api/repositories",
            Some(&cookie),
            serde_json::json!({
                "name": "Collision Repo",
                "description": "collision test",
                "visibility": "private",
            }),
        )
    };

    let first_resp = send(&app, make_request()).await;
    assert_eq!(first_resp.status(), StatusCode::CREATED);
    let first_slug = body_json(first_resp).await["slug"]
        .as_str()
        .expect("slug is a string")
        .to_string();

    let second_resp = send(&app, make_request()).await;
    assert_eq!(second_resp.status(), StatusCode::CREATED);
    let second_slug = body_json(second_resp).await["slug"]
        .as_str()
        .expect("slug is a string")
        .to_string();

    assert_eq!(first_slug, "collision-repo");
    assert_eq!(second_slug, "collision-repo-2");
    assert_ne!(first_slug, second_slug);
}

#[tokio::test]
async fn delete_removes_the_row_from_the_repositories_table() {
    let (app, ctx) = test_app_with_state().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let delete_resp = send(
        &app,
        plain_request("DELETE", "/api/repositories/starforge-vfx", Some(&cookie)),
    )
    .await;
    assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

    let row = sqlx::query("SELECT COUNT(*) as count FROM repositories WHERE slug = ?1")
        .bind("starforge-vfx")
        .fetch_one(&ctx.db)
        .await
        .expect("query repositories");
    let count: i64 = row.get("count");
    assert_eq!(count, 0, "repositories row should be gone after DELETE");
}

/// Proves the `ON DELETE CASCADE` FK mechanism (see migrations/
/// 0001_vcs_schema.sql) actually fires end-to-end through `DELETE
/// /api/repositories/{slug}` — independent of whether any handler populates
/// `branches` yet in this phase (none do; that's Phase 4). A throwaway row is
/// inserted directly against a real repo's slug, and must disappear once
/// that repo is deleted.
#[tokio::test]
async fn deleting_a_repository_cascades_to_referencing_vcs_rows() {
    let (app, ctx) = test_app_with_state().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    sqlx::query(
        "INSERT INTO branches (repo_slug, name, head_commit_hash, is_default)
         VALUES (?1, 'throwaway', NULL, 0)",
    )
    .bind("starforge-vfx")
    .execute(&ctx.db)
    .await
    .expect("insert throwaway branches row");

    let before_row = sqlx::query("SELECT COUNT(*) as count FROM branches WHERE repo_slug = ?1")
        .bind("starforge-vfx")
        .fetch_one(&ctx.db)
        .await
        .expect("query branches before delete");
    let before_count: i64 = before_row.get("count");
    // As of Phase 4, every repository already has real branch rows (`main`
    // plus, for seeded repos, real feature branches from `vcs_seed` — see
    // `vcs_store::init_main_branch`/`vcs_seed::seed_demo_history`), so this
    // is no longer exactly 1 the way it was when `branches` was still an
    // empty, unused table (Phase 2/3). The throwaway row inserted above is
    // still definitely present (>= 1); what this test actually cares about
    // — the cascade wiping the table to 0 rows after delete — is unaffected
    // either way.
    assert!(
        before_count >= 1,
        "throwaway branches row should be present before delete"
    );

    let delete_resp = send(
        &app,
        plain_request("DELETE", "/api/repositories/starforge-vfx", Some(&cookie)),
    )
    .await;
    assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

    let after_row = sqlx::query("SELECT COUNT(*) as count FROM branches WHERE repo_slug = ?1")
        .bind("starforge-vfx")
        .fetch_one(&ctx.db)
        .await
        .expect("query branches after delete");
    let after_count: i64 = after_row.get("count");
    assert_eq!(
        after_count, 0,
        "branches row should have been cascade-deleted along with its repository"
    );
}
