//! Integration test support: builds a fully independent app instance per
//! test (fresh `state::seed()` + a private SQLite in-memory database) and
//! drives it in-process via `tower::ServiceExt::oneshot`, i.e. real
//! `http::Request`/`http::Response` values flow through the exact same
//! `Router` (routing, CORS layer, `require_auth` middleware, handlers) that
//! `main.rs` binds to a TCP listener — no real socket, no shared state
//! across tests, and critically: never opens `lorehub.db`, the real file
//! `cargo run` reads/writes (see `test_app` below).
use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, Response, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::state::{AppContext, SharedState};
use crate::{RouterConfig, build_router, db, state};

/// Simulated peer address stamped onto every request built by
/// [`json_request`]/[`plain_request`] via a `ConnectInfo<SocketAddr>`
/// extension — `tower_governor`'s `PeerIpKeyExtractor` (used by the login
/// rate limiter, see `main.rs::build_router`) reads its key from that
/// extension. In production this is populated by
/// `into_make_service_with_connect_info` from the real TCP peer; the
/// in-process `oneshot` requests these tests build never touch a real
/// socket, so there's no "real" peer address to read — this stands in for
/// it. Tests that specifically need to simulate *distinct* source IPs (the
/// rate-limit tests) use [`login_request_from`] to override it.
pub const DEFAULT_TEST_PEER_ADDR: SocketAddr = SocketAddr::new(
    std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
    0,
);

mod access_control;
mod auth;
mod authz;
mod body_limit;
mod rate_limit;
mod repositories;
mod upload_and_range;
mod vcs;

/// Demo account seeded by `state::seed()` — every seeded account shares the
/// password `"lorehub"` (see `state.rs`).
pub const DEMO_EMAIL: &str = "aiko.tanaka@nebula.studio";
pub const DEMO_PASSWORD: &str = "lorehub";

/// A repository slug present in `state::seed()`'s demo dataset.
pub const SEEDED_SLUG: &str = "starforge-vfx";
/// A second, distinct seeded slug — used by tests asserting per-repo
/// isolation (uploads, etc.) don't leak across repositories.
pub const OTHER_SEEDED_SLUG: &str = "hollow-keep-env";

/// Builds a brand-new `Router` backed by its own `state::seed()` snapshot
/// and its own private SQLite database opened in in-memory mode
/// (`sqlite://:memory:` — verified against `sqlx-sqlite`'s own parser test
/// suite, `options/parse.rs::test_parse_in_memory`). Every test calls this
/// itself, so no two tests ever share an `AppState` or a database — safe
/// under `cargo test`'s default parallel execution, and the real dev
/// `lorehub.db` (opened only by `main()`, never by this helper) is never
/// touched.
pub async fn test_app() -> Router {
    test_app_with_config(RouterConfig::from_env()).await
}

/// Like [`test_app`], but with an explicit [`RouterConfig`] instead of the
/// env-derived default — used by tests that need a tiny body-size limit or
/// a tiny login rate-limit burst so the test runs fast, without mutating
/// any process-global env var (which would race every other test running
/// in the same `cargo test` process; see the module docs on
/// `DEFAULT_TEST_PEER_ADDR` and `RouterConfig` in `main.rs` for why that
/// matters here).
pub async fn test_app_with_config(config: RouterConfig) -> Router {
    let pool = db::connect(":memory:").await;
    let seeded = state::seed();
    let shared_state: SharedState = Arc::new(AppContext::new(seeded, pool));
    build_router(shared_state, config)
}

/// Drives `req` through `app` via `oneshot` and returns the response.
pub async fn send(app: &Router, req: Request<Body>) -> Response<Body> {
    app.clone()
        .oneshot(req)
        .await
        .expect("router is infallible")
}

/// Collects a response body and parses it as JSON.
pub async fn body_json(resp: Response<Body>) -> serde_json::Value {
    let bytes = resp
        .into_body()
        .collect()
        .await
        .expect("collect response body")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("response body is valid JSON")
}

/// Collects a response body as raw bytes.
pub async fn body_bytes(resp: Response<Body>) -> Vec<u8> {
    resp.into_body()
        .collect()
        .await
        .expect("collect response body")
        .to_bytes()
        .to_vec()
}

