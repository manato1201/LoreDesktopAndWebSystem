//! Covers `RequestBodyLimitLayer` (see `main.rs::build_router`), which caps
//! request body size to defend against a client sending an arbitrarily
//! large JSON body (e.g. a base64-encoded upload) to exhaust server memory.
//!
//! Uses `test_app_with_config` with a deliberately tiny `max_body_bytes`
//! (512 bytes) so the test runs fast and the pass/fail boundary is exact,
//! rather than exercising the real ~48 MiB production default (which would
//! mean allocating and sending tens of megabytes per test run).
//!
//! Both tests hit `POST /api/auth/login`: it's public (no session cookie
//! needed) and its `LoginRequest` extractor ignores unknown JSON fields, so
//! padding the body with an extra field lets us hit an exact byte count
//! while still using valid credentials — the "just under the limit" case
//! needs the login to actually succeed, not just avoid the body-limit
//! rejection.
use axum::http::StatusCode;

use super::{DEMO_EMAIL, DEMO_PASSWORD, json_request, send, test_app_with_config};
use crate::RouterConfig;

const TEST_MAX_BODY_BYTES: usize = 512;

fn tiny_body_limit_config() -> RouterConfig {
    let mut config = RouterConfig::from_env();
    config.max_body_bytes = TEST_MAX_BODY_BYTES;
    config
}

/// A valid login body padded with an extra (ignored) field so its
/// serialized size can be pushed to an exact target length.
fn login_body_with_padding(padding_len: usize) -> serde_json::Value {
    serde_json::json!({
        "email": DEMO_EMAIL,
        "password": DEMO_PASSWORD,
        "padding": "a".repeat(padding_len),
    })
}

#[tokio::test]
async fn oversized_request_body_is_rejected_with_413() {
    let app = test_app_with_config(tiny_body_limit_config()).await;

    let body = login_body_with_padding(2_000);
    assert!(
        body.to_string().len() > TEST_MAX_BODY_BYTES,
        "sanity check: padded body must actually exceed the configured limit"
    );

    let req = json_request("POST", "/api/auth/login", None, body);
    let resp = send(&app, req).await;
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn request_body_just_under_the_limit_still_succeeds() {
    let app = test_app_with_config(tiny_body_limit_config()).await;

    // No padding — the bare login body is comfortably under 512 bytes, and
    // must still be processed normally (rejecting oversized bodies must not
    // come at the cost of rejecting normal-sized ones).
    let body = login_body_with_padding(0);
    assert!(
        body.to_string().len() < TEST_MAX_BODY_BYTES,
        "sanity check: unpadded login body must already be under the test limit"
    );

    let req = json_request("POST", "/api/auth/login", None, body);
    let resp = send(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "a request comfortably under the body-size limit must still succeed"
    );
}
