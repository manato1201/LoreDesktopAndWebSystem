//! Covers the per-source-IP login rate limiter (`tower_governor`, wired to
//! `POST /api/auth/login` specifically in `main.rs::build_router`).
//!
//! Uses `test_app_with_config` with a deliberately tiny burst (3 requests,
//! 60s refill period) instead of the real production default (8) so the
//! test can trip the limiter deterministically without waiting on real
//! time. Each test builds its own `test_app_with_config`, which constructs
//! a brand-new `tower_governor` rate limiter internally — so unlike a
//! hand-rolled global limiter, there is no shared state across tests (or
//! across the ~50 other tests in this suite that log in once each) to
//! worry about.
//!
//! Source IP is simulated via `login_request_from`, which stamps a
//! `ConnectInfo<SocketAddr>` extension directly onto the request —
//! `tower::ServiceExt::oneshot`'s in-process request model has no real
//! socket, so there's no genuine peer address to vary. This is the one
//! piece of test-plumbing needed to exercise `PeerIpKeyExtractor` at all.
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use axum::http::StatusCode;

use super::{DEMO_EMAIL, login_request_from, send, test_app_with_config};
use crate::RouterConfig;

const TEST_BURST: u32 = 3;

fn addr(last_octet: u8) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, last_octet)), 0)
}

fn tiny_rate_limit_config() -> RouterConfig {
    let mut config = RouterConfig::from_env();
    config.login_rate_limit_burst = TEST_BURST;
    // Long enough that the token bucket never partially refills mid-test —
    // keeps the assertion "exactly `TEST_BURST` requests succeed, then
    // every further one is blocked" deterministic.
    config.login_rate_limit_period = Duration::from_secs(60);
    config
}

#[tokio::test]
async fn nth_plus_one_login_attempt_from_same_source_ip_is_rate_limited() {
    let app = test_app_with_config(tiny_rate_limit_config()).await;
    let source = addr(1);

    for attempt in 1..=TEST_BURST {
        let resp = send(&app, login_request_from(DEMO_EMAIL, source)).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "attempt {attempt} of {TEST_BURST} should still be within the burst"
        );
    }

    // The (burst + 1)th rapid attempt from the same source IP must be
    // rejected with 429, not passed through to the login handler.
    let blocked = send(&app, login_request_from(DEMO_EMAIL, source)).await;
    assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn rate_limit_is_scoped_per_source_ip_not_global() {
    let app = test_app_with_config(tiny_rate_limit_config()).await;
    let source_a = addr(10);
    let source_b = addr(20);

    // Exhaust source A's burst.
    for _ in 0..TEST_BURST {
        let resp = send(&app, login_request_from(DEMO_EMAIL, source_a)).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
    let blocked = send(&app, login_request_from(DEMO_EMAIL, source_a)).await;
    assert_eq!(
        blocked.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "sanity check: source A's burst should now be exhausted"
    );

    // Source B has made zero requests — a per-IP limiter must not have
    // consumed any of its budget just because source A hit its own limit.
    // (This is also the reason this is a per-*IP*, not per-*email*, limiter
    // in the first place: both sources authenticate as the same demo
    // account here, so a per-email limiter would incorrectly block this.)
    let resp = send(&app, login_request_from(DEMO_EMAIL, source_b)).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "a different source IP must not be affected by another IP's exhausted burst"
    );
}
