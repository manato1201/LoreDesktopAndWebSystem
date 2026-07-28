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

// ---------------------------------------------------------------------
// Text upload (generalized upload endpoint, kind inferred from extension)
// ---------------------------------------------------------------------

const TEXT_UPLOAD_PATH: &str = "notes/upload-test.txt";

fn text_upload_bytes() -> Vec<u8> {
    b"Hello from LoreForge Client!\nThis is real uploaded text content, not a demo fixture.\n"
        .to_vec()
}

async fn upload_test_text(app: &axum::Router, cookie: &str, slug: &str) {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(text_upload_bytes());
    let req = json_request(
        "POST",
        &format!("/api/repositories/{slug}/upload"),
        Some(cookie),
        serde_json::json!({ "path": TEXT_UPLOAD_PATH, "contentBase64": b64 }),
    );
    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
}

fn text_content_request(slug: &str, cookie: &str, range: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/repositories/{slug}/content/{TEXT_UPLOAD_PATH}"
        ))
        .header(header::COOKIE, cookie);
    if let Some(r) = range {
        builder = builder.header(header::RANGE, r);
    }
    builder.body(Body::empty()).unwrap()
}

#[tokio::test]
async fn upload_stores_real_text_bytes_and_get_file_content_returns_them_with_content_type() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    upload_test_text(&app, &cookie, SEEDED_SLUG).await;

    let resp = send(&app, text_content_request(SEEDED_SLUG, &cookie, None)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(content_type, "text/plain; charset=utf-8");

    let bytes = body_bytes(resp).await;
    assert_eq!(bytes, text_upload_bytes());
}

/// Same per-repo-isolation regression shape as the existing image test, now
/// for the uploaded-text store.
#[tokio::test]
async fn text_upload_on_one_repo_is_404_on_a_different_repo_at_the_same_path() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    upload_test_text(&app, &cookie, SEEDED_SLUG).await;

    let other_resp = send(&app, text_content_request(OTHER_SEEDED_SLUG, &cookie, None)).await;
    assert_eq!(other_resp.status(), StatusCode::NOT_FOUND);

    let own_resp = send(&app, text_content_request(SEEDED_SLUG, &cookie, None)).await;
    assert_eq!(own_resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn ranged_request_against_uploaded_text_returns_correct_slice() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    upload_test_text(&app, &cookie, SEEDED_SLUG).await;

    let full_body = text_upload_bytes();
    let ranged_resp = send(
        &app,
        text_content_request(SEEDED_SLUG, &cookie, Some("bytes=0-9")),
    )
    .await;
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

// ---------------------------------------------------------------------
// Audio upload (generalized upload endpoint, kind inferred from extension)
// ---------------------------------------------------------------------

const AUDIO_UPLOAD_PATH: &str = "audio/upload-test.wav";

/// 24 arbitrary, non-repeating bytes (distinct pattern from `upload_bytes`)
/// so Range-slice assertions can't accidentally pass on uniform padding.
fn audio_upload_bytes() -> Vec<u8> {
    (0u8..24)
        .map(|i| i.wrapping_mul(11).wrapping_add(5))
        .collect()
}

async fn upload_test_audio(app: &axum::Router, cookie: &str, slug: &str) {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(audio_upload_bytes());
    let req = json_request(
        "POST",
        &format!("/api/repositories/{slug}/upload"),
        Some(cookie),
        serde_json::json!({ "path": AUDIO_UPLOAD_PATH, "contentBase64": b64 }),
    );
    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
}

fn audio_request(slug: &str, cookie: &str, range: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder()
        .method("GET")
        .uri(format!(
            "/api/repositories/{slug}/audio/{AUDIO_UPLOAD_PATH}"
        ))
        .header(header::COOKIE, cookie);
    if let Some(r) = range {
        builder = builder.header(header::RANGE, r);
    }
    builder.body(Body::empty()).unwrap()
}

#[tokio::test]
async fn upload_stores_real_audio_bytes_and_get_audio_returns_them_with_content_type() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    upload_test_audio(&app, &cookie, SEEDED_SLUG).await;

    let resp = send(&app, audio_request(SEEDED_SLUG, &cookie, None)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(content_type, "audio/wav");

    let bytes = body_bytes(resp).await;
    assert_eq!(bytes, audio_upload_bytes());
}

/// Same per-repo-isolation regression shape as the existing image test, now
/// for the uploaded-audio store.
#[tokio::test]
async fn audio_upload_on_one_repo_is_404_on_a_different_repo_at_the_same_path() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    upload_test_audio(&app, &cookie, SEEDED_SLUG).await;

    let other_resp = send(&app, audio_request(OTHER_SEEDED_SLUG, &cookie, None)).await;
    assert_eq!(other_resp.status(), StatusCode::NOT_FOUND);

    let own_resp = send(&app, audio_request(SEEDED_SLUG, &cookie, None)).await;
    assert_eq!(own_resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn ranged_request_against_uploaded_audio_returns_correct_slice() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    upload_test_audio(&app, &cookie, SEEDED_SLUG).await;

    let full_body = audio_upload_bytes();
    let ranged_resp = send(
        &app,
        audio_request(SEEDED_SLUG, &cookie, Some("bytes=5-14")),
    )
    .await;
    assert_eq!(ranged_resp.status(), StatusCode::PARTIAL_CONTENT);
    let content_range = ranged_resp
        .headers()
        .get(header::CONTENT_RANGE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(content_range, format!("bytes 5-14/{}", full_body.len()));

    let ranged_body = body_bytes(ranged_resp).await;
    assert_eq!(ranged_body, &full_body[5..=14]);
}

#[tokio::test]
async fn audio_upload_content_type_infers_mp3_extension() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(audio_upload_bytes());
    let mp3_path = "audio/upload-test.mp3";
    let upload_req = json_request(
        "POST",
        &format!("/api/repositories/{SEEDED_SLUG}/upload"),
        Some(&cookie),
        serde_json::json!({ "path": mp3_path, "contentBase64": b64 }),
    );
    assert_eq!(send(&app, upload_req).await.status(), StatusCode::CREATED);

    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/api/repositories/{SEEDED_SLUG}/audio/{mp3_path}"))
        .header(header::COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let resp = send(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(content_type, "audio/mpeg");
}