/// Builds a JSON request with an optional `Cookie` header. Carries a
/// `ConnectInfo<DEFAULT_TEST_PEER_ADDR>` extension so routes behind the
/// per-IP login rate limiter (`tower_governor`) can extract a key instead of
/// failing key-extraction outright — see `DEFAULT_TEST_PEER_ADDR`'s docs.
pub fn json_request(
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    body: serde_json::Value,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(c) = cookie {
        builder = builder.header(header::COOKIE, c);
    }
    let mut req = builder
        .body(Body::from(body.to_string()))
        .expect("build json request");
    req.extensions_mut()
        .insert(ConnectInfo(DEFAULT_TEST_PEER_ADDR));
    req
}

/// Builds a bodyless request (GET/DELETE/etc.) with an optional `Cookie`
/// header. See [`json_request`] for why a `ConnectInfo` extension is
/// stamped on by default.
pub fn plain_request(method: &str, uri: &str, cookie: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        builder = builder.header(header::COOKIE, c);
    }
    let mut req = builder.body(Body::empty()).expect("build plain request");
    req.extensions_mut()
        .insert(ConnectInfo(DEFAULT_TEST_PEER_ADDR));
    req
}

/// Builds a `POST /api/auth/login` request for `email` (password is always
/// `"lorehub"` for seeded accounts) carrying an explicit simulated peer
/// address, overriding [`json_request`]'s default `ConnectInfo`. Used by the
/// login rate-limit tests to simulate multiple distinct source IPs against
/// the same in-process router — there's no real TCP peer in an in-process
/// `oneshot` request, so this is the only way to vary "source IP" at all.
pub fn login_request_from(email: &str, addr: SocketAddr) -> Request<Body> {
    let mut req = json_request(
        "POST",
        "/api/auth/login",
        None,
        serde_json::json!({ "email": email, "password": DEMO_PASSWORD }),
    );
    req.extensions_mut().insert(ConnectInfo(addr));
    req
}

/// Every individual `Set-Cookie` header value on a response, in order.
pub fn set_cookies(resp: &Response<Body>) -> Vec<String> {
    resp.headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|v| {
            v.to_str()
                .expect("set-cookie header is valid utf8")
                .to_string()
        })
        .collect()
}

/// Splits a raw `Set-Cookie` header string into its `(name, value)` pair,
/// discarding attributes (`Path=...; HttpOnly; ...`).
pub fn cookie_pair(set_cookie: &str) -> (String, String) {
    let first = set_cookie
        .split(';')
        .next()
        .expect("non-empty cookie string");
    let (name, value) = first
        .split_once('=')
        .expect("set-cookie has a name=value pair");
    (name.to_string(), value.to_string())
}

/// Reads the `Max-Age` attribute off a raw `Set-Cookie` header string.
pub fn cookie_max_age(set_cookie: &str) -> i64 {
    set_cookie
        .split(';')
        .map(|part| part.trim())
        .find_map(|part| part.strip_prefix("Max-Age="))
        .expect("set-cookie has a Max-Age attribute")
        .parse()
        .expect("Max-Age is a valid integer")
}

/// Logs in as `email` (password is always `"lorehub"` for seeded accounts)
/// and returns `(combined "Cookie" header value, raw access Set-Cookie,
/// raw refresh Set-Cookie)` so callers can both attach the session to
/// further requests and independently inspect the two cookies' attributes.
pub async fn login(app: &Router, email: &str) -> (String, String, String) {
    let req = json_request(
        "POST",
        "/api/auth/login",
        None,
        serde_json::json!({ "email": email, "password": DEMO_PASSWORD }),
    );
    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "login should succeed");
    let cookies = set_cookies(&resp);
    assert_eq!(cookies.len(), 2, "login should set exactly two cookies");

    let access = cookies
        .iter()
        .find(|c| c.starts_with(&format!("{}=", crate::auth::SESSION_COOKIE)))
        .expect("access token cookie present")
        .clone();
    let refresh = cookies
        .iter()
        .find(|c| c.starts_with(&format!("{}=", crate::auth::REFRESH_COOKIE)))
        .expect("refresh token cookie present")
        .clone();

    let combined = cookies
        .iter()
        .map(|c| {
            let (name, value) = cookie_pair(c);
            format!("{name}={value}")
        })
        .collect::<Vec<_>>()
        .join("; ");

    (combined, access, refresh)
}

/// Convenience wrapper around [`login`] for tests that only need the
/// combined `Cookie` header value.
pub async fn login_cookie(app: &Router, email: &str) -> String {
    login(app, email).await.0
}
