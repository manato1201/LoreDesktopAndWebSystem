//! Integration tests for `GET /api/repositories/{slug}/diff/{*path}
//! ?from=..&to=..` (see `handlers::get_diff`). Builds real commits through
//! the same upload -> stage -> commit path `tests/vcs.rs` uses, then diffs
//! between them — this endpoint is deliberately independent of the seeded
//! `PullRequest`/`PrDiffFile` machinery, so every fixture here is
//! self-contained rather than relying on seeded PR data.
use axum::http::StatusCode;

use super::{
    DEMO_EMAIL, SEEDED_SLUG, body_json, json_request, login_cookie, plain_request, send, test_app,
};

/// A seeded `Member` with no grant on "Assets/Characters" (see
/// `tests/authz.rs`'s fixture recap) — used by the ACL-denial test below.
const QA_CONTRACTOR_EMAIL: &str = "diego.fernandez@nebula.studio";

/// Base64-encodes `content` and POSTs it to `slug`'s upload endpoint,
/// asserting success. Mirrors `tests/vcs.rs`'s identically-named helper.
async fn upload(app: &axum::Router, cookie: &str, slug: &str, path: &str, content: &[u8]) {
    use base64::Engine;
    let resp = send(
        app,
        json_request(
            "POST",
            &format!("/api/repositories/{slug}/upload"),
            Some(cookie),
            serde_json::json!({
                "path": path,
                "contentBase64": base64::engine::general_purpose::STANDARD.encode(content),
            }),
        ),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "upload of {path} should succeed"
    );
}

/// Stages `path` with `change_type` ("added"/"modified"/"deleted") in
/// `slug`, asserting the stage call itself succeeded.
async fn stage(app: &axum::Router, cookie: &str, slug: &str, path: &str, change_type: &str) {
    let resp = send(
        app,
        json_request(
            "POST",
            &format!("/api/repositories/{slug}/tree/stage"),
            Some(cookie),
            serde_json::json!({ "path": path, "changeType": change_type, "staged": true }),
        ),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "staging {path} should succeed"
    );
}

