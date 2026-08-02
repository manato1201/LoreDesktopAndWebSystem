//! Covers `GET /metrics` (see `main.rs::build_router`'s `/metrics` route
//! and `metrics_layer_and_handle`), the Prometheus scrape endpoint added
//! for operational visibility into real production traffic.
//!
//! Asserts two things together: the response is genuinely Prometheus
//! text-exposition format (not just "some 200 response" — checked against
//! `axum-prometheus` 0.10.1's real output shape, confirmed by running the
//! server locally and curling `/metrics`, rather than assumed), and it
//! actually reflects request traffic that flowed through *other* routes on
//! this same router — proving the instrumentation is wired into the
//! router's real request path, not just serving a hardcoded snapshot.
//!
//! `axum-prometheus`'s underlying `metrics_exporter_prometheus::Recorder`
//! installs itself as the process-global `metrics` recorder exactly once
//! (see `metrics_layer_and_handle`'s doc comment in `main.rs`), so every
//! router built anywhere in this test binary — across every test file, not
//! just this one — shares and accumulates into the same counter. That's
//! why this test asserts the count for its own requests is "at least N",
//! not an exact value: other tests hitting `/api/health` concurrently would
//! otherwise make an exact-count assertion flaky.
use axum::http::StatusCode;

use super::{body_bytes, plain_request, send, test_app};

#[tokio::test]
async fn metrics_endpoint_reports_prometheus_format_and_reflects_prior_requests() {
    let app = test_app().await;

    // Hit an unrelated, already-existing endpoint a few times first, so
    // there's traffic on the router for `/metrics` to have picked up.
    const HEALTH_REQUESTS: u32 = 3;
    for _ in 0..HEALTH_REQUESTS {
        let req = plain_request("GET", "/api/health", None);
        let resp = send(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    let req = plain_request("GET", "/metrics", None);
    let resp = send(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = body_bytes(resp).await;
    let body = String::from_utf8(bytes).expect("metrics body is valid utf8");

    // Prometheus text-exposition format: every metric family is preceded by
    // a `# TYPE <name> <kind>` comment line before its samples.
    // `axum-prometheus` emits this for its request counter,
    // `axum_http_requests_total` — real output observed by running the
    // server locally and curling `/metrics` looks like:
    //   # TYPE axum_http_requests_total counter
    //   axum_http_requests_total{method="GET",status="200",endpoint="/api/health"} 3
    assert!(
        body.contains("# TYPE axum_http_requests_total counter"),
        "expected a Prometheus TYPE line for the request counter, got:\n{body}"
    );

    // Find the sample line for our own GET /api/health, 200 traffic and
    // confirm its count reflects (at least) the requests made above.
    let health_sample = body
        .lines()
        .find(|line| {
            line.starts_with("axum_http_requests_total{")
                && line.contains("method=\"GET\"")
                && line.contains("status=\"200\"")
                && line.contains("endpoint=\"/api/health\"")
        })
        .unwrap_or_else(|| {
            panic!(
                "expected an axum_http_requests_total sample for GET /api/health 200, got:\n{body}"
            )
        });

    let count: u64 = health_sample
        .rsplit(' ')
        .next()
        .expect("sample line has a trailing count value")
        .parse()
        .unwrap_or_else(|err| {
            panic!("sample count is not a valid integer: {err}\nline: {health_sample}")
        });

    assert!(
        count >= u64::from(HEALTH_REQUESTS),
        "expected the /api/health counter to be at least {HEALTH_REQUESTS} (this test's own \
         requests), got {count} — line: {health_sample}"
    );
}
