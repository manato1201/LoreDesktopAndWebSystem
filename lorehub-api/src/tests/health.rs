//! Covers `GET /api/health` (see `handlers::health`), the trivial liveness
//! probe used by the Docker `HEALTHCHECK` (see `Dockerfile`). Asserts it's
//! reachable with no `Cookie` header at all — proving it needs no
//! authentication, unlike almost every other route in `protected_routes`.
use axum::http::StatusCode;

use super::{body_json, plain_request, send, test_app};

#[tokio::test]
async fn health_returns_ok_without_authentication() {
    let app = test_app().await;

    // No cookie passed at all — this must not require a session.
    let req = plain_request("GET", "/api/health", None);
    let resp = send(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body, serde_json::json!({ "status": "ok" }));
}
