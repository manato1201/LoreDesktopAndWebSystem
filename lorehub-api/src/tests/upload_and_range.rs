use axum::body::Body;
use axum::http::{Request, StatusCode, header};

use super::{
    DEMO_EMAIL, OTHER_SEEDED_SLUG, SEEDED_SLUG, body_bytes, json_request, login_cookie, send,
    test_app,
};

const UPLOAD_PATH: &str = "diffs/range-test.png";

/// Base64 of 20 arbitrary, non-repeating bytes, so Range-slice assertions
/// can't accidentally pass on a resource that's uniform padding.
fn upload_bytes() -> Vec<u8> {
    (0u8..20)
        .map(|i| i.wrapping_mul(7).wrapping_add(3))
        .collect()
}

async fn upload_test_file(app: &axum::Router, cookie: &str, slug: &str) {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(upload_bytes());
    let req = json_request(
        "POST",
        &format!("/api/repositories/{slug}/upload"),
        Some(cookie),
        serde_json::json!({ "path": UPLOAD_PATH, "contentBase64": b64 }),
    );
    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
}

fn image_request(slug: &str, cookie: &str, range: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder()
        .method("GET")
        .uri(format!("/api/repositories/{slug}/image/{UPLOAD_PATH}"))
        .header(header::COOKIE, cookie);
    if let Some(r) = range {
        builder = builder.header(header::RANGE, r);
    }
    builder.body(Body::empty()).unwrap()
}

#[tokio::test]
async fn upload_stores_real_bytes_and_get_image_returns_them_with_content_type() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    upload_test_file(&app, &cookie, SEEDED_SLUG).await;

    let resp = send(&app, image_request(SEEDED_SLUG, &cookie, None)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(content_type, "image/png");

    let bytes = body_bytes(resp).await;
    assert_eq!(bytes, upload_bytes());
}

/// Regression test for the per-repo-keying refactor: an upload to one repo
/// must not be visible (or leak a 200) from a *different* repo at the same
/// path.
#[tokio::test]
async fn upload_on_one_repo_is_404_on_a_different_repo_at_the_same_path() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    upload_test_file(&app, &cookie, SEEDED_SLUG).await;

    let other_resp = send(&app, image_request(OTHER_SEEDED_SLUG, &cookie, None)).await;
    assert_eq!(other_resp.status(), StatusCode::NOT_FOUND);

    // And the original repo still serves it correctly (isolation cuts both
    // ways — the other repo's 404 isn't from some accidental global wipe).
    let own_resp = send(&app, image_request(SEEDED_SLUG, &cookie, None)).await;
    assert_eq!(own_resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn upload_rejects_empty_path() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(upload_bytes());
    let resp = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/upload"),
            Some(&cookie),
            serde_json::json!({ "path": "   ", "contentBase64": b64 }),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn range_full_request_is_200_with_accept_ranges() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    upload_test_file(&app, &cookie, SEEDED_SLUG).await;

    let resp = send(&app, image_request(SEEDED_SLUG, &cookie, None)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().get(header::ACCEPT_RANGES).unwrap(), "bytes");
    let full_body = body_bytes(resp).await;
    assert_eq!(full_body, upload_bytes());
}

#[tokio::test]
async fn satisfiable_range_returns_206_with_correct_content_range_and_matching_bytes() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    upload_test_file(&app, &cookie, SEEDED_SLUG).await;

    // Full content first, to compute the expected slice independently of
    // the ranged response.
    let full_resp = send(&app, image_request(SEEDED_SLUG, &cookie, None)).await;
    let full_body = body_bytes(full_resp).await;

    let ranged_resp = send(&app, image_request(SEEDED_SLUG, &cookie, Some("bytes=0-9"))).await;
    assert_eq!(ranged_resp.status(), StatusCode::PARTIAL_CONTENT);
    let content_range = ranged_resp
        .headers()
        .get(header::CONTENT_RANGE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(content_range, format!("bytes 0-9/{}", full_body.len()));

    let ranged_body = body_bytes(ranged_resp).await;
    assert_eq!(ranged_body, &full_body[0..=9]);
}

#[tokio::test]
async fn out_of_bounds_range_is_416_with_content_range_total() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    upload_test_file(&app, &cookie, SEEDED_SLUG).await;

    let total = upload_bytes().len();
    let out_of_range = format!("bytes={}-{}", total + 100, total + 200);
    let resp = send(
        &app,
        image_request(SEEDED_SLUG, &cookie, Some(&out_of_range)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    let content_range = resp
        .headers()
        .get(header::CONTENT_RANGE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(content_range, format!("bytes */{total}"));
}

/// Not directly required by the checklist, but cheap and adjacent: confirms
/// a `Range` header is simply ignored (full `200`) when the client doesn't
/// send one — already covered above — versus when the header is present but
/// unparsable, which should also degrade to a full response rather than an
/// error.
#[tokio::test]
async fn unparsable_range_header_falls_back_to_full_content() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    upload_test_file(&app, &cookie, SEEDED_SLUG).await;

    let resp = send(
        &app,
        image_request(SEEDED_SLUG, &cookie, Some("bytes=not-a-number-")),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_bytes(resp).await;
    assert_eq!(body, upload_bytes());
}
