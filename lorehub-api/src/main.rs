mod auth;
mod authz;
mod db;
mod email;
mod handlers;
mod image_assets;
mod models;
mod state;
#[cfg(test)]
mod tests;

use std::sync::Arc;
use std::time::Duration;

use axum::http::{HeaderValue, Method, header};
use axum::routing::{delete, get, patch, post};
use axum::{Router, middleware};
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

const DEFAULT_CORS_ORIGIN: &str = "http://localhost:3000";

/// 48 MiB. Uploads (`POST /api/repositories/{slug}/upload`) carry file
/// content as base64 inside the JSON body, so raw bytes are ~33% smaller
/// than this — see `docs/TECHNICAL_REFERENCE.md` §2.1 ("アップロードのボディ
/// サイズ上限に関する既知の制約") for why that encoding is itself a
/// pre-existing limitation for large binary assets, independent of this cap.
const DEFAULT_MAX_BODY_BYTES: usize = 48 * 1024 * 1024;

/// Allows a burst of `DEFAULT_LOGIN_RATE_LIMIT_BURST` login attempts per
/// source IP, then refills one attempt every `DEFAULT_LOGIN_RATE_LIMIT_PERIOD`
/// — i.e. a real user mistyping their password a handful of times in a row
/// is unaffected, while sustained credential-stuffing against one IP is
/// throttled hard. See `RouterConfig::from_env` / `build_router` for where
/// this is wired to `POST /api/auth/login` specifically.
const DEFAULT_LOGIN_RATE_LIMIT_BURST: u32 = 8;
const DEFAULT_LOGIN_RATE_LIMIT_PERIOD: Duration = Duration::from_secs(60);

/// Runtime configuration for [`build_router`] — everything here is either
/// read from an env var at startup (`RouterConfig::from_env`, used by
/// `main`) or constructed directly with explicit values (used by tests that
/// need a tiny body limit or a tiny rate-limit burst without mutating
/// process-global env vars, which would race other tests in the same
/// binary).
pub struct RouterConfig {
    pub cors_origin: HeaderValue,
    pub max_body_bytes: usize,
    pub login_rate_limit_burst: u32,
    pub login_rate_limit_period: Duration,
}

impl RouterConfig {
    /// Reads `LOREHUB_WEB_ORIGIN` and `LOREHUB_MAX_BODY_BYTES`, falling back
    /// to the documented defaults when unset. A value that IS set but fails
    /// to parse is a startup-time config error (panics with a clear
    /// message) rather than a silent fallback — see `resolve_cors_origin`/
    /// `resolve_max_body_bytes`.
    pub fn from_env() -> Self {
        Self {
            cors_origin: resolve_cors_origin(std::env::var("LOREHUB_WEB_ORIGIN").ok()),
            max_body_bytes: resolve_max_body_bytes(std::env::var("LOREHUB_MAX_BODY_BYTES").ok()),
            login_rate_limit_burst: DEFAULT_LOGIN_RATE_LIMIT_BURST,
            login_rate_limit_period: DEFAULT_LOGIN_RATE_LIMIT_PERIOD,
        }
    }
}

/// `raw` is `Some(..)` iff `LOREHUB_WEB_ORIGIN` was explicitly set. Takes an
/// `Option<String>` rather than reading the env var itself so it can be
/// unit-tested with explicit inputs instead of mutating process-global env
/// state.
fn resolve_cors_origin(raw: Option<String>) -> HeaderValue {
    let origin = raw.unwrap_or_else(|| DEFAULT_CORS_ORIGIN.to_string());
    origin.parse::<HeaderValue>().unwrap_or_else(|err| {
        panic!(
            "LOREHUB_WEB_ORIGIN is set to {origin:?}, which is not a valid HTTP header value: {err}"
        )
    })
}

/// See [`resolve_cors_origin`] for why this takes an `Option<String>`
/// instead of reading the env var directly.
fn resolve_max_body_bytes(raw: Option<String>) -> usize {
    match raw {
        None => DEFAULT_MAX_BODY_BYTES,
        Some(raw) => raw.parse::<usize>().unwrap_or_else(|err| {
            panic!(
                "LOREHUB_MAX_BODY_BYTES is set to {raw:?}, which is not a valid non-negative integer: {err}"
            )
        }),
    }
}

