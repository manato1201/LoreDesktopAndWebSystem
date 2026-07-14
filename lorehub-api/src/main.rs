mod handlers;
mod models;
mod state;

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, patch, post};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let shared_state: state::SharedState = Arc::new(RwLock::new(state::seed()));

    // Dev-only permissive CORS so the Next.js dev server (a different
    // origin/port) can call this API. Tighten this before any real
    // deployment.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/repositories", get(handlers::list_repositories))
        .route("/api/repositories/{slug}", get(handlers::get_repository))
        .route("/api/repositories/{slug}/tree", get(handlers::get_tree))
        .route(
            "/api/repositories/{slug}/tree/lock",
            post(handlers::toggle_lock),
        )
        .route(
            "/api/repositories/{slug}/commits",
            get(handlers::list_commits),
        )
        .route(
            "/api/repositories/{slug}/commits/{hash}",
            get(handlers::get_commit),
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
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:4000")
        .await
        .expect("failed to bind to 127.0.0.1:4000");

    tracing::info!("lorehub-api listening on http://127.0.0.1:4000");
    axum::serve(listener, app).await.expect("server error");
}