/// Commits whatever is currently staged in `slug`, asserting the commit
/// succeeded, and returns the new commit's hash.
async fn commit(app: &axum::Router, cookie: &str, slug: &str, message: &str) -> String {
    let resp = send(
        app,
        json_request(
            "POST",
            &format!("/api/repositories/{slug}/commits"),
            Some(cookie),
            serde_json::json!({ "message": message }),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED, "commit should succeed");
    let body = body_json(resp).await;
    body["hash"]
        .as_str()
        .expect("commit hash present")
        .to_string()
}

fn diff_uri(slug: &str, path: &str, from: &str, to: &str) -> String {
    format!("/api/repositories/{slug}/diff/{path}?from={from}&to={to}")
}

/// The main happy-path case: a text file genuinely modified between two
/// commits produces a real, line-level diff — asserted against the exact
/// expected output (which line changed, its old/new line numbers), not just
/// "lines is non-empty".
#[tokio::test]
async fn modified_text_file_returns_line_level_diff() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    let path = "Diff/mod.txt";

    upload(
        &app,
        &cookie,
        SEEDED_SLUG,
        path,
        b"line one\nline two\nline three\n",
    )
    .await;
    stage(&app, &cookie, SEEDED_SLUG, path, "added").await;
    let commit_a = commit(&app, &cookie, SEEDED_SLUG, "add mod.txt").await;

    upload(
        &app,
        &cookie,
        SEEDED_SLUG,
        path,
        b"line one\nline TWO changed\nline three\nline four\n",
    )
    .await;
    stage(&app, &cookie, SEEDED_SLUG, path, "modified").await;
    let commit_b = commit(&app, &cookie, SEEDED_SLUG, "modify mod.txt").await;

    let resp = send(
        &app,
        plain_request(
            "GET",
            &diff_uri(SEEDED_SLUG, path, &commit_a, &commit_b),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    assert_eq!(body["path"], path);
    assert_eq!(body["fromHash"], commit_a);
    assert_eq!(body["toHash"], commit_b);
    assert_eq!(body["changeType"], "modified");
    assert_eq!(body["isBinary"], false);

    let lines = body["lines"]
        .as_array()
        .expect("lines present for a modified text diff");
    assert_eq!(lines.len(), 5, "unexpected diff shape: {lines:#?}");

    assert_eq!(lines[0]["type"], "context");
    assert_eq!(lines[0]["text"], "line one");
    assert_eq!(lines[0]["oldLine"], 1);
    assert_eq!(lines[0]["newLine"], 1);

    assert_eq!(lines[1]["type"], "remove");
    assert_eq!(lines[1]["text"], "line two");
    assert_eq!(lines[1]["oldLine"], 2);
    assert_eq!(lines[1]["newLine"], serde_json::Value::Null);

    assert_eq!(lines[2]["type"], "add");
    assert_eq!(lines[2]["text"], "line TWO changed");
    assert_eq!(lines[2]["oldLine"], serde_json::Value::Null);
    assert_eq!(lines[2]["newLine"], 2);

    assert_eq!(lines[3]["type"], "context");
    assert_eq!(lines[3]["text"], "line three");
    assert_eq!(lines[3]["oldLine"], 3);
    assert_eq!(lines[3]["newLine"], 3);

    assert_eq!(lines[4]["type"], "add");
    assert_eq!(lines[4]["text"], "line four");
    assert_eq!(lines[4]["oldLine"], serde_json::Value::Null);
    assert_eq!(lines[4]["newLine"], 4);
}

/// A path present at `to` but not at `from` is `"added"` — `lines` stays
/// `null` even though the plan permits a future full-file rendering, this
/// phase deliberately keeps it simple.
#[tokio::test]
async fn added_path_reports_added_with_null_lines() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    let path = "Diff/added.txt";

    // Baseline commit that does not touch `path` at all.
    upload(
        &app,
        &cookie,
        SEEDED_SLUG,
        "Diff/baseline.txt",
        b"unrelated\n",
    )
    .await;
    stage(&app, &cookie, SEEDED_SLUG, "Diff/baseline.txt", "added").await;
    let commit_a = commit(&app, &cookie, SEEDED_SLUG, "baseline commit").await;

    upload(&app, &cookie, SEEDED_SLUG, path, b"brand new file\n").await;
    stage(&app, &cookie, SEEDED_SLUG, path, "added").await;
    let commit_b = commit(&app, &cookie, SEEDED_SLUG, "add new file").await;

    let resp = send(
        &app,
        plain_request(
            "GET",
            &diff_uri(SEEDED_SLUG, path, &commit_a, &commit_b),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["changeType"], "added");
    assert_eq!(body["isBinary"], false);
    assert_eq!(body["lines"], serde_json::Value::Null);
}

/// A path present at `from` but not at `to` (staged+committed as deleted) is
/// `"deleted"` with `lines: null`.
#[tokio::test]
async fn deleted_path_reports_deleted_with_null_lines() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    let path = "Diff/deleted.txt";

    upload(&app, &cookie, SEEDED_SLUG, path, b"will be deleted\n").await;
    stage(&app, &cookie, SEEDED_SLUG, path, "added").await;
    let commit_a = commit(&app, &cookie, SEEDED_SLUG, "add file to delete").await;

    stage(&app, &cookie, SEEDED_SLUG, path, "deleted").await;
    let commit_b = commit(&app, &cookie, SEEDED_SLUG, "delete the file").await;

    let resp = send(
        &app,
        plain_request(
            "GET",
            &diff_uri(SEEDED_SLUG, path, &commit_a, &commit_b),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["changeType"], "deleted");
    assert_eq!(body["lines"], serde_json::Value::Null);
}

/// A path whose content hash is identical at both commits (carried forward
/// unmodified by an unrelated commit in between) is `"unchanged"`.
#[tokio::test]
async fn unchanged_path_reports_unchanged() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    let path = "Diff/unchanged.txt";
    let other = "Diff/other-for-unchanged.txt";

    upload(&app, &cookie, SEEDED_SLUG, path, b"never touched again\n").await;
    stage(&app, &cookie, SEEDED_SLUG, path, "added").await;
    let commit_a = commit(&app, &cookie, SEEDED_SLUG, "add unchanged file").await;

    // A second commit that touches a completely different path — `path`
    // carries forward into this commit's tree snapshot with the exact same
    // content hash.
    upload(&app, &cookie, SEEDED_SLUG, other, b"some other content\n").await;
    stage(&app, &cookie, SEEDED_SLUG, other, "added").await;
    let commit_b = commit(&app, &cookie, SEEDED_SLUG, "unrelated change").await;

    let resp = send(
        &app,
        plain_request(
            "GET",
            &diff_uri(SEEDED_SLUG, path, &commit_a, &commit_b),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["changeType"], "unchanged");
    assert_eq!(body["lines"], serde_json::Value::Null);
}

/// A binary-kind path (by extension — see `vcs_store::file_kind_for_path`)
/// reports `isBinary: true` and `lines: null` even when genuinely modified.
#[tokio::test]
async fn binary_file_reports_is_binary_with_null_lines_even_when_modified() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    let path = "Diff/texture.png";

    upload(&app, &cookie, SEEDED_SLUG, path, &[0u8, 1, 2, 3]).await;
    stage(&app, &cookie, SEEDED_SLUG, path, "added").await;
    let commit_a = commit(&app, &cookie, SEEDED_SLUG, "add binary").await;

    upload(&app, &cookie, SEEDED_SLUG, path, &[4u8, 5, 6, 7, 8]).await;
    stage(&app, &cookie, SEEDED_SLUG, path, "modified").await;
    let commit_b = commit(&app, &cookie, SEEDED_SLUG, "modify binary").await;

    let resp = send(
        &app,
        plain_request(
            "GET",
            &diff_uri(SEEDED_SLUG, path, &commit_a, &commit_b),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["changeType"], "modified");
    assert_eq!(body["isBinary"], true);
    assert_eq!(body["lines"], serde_json::Value::Null);
}

/// An unknown/invalid commit hash for either `from` or `to` is a 404 — not
/// silently treated as "path absent at that commit" (see `vcs_store::
/// commit_exists`'s doc comment for why this distinction matters).
#[tokio::test]
async fn unknown_commit_hash_is_404() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    let path = "Diff/whatever.txt";

    upload(&app, &cookie, SEEDED_SLUG, path, b"content\n").await;
    stage(&app, &cookie, SEEDED_SLUG, path, "added").await;
    let commit_a = commit(&app, &cookie, SEEDED_SLUG, "add whatever").await;

    let bogus = "0".repeat(64);

    let bad_to = send(
        &app,
        plain_request(
            "GET",
            &diff_uri(SEEDED_SLUG, path, &commit_a, &bogus),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(bad_to.status(), StatusCode::NOT_FOUND);

    let bad_from = send(
        &app,
        plain_request(
            "GET",
            &diff_uri(SEEDED_SLUG, path, &bogus, &commit_a),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(bad_from.status(), StatusCode::NOT_FOUND);
}

/// `from`/`to` are both required query parameters — missing either (or
/// both) is a 400, not a silent full-file diff or a panic.
#[tokio::test]
async fn missing_from_or_to_query_param_is_400() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    let path = "Diff/whatever.txt";

    upload(&app, &cookie, SEEDED_SLUG, path, b"content\n").await;
    stage(&app, &cookie, SEEDED_SLUG, path, "added").await;
    let commit_a = commit(&app, &cookie, SEEDED_SLUG, "add whatever").await;

    let missing_to = send(
        &app,
        plain_request(
            "GET",
            &format!("/api/repositories/{SEEDED_SLUG}/diff/{path}?from={commit_a}"),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(missing_to.status(), StatusCode::BAD_REQUEST);

    let missing_from = send(
        &app,
        plain_request(
            "GET",
            &format!("/api/repositories/{SEEDED_SLUG}/diff/{path}?to={commit_a}"),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(missing_from.status(), StatusCode::BAD_REQUEST);

    let missing_both = send(
        &app,
        plain_request(
            "GET",
            &format!("/api/repositories/{SEEDED_SLUG}/diff/{path}"),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(missing_both.status(), StatusCode::BAD_REQUEST);
}

/// Same auth gate as every other content-reading endpoint: no session
/// cookie at all is a 401.
#[tokio::test]
async fn diff_without_auth_cookie_is_401() {
    let app = test_app().await;

    let resp = send(
        &app,
        plain_request("GET", &diff_uri(SEEDED_SLUG, "README.md", "a", "b"), None),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// Same path-ACL gate as `get_file_content`/`get_image`/`get_audio`: a
/// member with no Read grant (directly or via team) on the path's ACL
/// ancestor is denied with 403 — checked before any commit-hash validation,
/// so bogus `from`/`to` values here are fine (never reached).
#[tokio::test]
async fn diff_denied_by_acl_is_403() {
    let app = test_app().await;
    let cookie = login_cookie(&app, QA_CONTRACTOR_EMAIL).await;

    let resp = send(
        &app,
        plain_request(
            "GET",
            &diff_uri(SEEDED_SLUG, "Assets/Characters/hero_diffuse.png", "a", "b"),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