/// Builds the full `Router` (public + protected routes, CORS, tracing,
/// body-size limit, login rate limiting) bound to `shared_state` and
/// configured by `config`. Factored out of `main` so integration tests
/// (`src/tests/`) can drive the exact same routing/middleware stack via
/// `tower::ServiceExt::oneshot` against an isolated, in-memory-backed
/// `SharedState` instead of a real TCP listener.
pub fn build_router(shared_state: state::SharedState, config: RouterConfig) -> Router {
    // CORS origin is configurable via `LOREHUB_WEB_ORIGIN` (see
    // `RouterConfig::from_env`); defaults to the Next.js dev server.
    // `allow_credentials` requires an explicit origin (no `Any`) per the
    // CORS spec.
    let cors = CorsLayer::new()
        .allow_origin(config.cors_origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true);

    // Per-source-IP rate limiting on login specifically — not per-email,
    // since a per-email limiter would let an attacker lock a legitimate
    // user out just by spamming failed attempts against their address.
    // Requires the request to carry a `ConnectInfo<SocketAddr>` extension;
    // `main` wires that up via `into_make_service_with_connect_info`, and
    // tests insert it directly on the requests they build (see
    // `src/tests/mod.rs`).
    let login_rate_limit = GovernorConfigBuilder::default()
        .period(config.login_rate_limit_period)
        .burst_size(config.login_rate_limit_burst)
        .finish()
        .expect("login rate limit: burst_size and period must both be non-zero");
    let login_route = Router::new()
        .route("/api/auth/login", post(handlers::login))
        .layer(GovernorLayer::new(login_rate_limit));

    // A second, independent rate limiter (own `GovernorConfigBuilder`, own
    // governor instance/state — deliberately not shared with `login_route`'s)
    // scoped to `POST /api/auth/forgot-password` alone. Without this, an
    // attacker could email-bomb a victim's inbox with reset links by hitting
    // this endpoint repeatedly; the limit is on source IP just like login's,
    // for the same reason (a per-email limiter would let an attacker lock a
    // victim out of *requesting* their own reset).
    let forgot_password_rate_limit = GovernorConfigBuilder::default()
        .period(config.login_rate_limit_period)
        .burst_size(config.login_rate_limit_burst)
        .finish()
        .expect("forgot-password rate limit: burst_size and period must both be non-zero");
    let forgot_password_route = Router::new()
        .route("/api/auth/forgot-password", post(handlers::forgot_password))
        .layer(GovernorLayer::new(forgot_password_rate_limit));

    // `/api/auth/refresh` must stay public (no `require_auth` layer) — its
    // entire purpose is recovering from an *expired* access token, so it
    // can't itself require a valid one. It authenticates via the separate
    // refresh-token cookie instead (see handlers::refresh).
    //
    // `/api/auth/accept-invite`, `/api/auth/invite/{token}`, and
    // `/api/auth/reset-password` are public for the same underlying reason as
    // each other: the caller doesn't have a session yet (accept-invite/
    // invite-preview) or is deliberately proving identity via a one-time
    // token instead of a session (reset-password). `reset-password` itself
    // isn't separately rate-limited — the token is the secret, and
    // brute-forcing a 192-bit random token isn't practical.
    let public_routes = login_route
        .route("/api/health", get(handlers::health))
        .route("/api/auth/refresh", post(handlers::refresh))
        .route("/api/auth/accept-invite", post(handlers::accept_invite))
        .route("/api/auth/invite/{token}", get(handlers::get_invite))
        .route("/api/auth/reset-password", post(handlers::reset_password))
        .merge(forgot_password_route);

    // Everything else requires a valid session, including plain reads —
    // lorehub-web forwards the session cookie from Server Components (see
    // src/lib/auth-server.ts) so this stays transparent to the browser.
    let protected_routes = Router::new()
        .route("/api/auth/logout", post(handlers::logout))
        .route("/api/auth/me", get(handlers::me))
        .route("/api/auth/change-password", post(handlers::change_password))
        .route(
            "/api/repositories",
            get(handlers::list_repositories).post(handlers::create_repository),
        )
        .route(
            "/api/repositories/{slug}",
            get(handlers::get_repository)
                .patch(handlers::update_repository)
                .delete(handlers::delete_repository),
        )
        .route("/api/repositories/{slug}/tree", get(handlers::get_tree))
        .route(
            "/api/repositories/{slug}/tree/lock",
            post(handlers::toggle_lock),
        )
        .route(
            "/api/repositories/{slug}/content/{*path}",
            get(handlers::get_file_content),
        )
        .route(
            "/api/repositories/{slug}/image/{*path}",
            get(handlers::get_image),
        )
        .route(
            "/api/repositories/{slug}/image-before/{*path}",
            get(handlers::get_image_before),
        )
        .route(
            "/api/repositories/{slug}/upload",
            post(handlers::upload_file),
        )
        .route(
            "/api/repositories/{slug}/audio/{*path}",
            get(handlers::get_audio),
        )
        .route(
            "/api/repositories/{slug}/commits",
            get(handlers::list_commits).post(handlers::create_commit),
        )
        .route(
            "/api/repositories/{slug}/commits/{hash}",
            get(handlers::get_commit),
        )
        .route(
            "/api/repositories/{slug}/branches",
            get(handlers::list_branches).post(handlers::create_branch),
        )
        .route(
            "/api/repositories/{slug}/branches/current",
            get(handlers::get_current_branch),
        )
        .route(
            "/api/repositories/{slug}/checkout",
            post(handlers::checkout_branch),
        )
        .route(
            "/api/repositories/{slug}/pending",
            get(handlers::get_pending_changes),
        )
        .route(
            "/api/repositories/{slug}/tree/stage",
            post(handlers::stage_change),
        )
        .route("/api/pulls", get(handlers::list_pull_requests))
        .route("/api/pulls/{id}", get(handlers::get_pull_request))
        .route("/api/pulls/{id}/comments", post(handlers::add_comment))
        .route(
            "/api/access-control/entries",
            get(handlers::get_access_entries).put(handlers::apply_access_entries),
        )
        .route(
            "/api/access-control/entries/toggle",
            post(handlers::toggle_permission),
        )
        .route("/api/org/members", get(handlers::list_members))
        .route(
            "/api/org/members/{email}",
            patch(handlers::update_member_role),
        )
        .route(
            "/api/org/invites",
            get(handlers::list_invites).post(handlers::create_invite),
        )
        .route("/api/org/invites/{email}", delete(handlers::revoke_invite))
        .route("/api/org/storage", get(handlers::get_storage))
        .route("/api/org/audit-log", get(handlers::get_audit_log))
        .layer(middleware::from_fn_with_state(
            shared_state.clone(),
            handlers::require_auth,
        ));

    public_routes
        .merge(protected_routes)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        // Global request body size cap (configurable via
        // `LOREHUB_MAX_BODY_BYTES`) — without this, a client can send an
        // arbitrarily large JSON body (e.g. a base64-encoded upload) and
        // exhaust server memory. Returns `413 Payload Too Large` for
        // anything over the limit.
        .layer(RequestBodyLimitLayer::new(config.max_body_bytes))
        .with_state(shared_state)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let pool = db::connect("lorehub.db").await;
    let initial_state = match db::load_state(&pool).await {
        Some(state) => {
            tracing::info!("loaded persisted state from lorehub.db");
            state
        }
        None => {
            tracing::info!("no persisted state found — seeding lorehub.db");
            let seeded = state::seed();
            db::save_all(&pool, &seeded).await;
            seeded
        }
    };

    let shared_state: state::SharedState = Arc::new(state::AppContext::new(
        initial_state,
        pool,
        email::SmtpConfig::from_env(),
    ));
    let app = build_router(shared_state, RouterConfig::from_env());

    // `0.0.0.0`, not `127.0.0.1`: a loopback-only bind is unreachable from
    // outside its own network namespace, which is invisible in local `cargo
    // run` (host loopback already covers "this machine") but breaks
    // completely under Docker — the container's host-published port and any
    // other compose service reach it via the container's non-loopback
    // interface, not its 127.0.0.1 (see docker-compose.yml / Dockerfile).
    // The actual access boundary is CORS + session auth, not the bind
    // address; a real public deployment additionally sits behind a reverse
    // proxy (see docs/DEPLOYMENT.md), same as any other server.
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000")
        .await
        .expect("failed to bind to 0.0.0.0:4000");

    tracing::info!("lorehub-api listening on http://0.0.0.0:4000");
    // `into_make_service_with_connect_info` populates a `ConnectInfo<SocketAddr>`
    // extension on every incoming request with the real peer address — the
    // per-IP login rate limiter (see `build_router`) reads it via
    // `tower_governor`'s `PeerIpKeyExtractor`. Plain `axum::serve(listener, app)`
    // (the previous setup) never wired this, so the server couldn't see
    // client IPs at all.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .expect("server error");
}
