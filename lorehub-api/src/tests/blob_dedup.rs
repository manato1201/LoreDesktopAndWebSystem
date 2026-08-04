//! Direct-SQL coverage for the content-addressed upload write path added in
//! this phase (see `blob_meta_store.rs`/`blob_store.rs`, wired into
//! `handlers::upload_file`): real dedup in `file_blobs` when identical bytes
//! land at two different paths, and `staged_content` correctly repointing
//! at a new hash (without deleting the old blob) when a path is re-uploaded
//! with different content.
use axum::http::StatusCode;
use sqlx::Row;

use super::{
    DEMO_EMAIL, SEEDED_SLUG, body_bytes, json_request, login_cookie, send, test_app_with_state,
};

async fn upload(app: &axum::Router, cookie: &str, slug: &str, path: &str, bytes: &[u8]) {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    let req = json_request(
        "POST",
        &format!("/api/repositories/{slug}/upload"),
        Some(cookie),
        serde_json::json!({ "path": path, "contentBase64": b64 }),
    );
    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
}

/// Uploading the exact same byte content to two different paths in the same
/// repo must produce exactly one `file_blobs` row for that
/// `(repo_slug, content_hash)` pair — the primary key is content-hash-only,
/// not path-scoped, so this is real dedup rather than incidental.
#[tokio::test]
async fn identical_content_at_two_paths_dedups_to_one_file_blobs_row() {
    let (app, ctx) = test_app_with_state().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let bytes = b"identical bytes uploaded to two different paths".to_vec();
    upload(&app, &cookie, SEEDED_SLUG, "dir-a/file.txt", &bytes).await;
    upload(&app, &cookie, SEEDED_SLUG, "dir-b/file-copy.txt", &bytes).await;

    let content_hash = crate::content_hash::sha256_hex(&bytes);

    let row = sqlx::query(
        "SELECT COUNT(*) as count FROM file_blobs WHERE repo_slug = ?1 AND content_hash = ?2",
    )
    .bind(SEEDED_SLUG)
    .bind(&content_hash)
    .fetch_one(&ctx.db)
    .await
    .expect("query file_blobs");
    let count: i64 = row.get("count");
    assert_eq!(
        count, 1,
        "identical content uploaded to two paths should produce exactly one file_blobs row"
    );

    // Both paths should independently have a staged_content row pointing at
    // the same content hash.
    let staged_rows = sqlx::query(
        "SELECT path FROM staged_content WHERE repo_slug = ?1 AND content_hash = ?2 ORDER BY path",
    )
    .bind(SEEDED_SLUG)
    .bind(&content_hash)
    .fetch_all(&ctx.db)
    .await
    .expect("query staged_content");
    let paths: Vec<String> = staged_rows.iter().map(|r| r.get("path")).collect();
    assert_eq!(paths, vec!["dir-a/file.txt", "dir-b/file-copy.txt"]);
}

/// Re-uploading to the same path with DIFFERENT content must repoint
/// `staged_content` at the new hash (the read path returns the new bytes),
/// while the old blob's `file_blobs` row and on-disk bytes are left alone —
/// content-addressed storage keeps every version around, deliberately, even
/// once no path currently points at it.
#[tokio::test]
async fn reuploading_same_path_with_different_content_updates_staged_content_without_deleting_old_blob()
 {
    let (app, ctx) = test_app_with_state().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let path = "notes/versioned.txt";
    let first_bytes = b"version one of the content".to_vec();
    let second_bytes = b"version two - completely different content".to_vec();

    upload(&app, &cookie, SEEDED_SLUG, path, &first_bytes).await;
    let first_hash = crate::content_hash::sha256_hex(&first_bytes);

    upload(&app, &cookie, SEEDED_SLUG, path, &second_bytes).await;
    let second_hash = crate::content_hash::sha256_hex(&second_bytes);

    assert_ne!(first_hash, second_hash);

    // staged_content now points at the new hash, not the old one.
    let row =
        sqlx::query("SELECT content_hash FROM staged_content WHERE repo_slug = ?1 AND path = ?2")
            .bind(SEEDED_SLUG)
            .bind(path)
            .fetch_one(&ctx.db)
            .await
            .expect("query staged_content");
    let current_hash: String = row.get("content_hash");
    assert_eq!(current_hash, second_hash);

    // The old blob's file_blobs row is still present — nothing was deleted.
    let old_blob_row = sqlx::query(
        "SELECT COUNT(*) as count FROM file_blobs WHERE repo_slug = ?1 AND content_hash = ?2",
    )
    .bind(SEEDED_SLUG)
    .bind(&first_hash)
    .fetch_one(&ctx.db)
    .await
    .expect("query file_blobs for old hash");
    let old_blob_count: i64 = old_blob_row.get("count");
    assert_eq!(
        old_blob_count, 1,
        "the old blob's file_blobs row should still exist after re-upload"
    );

    // And the new blob's file_blobs row exists too.
    let new_blob_row = sqlx::query(
        "SELECT COUNT(*) as count FROM file_blobs WHERE repo_slug = ?1 AND content_hash = ?2",
    )
    .bind(SEEDED_SLUG)
    .bind(&second_hash)
    .fetch_one(&ctx.db)
    .await
    .expect("query file_blobs for new hash");
    let new_blob_count: i64 = new_blob_row.get("count");
    assert_eq!(new_blob_count, 1);

    // Fetching the path now returns the new bytes, confirming the read path
    // (get_file_content -> resolve_uploaded_content) actually follows the
    // updated staged_content pointer.
    let get_req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/api/repositories/{SEEDED_SLUG}/content/{path}"))
        .header(axum::http::header::COOKIE, &cookie)
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = send(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let returned_bytes = body_bytes(resp).await;
    assert_eq!(returned_bytes, second_bytes);
}
