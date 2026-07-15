mod auth;
mod db;
mod handlers;
mod image_assets;
mod models;
mod state;

use std::sync::Arc;

use axum::http::{HeaderValue, Method, header};
use axum::routing::{get, patch, post};
use axum::{Router, middleware};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

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

    let shared_state: state::SharedState = Arc::new(state::AppContext::new(initial_state, pool));

    // Dev-only CORS scoped to the Next.js dev server. `allow_credentials`
    // requires an explicit origin (no `Any`) per the CORS spec.
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true);

    let public_routes = Router::new().route("/api/auth/login", post(handlers::login));

    // Everything else requires a valid session, including plain reads —
    // lorehub-web forwards the session cookie from Server Components (see
    // src/lib/auth-server.ts) so this stays transparent to the browser.
    let protected_routes = Router::new()
        .route("/api/auth/logout", post(handlers::logout))
        .route("/api/auth/me", get(handlers::me))
        .route("/api/repositories", get(handlers::list_repositories))
        .route("/api/repositories/{slug}", get(handlers::get_repository))
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
            "/api/repositories/{slug}/audio/{*path}",
            get(handlers::get_audio),
        )
        .route(
            "/api/repositories/{slug}/commits",
            get(handlers::list_commits),
        )
        .route(
            "/api/repositories/{slug}/commits/{hash}",
            get(handlers::get_commit),
        )
        .route(
            "/api/repositories/{slug}/branches",
            get(handlers::list_branches),
        )
        .route("/api/pulls", get(handlers::list_pull_requests))
        .route("/api/pulls/{id}", get(handlers::get_pull_request))
        .route("/api/pulls/{id}/comments", post(handlers::add_comment))
        .route(
            "/api/access-control/entries",
            get(handlers::get_access_entries),
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
        .route("/api/org/storage", get(handlers::get_storage))
        .route("/api/org/audit-log", get(handlers::get_audit_log))
        .layer(middleware::from_fn_with_state(
            shared_state.clone(),
            handlers::require_auth,
        ));

    let app = public_routes
        .merge(protected_routes)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:4000")
        .await
        .expect("failed to bind to 127.0.0.1:4000");

    tracing::info!("lorehub-api listening on http://127.0.0.1:4000");
    axum::serve(listener, app).await.expect("server error");
}
