use axum::Json;
use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::collections::HashMap;

use crate::auth;
use crate::models::*;
use crate::repo_store;
use crate::state::{SessionEntry, SharedState};

fn not_found(message: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": message })),
    )
        .into_response()
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": "unauthorized" })),
    )
        .into_response()
}

fn bad_request(message: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": message })),
    )
        .into_response()
}

fn forbidden(message: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({ "error": message })),
    )
        .into_response()
}

/// `PermissionLevel` -> the verb used in audit-log denial messages
/// ("denied {verb} access to ...") and the forbidden-response body.
fn permission_verb(level: PermissionLevel) -> &'static str {
    match level {
        PermissionLevel::Read => "read",
        PermissionLevel::Write => "write",
        PermissionLevel::Lock => "lock",
    }
}

/// Records a denied-access audit entry and returns the `403 Forbidden`
/// response for a path-based ACL check (`authz::check_path_permission`)
/// that failed. Visibility into denied attempts is itself a
/// security-relevant feature, so every ACL denial is logged the same way a
/// successful mutation would be — just with a "denied ..." action instead.
async fn deny_path_access(
    ctx: &SharedState,
    user: &OrgMember,
    level: PermissionLevel,
    path: &str,
) -> Response {
    let verb = permission_verb(level);
    let mut state = ctx.write().await;
    state.record_audit(&user.name, &format!("denied {verb} access to"), path);
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;
    forbidden(&format!("insufficient permissions ({verb}) for this path"))
}

/// Records a denied-access audit entry and returns the `403 Forbidden`
/// response for a role-gated (RBAC) action that a `Member`-role caller
/// attempted. See [`deny_path_access`] for why denials are audited too.
async fn deny_role_action(
    ctx: &SharedState,
    user: &OrgMember,
    action: &str,
    target: &str,
) -> Response {
    let mut state = ctx.write().await;
    state.record_audit(&user.name, &format!("denied {action}"), target);
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;
    forbidden("requires Owner or Admin role")
}

/// `true` iff `user`'s role is `Owner` or `Admin` — the RBAC bar for the
/// org-level actions in Part 3 of the authorization work (delete repo,
/// change another member's role, apply an access-control graph). A `Member`
/// never passes this regardless of team membership.
fn is_owner_or_admin(user: &OrgMember) -> bool {
    matches!(user.role, MemberRole::Owner | MemberRole::Admin)
}

/// Resolves the session cookie to the authenticated `OrgMember` and inserts
/// it into request extensions so downstream handlers can pull it out via
/// `Extension<OrgMember>` instead of re-checking the session themselves.
///
/// An access token past its `expires_at` is treated identically to a
/// missing/unknown token (401) — the client is expected to recover by
/// calling `POST /api/auth/refresh` (see [`refresh`]), not by this
/// middleware refreshing anything itself. When an expired entry is found it
/// is also removed from `sessions` so the map doesn't grow unboundedly with
/// dead tokens; this cleanup is a nice-to-have, not required for
/// correctness (an expired entry is already rejected either way).
pub async fn require_auth(
    State(ctx): State<SharedState>,
    mut request: Request,
    next: Next,
) -> Response {
    let token = auth::extract_session_token(request.headers());
    let now = auth::current_unix_time();

    let state_guard = ctx.read().await;
    let session = token
        .as_ref()
        .and_then(|t| state_guard.sessions.get(t).cloned());
    drop(state_guard);

    let email = match session {
        Some(entry) if entry.expires_at > now => Some(entry.email),
        Some(_expired) => {
            if let Some(t) = &token {
                let mut state_guard = ctx.write().await;
                state_guard.sessions.remove(t);
                let sessions = state_guard.sessions.clone();
                drop(state_guard);
                crate::db::save_blob(&ctx.db, "sessions", &sessions).await;
            }
            None
        }
        None => None,
    };

    let Some(email) = email else {
        return unauthorized();
    };

    let state_guard = ctx.read().await;
    let Some(user) = state_guard
        .org_members
        .iter()
        .find(|m| m.email == email)
        .cloned()
    else {
        return unauthorized();
    };
    drop(state_guard);

    request.extensions_mut().insert(user);
    next.run(request).await
}

/// Trivial liveness probe — always `200`, never touches `AppState`. Used by
/// the Docker `HEALTHCHECK` (see `Dockerfile`) and by anything else that just
/// needs to know the process is up and accepting connections; deliberately
/// not a deep dependency check (e.g. it doesn't query the database).
pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub async fn login(State(ctx): State<SharedState>, Json(body): Json<LoginRequest>) -> Response {
    let mut state = ctx.write().await;

    let Some(hash) = state.credentials.get(&body.email).cloned() else {
        return unauthorized();
    };
    if !auth::verify_password(&body.password, &hash) {
        return unauthorized();
    }
    let Some(user) = state
        .org_members
        .iter()
        .find(|m| m.email == body.email)
        .cloned()
    else {
        return unauthorized();
    };

    let now = auth::current_unix_time();
    let access_token = auth::generate_token();
    let refresh_token = auth::generate_token();
    state.sessions.insert(
        access_token.clone(),
        SessionEntry {
            email: body.email.clone(),
            expires_at: now + auth::ACCESS_TOKEN_TTL_SECS,
        },
    );
    state.refresh_tokens.insert(
        refresh_token.clone(),
        SessionEntry {
            email: body.email.clone(),
            expires_at: now + auth::REFRESH_TOKEN_TTL_SECS,
        },
    );
    let sessions = state.sessions.clone();
    let refresh_tokens = state.refresh_tokens.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "sessions", &sessions).await;
    crate::db::save_blob(&ctx.db, "refresh_tokens", &refresh_tokens).await;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        auth::session_cookie(&access_token).parse().unwrap(),
    );
    // `insert` would overwrite the header slot set above; `append` is what
    // lets a response carry two independent `Set-Cookie` headers.
    headers.append(
        header::SET_COOKIE,
        auth::refresh_cookie(&refresh_token).parse().unwrap(),
    );

    (headers, Json(serde_json::json!({ "user": user }))).into_response()
}

/// `POST /api/auth/refresh` — public (no valid access token required, since
/// the whole point is recovering from an expired one). Reads the refresh
/// cookie, and if it names a live (non-expired) entry in `refresh_tokens`,
/// rotates it: the old refresh token is removed unconditionally (so a
/// stolen/replayed copy of it becomes useless after one use, standard
/// refresh-token-rotation practice) and a brand-new access token + refresh
/// token pair is issued, persisted, and returned via `Set-Cookie`. Any other
/// case (missing cookie, unknown token, expired token, or a user that no
/// longer exists) is a 401.
pub async fn refresh(State(ctx): State<SharedState>, headers: HeaderMap) -> Response {
    let Some(refresh_token) = auth::extract_refresh_token(&headers) else {
        return unauthorized();
    };

    let mut state = ctx.write().await;
    let now = auth::current_unix_time();

    // Remove unconditionally — even an expired or otherwise-rejected token
    // must not be reusable afterwards.
    let Some(entry) = state.refresh_tokens.remove(&refresh_token) else {
        return unauthorized();
    };

    if entry.expires_at <= now {
        let refresh_tokens = state.refresh_tokens.clone();
        drop(state);
        crate::db::save_blob(&ctx.db, "refresh_tokens", &refresh_tokens).await;
        return unauthorized();
    }

    let Some(user) = state
        .org_members
        .iter()
        .find(|m| m.email == entry.email)
        .cloned()
    else {
        let refresh_tokens = state.refresh_tokens.clone();
        drop(state);
        crate::db::save_blob(&ctx.db, "refresh_tokens", &refresh_tokens).await;
        return unauthorized();
    };

    let new_access = auth::generate_token();
    let new_refresh = auth::generate_token();
    state.sessions.insert(
        new_access.clone(),
        SessionEntry {
            email: entry.email.clone(),
            expires_at: now + auth::ACCESS_TOKEN_TTL_SECS,
        },
    );
    state.refresh_tokens.insert(
        new_refresh.clone(),
        SessionEntry {
            email: entry.email,
            expires_at: now + auth::REFRESH_TOKEN_TTL_SECS,
        },
    );

    let sessions = state.sessions.clone();
    let refresh_tokens = state.refresh_tokens.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "sessions", &sessions).await;
    crate::db::save_blob(&ctx.db, "refresh_tokens", &refresh_tokens).await;

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::SET_COOKIE,
        auth::session_cookie(&new_access).parse().unwrap(),
    );
    response_headers.append(
        header::SET_COOKIE,
        auth::refresh_cookie(&new_refresh).parse().unwrap(),
    );

    (response_headers, Json(serde_json::json!({ "user": user }))).into_response()
}

pub async fn logout(State(ctx): State<SharedState>, headers: HeaderMap) -> Response {
    let access_token = auth::extract_session_token(&headers);
    let refresh_token = auth::extract_refresh_token(&headers);
    if access_token.is_some() || refresh_token.is_some() {
        let mut state = ctx.write().await;
        if let Some(token) = &access_token {
            state.sessions.remove(token);
        }
        if let Some(token) = &refresh_token {
            state.refresh_tokens.remove(token);
        }
        let sessions = state.sessions.clone();
        let refresh_tokens = state.refresh_tokens.clone();
        drop(state);
        crate::db::save_blob(&ctx.db, "sessions", &sessions).await;
        crate::db::save_blob(&ctx.db, "refresh_tokens", &refresh_tokens).await;
    }

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::SET_COOKIE,
        auth::cleared_session_cookie().parse().unwrap(),
    );
    response_headers.append(
        header::SET_COOKIE,
        auth::cleared_refresh_cookie().parse().unwrap(),
    );
    (response_headers, StatusCode::NO_CONTENT).into_response()
}

pub async fn me(axum::Extension(user): axum::Extension<OrgMember>) -> Json<OrgMember> {
    Json(user)
}

pub async fn list_repositories(State(ctx): State<SharedState>) -> Json<Vec<Repository>> {
    Json(repo_store::list(&ctx.db).await)
}

pub async fn get_repository(State(ctx): State<SharedState>, Path(slug): Path<String>) -> Response {
    match repo_store::get(&ctx.db, &slug).await {
        Some(repo) => Json(repo).into_response(),
        None => not_found("repository not found"),
    }
}

/// Kebab-cases an arbitrary repository name into an ASCII slug: lowercases
/// alphanumerics, collapses any run of other characters into a single `-`,
/// and never produces a leading/trailing dash.
fn slugify(name: &str) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(ch.to_ascii_lowercase());
            pending_dash = false;
        } else {
            pending_dash = true;
        }
    }
    if slug.is_empty() {
        "repository".to_string()
    } else {
        slug
    }
}

/// Appends `-2`, `-3`, ... to `base` until it no longer collides with an
/// existing repository slug.
async fn unique_slug(pool: &sqlx::SqlitePool, base: &str) -> String {
    if !repo_store::exists(pool, base).await {
        return base.to_string();
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{base}-{suffix}");
        if !repo_store::exists(pool, &candidate).await {
            return candidate;
        }
        suffix += 1;
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateRepositoryRequest {
    pub name: String,
    pub description: String,
    pub visibility: Visibility,
}

pub async fn create_repository(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Json(body): Json<CreateRepositoryRequest>,
) -> Response {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return bad_request("name is required");
    }

    let slug = unique_slug(&ctx.db, &slugify(&name)).await;
    let repository = repo_store::create(
        &ctx.db,
        repo_store::NewRepository {
            slug: slug.clone(),
            name,
            organization: "Nebula Studios".to_string(),
            description: body.description.trim().to_string(),
            visibility: body.visibility,
            is_seeded: false,
            size_label: "0 B".to_string(),
            locked_file_count: 0,
            updated_at: "just now".to_string(),
            created_at: auth::current_unix_time(),
        },
    )
    .await;

    let mut state = ctx.write().await;
    state.record_audit(&user.name, "created repository", &slug);
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    (StatusCode::CREATED, Json(repository)).into_response()
}

#[derive(Debug, Deserialize)]
pub struct UpdateRepositoryRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub visibility: Option<Visibility>,
}

pub async fn update_repository(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Path(slug): Path<String>,
    Json(body): Json<UpdateRepositoryRequest>,
) -> Response {
    if !repo_store::exists(&ctx.db, &slug).await {
        return not_found("repository not found");
    }

    if let Some(name) = &body.name
        && name.trim().is_empty()
    {
        return bad_request("name cannot be empty");
    }

    let patch = repo_store::RepositoryUpdate {
        name: body.name.map(|n| n.trim().to_string()),
        description: body.description.map(|d| d.trim().to_string()),
        visibility: body.visibility,
    };
    let repo = repo_store::update(&ctx.db, &slug, patch, "just now")
        .await
        .expect("repository existed a moment ago (existence just checked above)");

    let mut state = ctx.write().await;
    state.record_audit(&user.name, "updated settings for", &slug);
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    Json(repo).into_response()
}

pub async fn delete_repository(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Path(slug): Path<String>,
) -> Response {
    if !is_owner_or_admin(&user) {
        return deny_role_action(&ctx, &user, "delete repository", &slug).await;
    }

    // Deletes the `repositories` row itself; every VCS table's `repo_slug`
    // foreign key is `ON DELETE CASCADE` (see migrations/0001_vcs_schema.sql
    // and repo_store::delete's doc comment), so this alone also atomically
    // removes any tree/commit/branch/etc. rows for `slug` — nothing left to
    // separately clean up on the SQL side.
    if !repo_store::delete(&ctx.db, &slug).await {
        return not_found("repository not found");
    }

    let mut state = ctx.write().await;
    state.pull_requests.retain(|pr| pr.repo_slug != slug);

    state.record_audit(&user.name, "deleted repository", &slug);

    let pull_requests = state.pull_requests.clone();
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "pull_requests", &pull_requests).await;
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    StatusCode::NO_CONTENT.into_response()
}

/// Checks `staged_content` (the "current uploaded content" pointer for
/// `path` in `slug`'s working copy — see `blob_meta_store.rs`) and, if a row
/// exists there, loads the actual bytes from the filesystem blob store (see
/// `blob_store.rs`). Returns `None` when nothing has ever been uploaded to
/// this path in this repo (or, defensively, if `staged_content` points at a
/// blob that's somehow missing from disk) — either way the caller falls back
/// to the org-wide seeded content maps exactly as before this helper
/// existed. Shared by `get_file_content`/`get_image`/`get_audio`, unifying
/// what used to be three independently-maintained `uploaded_*` map lookups.
async fn resolve_uploaded_content(ctx: &SharedState, slug: &str, path: &str) -> Option<Vec<u8>> {
    let (content_hash, _size_bytes) =
        crate::blob_meta_store::get_staged_content(&ctx.db, slug, path).await?;
    let base_dir = std::path::Path::new(crate::blob_store::BASE_DIR);
    crate::blob_store::load_blob(base_dir, slug, &content_hash)
        .await
        .ok()
}

pub async fn get_file_content(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Path((slug, path)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if !repo_store::exists(&ctx.db, &slug).await {
        return not_found("repository not found");
    }
    let state = ctx.read().await;
    if !crate::authz::check_path_permission(&state, &user, &slug, &path, PermissionLevel::Read) {
        drop(state);
        return deny_path_access(&ctx, &user, PermissionLevel::Read, &path).await;
    }
    drop(state);

    // Uploaded content takes priority over the fixed demo text dataset —
    // same fallback structure as `get_image`.
    if let Some(bytes) = resolve_uploaded_content(&ctx, &slug, &path).await {
        return ranged_bytes_response(&headers, "text/plain; charset=utf-8", &bytes);
    }

    let state = ctx.read().await;
    match state.file_contents.get(&path) {
        Some(content) => {
            ranged_bytes_response(&headers, "text/plain; charset=utf-8", content.as_bytes())
        }
        None => not_found("no text content for this file"),
    }
}

/// A single inclusive byte range that has been validated against the
/// resource's total length, or the absence/rejection of one.
enum RangeOutcome {
    /// No `Range` header was present, or it couldn't be parsed / used the
    /// (unsupported) multi-range syntax — serve the whole resource as a
    /// normal `200 OK`, per the HTTP spec's guidance to ignore malformed or
    /// unsupported `Range` headers rather than erroring on them.
    FullContent,
    /// `Range: bytes=<start>-<end>` resolved to a satisfiable, inclusive
    /// `[start, end]` window within the resource.
    Partial { start: u64, end: u64 },
    /// The requested range starts at or beyond the resource's length (or is
    /// otherwise empty/invalid in a way the spec says to reject rather than
    /// ignore) — `416 Range Not Satisfiable`.
    Unsatisfiable,
}

/// Parses the simple single-range case of an incoming `Range` request
/// header (`bytes=<start>-<end>`, `bytes=<start>-`, or the suffix form
/// `bytes=-<n>`) against a resource of `total_len` bytes. Multi-range
/// requests (`bytes=0-10,20-30`) and anything not prefixed with `bytes=`
/// are treated as absent (`FullContent`) rather than rejected, matching how
/// browsers/curl behave when a server doesn't support what they asked for.
fn parse_range(headers: &HeaderMap, total_len: u64) -> RangeOutcome {
    let Some(raw) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) else {
        return RangeOutcome::FullContent;
    };
    let Some(spec) = raw.strip_prefix("bytes=") else {
        return RangeOutcome::FullContent;
    };
    if spec.contains(',') {
        return RangeOutcome::FullContent;
    }

    if total_len == 0 {
        return RangeOutcome::Unsatisfiable;
    }

    let Some((start_str, end_str)) = spec.split_once('-') else {
        return RangeOutcome::FullContent;
    };

    let (start, end) = if start_str.is_empty() {
        // Suffix range: last `n` bytes of the resource.
        let Ok(n) = end_str.parse::<u64>() else {
            return RangeOutcome::FullContent;
        };
        if n == 0 {
            return RangeOutcome::Unsatisfiable;
        }
        let n = n.min(total_len);
        (total_len - n, total_len - 1)
    } else {
        let Ok(start) = start_str.parse::<u64>() else {
            return RangeOutcome::FullContent;
        };
        let end = if end_str.is_empty() {
            total_len - 1
        } else {
            match end_str.parse::<u64>() {
                Ok(e) => e.min(total_len - 1),
                Err(_) => return RangeOutcome::FullContent,
            }
        };
        (start, end)
    };

    if start >= total_len || start > end {
        return RangeOutcome::Unsatisfiable;
    }

    RangeOutcome::Partial { start, end }
}

/// Serves `bytes` with real HTTP Range support: a satisfiable `Range`
/// header yields `206 Partial Content` with `Content-Range`/`Content-Length`
/// scoped to the requested window; no (or an unusable) `Range` header
/// yields the full body as `200 OK`; an out-of-bounds range yields
/// `416 Range Not Satisfiable`. `Accept-Ranges: bytes` is advertised on
/// every response so clients know ranged requests are supported.
fn ranged_bytes_response(headers: &HeaderMap, content_type: &str, bytes: &[u8]) -> Response {
    let total = bytes.len() as u64;
    match parse_range(headers, total) {
        RangeOutcome::FullContent => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, content_type.to_string()),
                (header::ACCEPT_RANGES, "bytes".to_string()),
            ],
            bytes.to_vec(),
        )
            .into_response(),
        RangeOutcome::Partial { start, end } => {
            let slice = bytes[start as usize..=end as usize].to_vec();
            (
                StatusCode::PARTIAL_CONTENT,
                [
                    (header::CONTENT_TYPE, content_type.to_string()),
                    (header::ACCEPT_RANGES, "bytes".to_string()),
                    (
                        header::CONTENT_RANGE,
                        format!("bytes {start}-{end}/{total}"),
                    ),
                    (header::CONTENT_LENGTH, slice.len().to_string()),
                ],
                slice,
            )
                .into_response()
        }
        RangeOutcome::Unsatisfiable => (
            StatusCode::RANGE_NOT_SATISFIABLE,
            [(header::CONTENT_RANGE, format!("bytes */{total}"))],
        )
            .into_response(),
    }
}

/// Infers a `Content-Type` from a file path's extension for uploaded image
/// bytes (whose real format we didn't otherwise track). Unrecognized
/// extensions fall back to a generic binary type rather than guessing wrong.
fn content_type_for_path(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

/// Infers a `Content-Type` for uploaded audio bytes from the path's
/// extension. Everything that isn't recognizably an MP3 is served as
/// `audio/wav`, matching the seeded demo audio's format — we don't want to
/// silently mislabel content-type, but WAV is also the only format the demo
/// dataset and this app's audio preview otherwise assume.
fn audio_content_type_for_path(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "mp3" => "audio/mpeg",
        _ => "audio/wav",
    }
}

/// Which of the three per-kind uploaded-content stores (`uploaded_images`/
/// `uploaded_text`/`uploaded_audio`) an uploaded path belongs in, inferred
/// from its extension the same way `content_type_for_path` infers a
/// `Content-Type` for `get_image`. Recognized text/audio extensions route to
/// their dedicated stores; everything else (including the existing image
/// extensions and any unrecognized extension) keeps the pre-existing
/// behavior of landing in `uploaded_images`, so this generalization can't
/// change what happens to an upload that used to work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UploadKind {
    Image,
    Text,
    Audio,
}

fn upload_kind_for_path(path: &str) -> UploadKind {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "txt" | "md" | "json" | "yaml" | "yml" => UploadKind::Text,
        "wav" | "mp3" => UploadKind::Audio,
        _ => UploadKind::Image,
    }
}

pub async fn get_image(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Path((slug, path)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if !repo_store::exists(&ctx.db, &slug).await {
        return not_found("repository not found");
    }
    let state = ctx.read().await;
    if !crate::authz::check_path_permission(&state, &user, &slug, &path, PermissionLevel::Read) {
        drop(state);
        return deny_path_access(&ctx, &user, PermissionLevel::Read, &path).await;
    }
    drop(state);

    // Uploaded content takes priority over the fixed demo SVGs — an upload
    // to a path that happens to match a demo image should show what the
    // user actually uploaded, not the seeded placeholder.
    if let Some(bytes) = resolve_uploaded_content(&ctx, &slug, &path).await {
        return ranged_bytes_response(&headers, content_type_for_path(&path), &bytes);
    }

    let state = ctx.read().await;
    match state.image_content.get(&path) {
        Some(svg) => ranged_bytes_response(&headers, "image/svg+xml", svg.as_bytes()),
        None => not_found("no image content for this file"),
    }
}

pub async fn get_image_before(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Path((slug, path)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if !repo_store::exists(&ctx.db, &slug).await {
        return not_found("repository not found");
    }
    let state = ctx.read().await;
    if !crate::authz::check_path_permission(&state, &user, &slug, &path, PermissionLevel::Read) {
        drop(state);
        return deny_path_access(&ctx, &user, PermissionLevel::Read, &path).await;
    }
    match state.image_content_before.get(&path) {
        Some(svg) => ranged_bytes_response(&headers, "image/svg+xml", svg.as_bytes()),
        None => not_found("no 'before' image for this file"),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadImageRequest {
    pub path: String,
    pub content_base64: String,
}

/// Stores raw uploaded file bytes for `slug`/`path` as a content-addressed
/// blob (see `content_hash.rs`/`blob_store.rs`/`blob_meta_store.rs`), served
/// back afterwards by `get_image`/`get_file_content`/`get_audio` via
/// `resolve_uploaded_content`. This is the write side of LoreForge Client's
/// "Add File" flow — the client base64-encodes a local file and posts it
/// here, then stages it via the existing `stage_change` endpoint.
///
/// Unlike the old per-kind-map write path, storage here is kind-agnostic:
/// every upload's bytes go through the same hash -> filesystem -> metadata
/// pipeline regardless of file type. `upload_kind_for_path` is still called
/// (its result just isn't consulted yet) purely to keep the extension-
/// sniffing logic alive for Phase 4, which needs it to populate
/// `tree_entries.file_kind` at commit time.
pub async fn upload_file(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Path(slug): Path<String>,
    Json(body): Json<UploadImageRequest>,
) -> Response {
    use base64::Engine;

    if !repo_store::exists(&ctx.db, &slug).await {
        return not_found("repository not found");
    }

    let path = body.path.trim().to_string();
    if path.is_empty() {
        return bad_request("path is required");
    }

    let state = ctx.read().await;
    if !crate::authz::check_path_permission(&state, &user, &slug, &path, PermissionLevel::Write) {
        drop(state);
        return deny_path_access(&ctx, &user, PermissionLevel::Write, &path).await;
    }
    drop(state);

    let bytes = match base64::engine::general_purpose::STANDARD.decode(&body.content_base64) {
        Ok(b) => b,
        Err(_) => return bad_request("contentBase64 is not valid base64"),
    };
    if bytes.is_empty() {
        return bad_request("uploaded file is empty");
    }

    // See doc comment above: not yet used to branch storage, only kept alive
    // for Phase 4's `tree_entries.file_kind`.
    let _kind = upload_kind_for_path(&path);

    let content_hash = crate::content_hash::sha256_hex(&bytes);
    let base_dir = std::path::Path::new(crate::blob_store::BASE_DIR);
    if let Err(err) = crate::blob_store::save_blob(base_dir, &slug, &content_hash, &bytes).await {
        tracing::error!(
            repo_slug = %slug,
            path = %path,
            error = %err,
            "failed to write uploaded blob to disk"
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to store uploaded file" })),
        )
            .into_response();
    }
    let size_bytes = bytes.len() as i64;
    crate::blob_meta_store::record_blob(&ctx.db, &slug, &content_hash, size_bytes).await;
    crate::blob_meta_store::upsert_staged_content(&ctx.db, &slug, &path, &content_hash, size_bytes)
        .await;

    let mut state = ctx.write().await;
    state.record_audit(&user.name, "uploaded", &path);
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    StatusCode::CREATED.into_response()
}

pub async fn get_audio(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Path((slug, path)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if !repo_store::exists(&ctx.db, &slug).await {
        return not_found("repository not found");
    }
    let state = ctx.read().await;
    if !crate::authz::check_path_permission(&state, &user, &slug, &path, PermissionLevel::Read) {
        drop(state);
        return deny_path_access(&ctx, &user, PermissionLevel::Read, &path).await;
    }
    drop(state);

    // Uploaded content takes priority over the fixed demo audio clip — same
    // fallback structure as `get_image`.
    if let Some(bytes) = resolve_uploaded_content(&ctx, &slug, &path).await {
        return ranged_bytes_response(&headers, audio_content_type_for_path(&path), &bytes);
    }

    let state = ctx.read().await;
    match state.audio_content.get(&path) {
        Some(bytes) => ranged_bytes_response(&headers, "audio/wav", bytes),
        None => not_found("no audio content for this file"),
    }
}

pub async fn get_tree(State(ctx): State<SharedState>, Path(slug): Path<String>) -> Response {
    if !repo_store::exists(&ctx.db, &slug).await {
        return not_found("repository not found");
    }
    let state = ctx.read().await;
    Json(state.tree.get(&slug).cloned().unwrap_or_default()).into_response()
}

#[derive(Debug, Deserialize)]
pub struct LockRequest {
    pub path: String,
    /// `true` to lock as the current user, `false` to unlock.
    pub lock: bool,
}

pub async fn toggle_lock(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Path(slug): Path<String>,
    Json(body): Json<LockRequest>,
) -> Response {
    if !repo_store::exists(&ctx.db, &slug).await {
        return not_found("repository not found");
    }
    let mut state = ctx.write().await;
    if !crate::authz::check_path_permission(&state, &user, &slug, &body.path, PermissionLevel::Lock)
    {
        drop(state);
        return deny_path_access(&ctx, &user, PermissionLevel::Lock, &body.path).await;
    }

    let locked_by = if body.lock {
        Some(user.name.clone())
    } else {
        None
    };
    let found = state.set_lock(&slug, &body.path, locked_by.clone());
    if !found {
        return not_found("file not found in tree");
    }

    let action = if body.lock { "locked" } else { "unlocked" };
    state.record_audit(&user.name, action, &body.path);

    let tree_for_repo = state.tree.get(&slug).cloned().unwrap_or_default();
    let tree = state.tree.clone();
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "tree", &tree).await;
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    Json(tree_for_repo).into_response()
}

pub async fn list_commits(State(ctx): State<SharedState>, Path(slug): Path<String>) -> Response {
    if !repo_store::exists(&ctx.db, &slug).await {
        return not_found("repository not found");
    }
    let state = ctx.read().await;
    Json(state.commits.get(&slug).cloned().unwrap_or_default()).into_response()
}

pub async fn list_branches(State(ctx): State<SharedState>, Path(slug): Path<String>) -> Response {
    if !repo_store::exists(&ctx.db, &slug).await {
        return not_found("repository not found");
    }
    let state = ctx.read().await;
    Json(state.branches.get(&slug).cloned().unwrap_or_default()).into_response()
}

pub async fn get_commit(
    State(ctx): State<SharedState>,
    Path((slug, hash)): Path<(String, String)>,
) -> Response {
    if !repo_store::exists(&ctx.db, &slug).await {
        return not_found("repository not found");
    }
    let state = ctx.read().await;
    match state
        .commits
        .get(&slug)
        .and_then(|commits| commits.iter().find(|c| c.hash == hash))
    {
        Some(commit) => Json(commit.clone()).into_response(),
        None => not_found("commit not found"),
    }
}

pub async fn get_current_branch(
    State(ctx): State<SharedState>,
    Path(slug): Path<String>,
) -> Response {
    if !repo_store::exists(&ctx.db, &slug).await {
        return not_found("repository not found");
    }
    let state = ctx.read().await;
    let branch = state
        .current_branch
        .get(&slug)
        .cloned()
        .unwrap_or_else(|| "main".to_string());
    Json(serde_json::json!({ "branch": branch })).into_response()
}

#[derive(Debug, Deserialize)]
pub struct CheckoutRequest {
    pub branch: String,
}

pub async fn checkout_branch(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Path(slug): Path<String>,
    Json(body): Json<CheckoutRequest>,
) -> Response {
    if !repo_store::exists(&ctx.db, &slug).await {
        return not_found("repository not found");
    }
    let mut state = ctx.write().await;

    let exists = state
        .branches
        .get(&slug)
        .map(|branches| branches.iter().any(|b| b.name == body.branch))
        .unwrap_or(false);
    if !exists {
        return not_found("branch not found");
    }

    state
        .current_branch
        .insert(slug.clone(), body.branch.clone());

    state.record_audit(
        &user.name,
        &format!("checked out branch {} on", body.branch),
        &slug,
    );

    let branch = body.branch.clone();
    let current_branch = state.current_branch.clone();
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "current_branch", &current_branch).await;
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    Json(serde_json::json!({ "branch": branch })).into_response()
}

#[derive(Debug, Deserialize)]
pub struct CreateBranchRequest {
    pub name: String,
    pub from: Option<String>,
}

pub async fn create_branch(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Path(slug): Path<String>,
    Json(body): Json<CreateBranchRequest>,
) -> Response {
    if !repo_store::exists(&ctx.db, &slug).await {
        return not_found("repository not found");
    }
    let mut state = ctx.write().await;

    let name = body.name.trim().to_string();
    if name.is_empty() {
        return bad_request("name is required");
    }

    let already_exists = state
        .branches
        .get(&slug)
        .map(|branches| branches.iter().any(|b| b.name == name))
        .unwrap_or(false);
    if already_exists {
        return bad_request("branch already exists");
    }

    let from_name = body.from.clone().unwrap_or_else(|| {
        state
            .current_branch
            .get(&slug)
            .cloned()
            .unwrap_or_else(|| "main".to_string())
    });

    let head = state
        .branches
        .get(&slug)
        .and_then(|branches| branches.iter().find(|b| b.name == from_name))
        .map(|b| b.head.clone())
        .unwrap_or_default();

    let branch = Branch {
        name: name.clone(),
        head,
        is_default: false,
    };
    state
        .branches
        .entry(slug.clone())
        .or_default()
        .push(branch.clone());

    state.record_audit(&user.name, &format!("created branch {name} on"), &slug);

    let branches = state.branches.clone();
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "branches", &branches).await;
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    (StatusCode::CREATED, Json(branch)).into_response()
}

pub async fn get_pending_changes(
    State(ctx): State<SharedState>,
    Path(slug): Path<String>,
) -> Response {
    if !repo_store::exists(&ctx.db, &slug).await {
        return not_found("repository not found");
    }
    let state = ctx.read().await;
    Json(
        state
            .pending_changes
            .get(&slug)
            .cloned()
            .unwrap_or_default(),
    )
    .into_response()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageRequest {
    pub path: String,
    pub change_type: FileChangeType,
    /// `true` to stage the given path with `change_type`, `false` to
    /// remove any pending entry for that path.
    pub staged: bool,
}

pub async fn stage_change(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Path(slug): Path<String>,
    Json(body): Json<StageRequest>,
) -> Response {
    if !repo_store::exists(&ctx.db, &slug).await {
        return not_found("repository not found");
    }
    let mut state = ctx.write().await;
    if !crate::authz::check_path_permission(
        &state,
        &user,
        &slug,
        &body.path,
        PermissionLevel::Write,
    ) {
        drop(state);
        return deny_path_access(&ctx, &user, PermissionLevel::Write, &body.path).await;
    }

    let entries = state.pending_changes.entry(slug.clone()).or_default();
    entries.retain(|c| c.path != body.path);
    if body.staged {
        entries.push(FileChange {
            path: body.path.clone(),
            change_type: body.change_type,
            size_delta_label: "—".to_string(),
        });
    }

    let action = if body.staged { "staged" } else { "unstaged" };
    state.record_audit(&user.name, action, &body.path);

    let pending_for_repo = state
        .pending_changes
        .get(&slug)
        .cloned()
        .unwrap_or_default();
    let pending_changes = state.pending_changes.clone();
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "pending_changes", &pending_changes).await;
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    Json(pending_for_repo).into_response()
}

#[derive(Debug, Deserialize)]
pub struct CreateCommitRequest {
    pub message: String,
    pub description: Option<String>,
}

pub async fn create_commit(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Path(slug): Path<String>,
    Json(body): Json<CreateCommitRequest>,
) -> Response {
    if !repo_store::exists(&ctx.db, &slug).await {
        return not_found("repository not found");
    }
    let mut state = ctx.write().await;

    let message = body.message.trim().to_string();
    if message.is_empty() {
        return bad_request("message is required");
    }

    let changed_files = state
        .pending_changes
        .get(&slug)
        .cloned()
        .unwrap_or_default();
    if changed_files.is_empty() {
        return bad_request("nothing to commit");
    }

    let branch_name = state
        .current_branch
        .get(&slug)
        .cloned()
        .unwrap_or_else(|| "main".to_string());
    let parent = state
        .branches
        .get(&slug)
        .and_then(|branches| branches.iter().find(|b| b.name == branch_name))
        .map(|b| b.head.clone());

    let hash = auth::generate_commit_hash();
    let short_hash = hash[..7].to_string();

    let commit = Commit {
        hash: hash.clone(),
        short_hash,
        message,
        description: body.description.filter(|d| !d.trim().is_empty()),
        author: user.name.clone(),
        author_initials: user.initials.clone(),
        timestamp: "just now".to_string(),
        changed_files,
        branch: branch_name.clone(),
        parents: parent.into_iter().collect(),
    };

    state
        .commits
        .entry(slug.clone())
        .or_default()
        .push(commit.clone());

    if let Some(branches) = state.branches.get_mut(&slug)
        && let Some(branch) = branches.iter_mut().find(|b| b.name == branch_name)
    {
        branch.head = hash;
    }

    state.pending_changes.insert(slug.clone(), Vec::new());

    state.record_audit(&user.name, "committed to", &slug);

    let commits = state.commits.clone();
    let branches = state.branches.clone();
    let pending_changes = state.pending_changes.clone();
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "commits", &commits).await;
    crate::db::save_blob(&ctx.db, "branches", &branches).await;
    crate::db::save_blob(&ctx.db, "pending_changes", &pending_changes).await;
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    (StatusCode::CREATED, Json(commit)).into_response()
}

#[derive(Debug, Deserialize)]
pub struct PullsQuery {
    pub status: Option<String>,
}

pub async fn list_pull_requests(
    State(ctx): State<SharedState>,
    Query(query): Query<PullsQuery>,
) -> Json<Vec<PullRequest>> {
    let state = ctx.read().await;
    let status = query.status.unwrap_or_else(|| "open".to_string());
    let filtered: Vec<PullRequest> = state
        .pull_requests
        .iter()
        .filter(|pr| {
            let pr_status = match pr.status {
                PrStatus::Open => "open",
                PrStatus::Merged => "merged",
                PrStatus::Closed => "closed",
            };
            pr_status == status
        })
        .cloned()
        .collect();
    Json(filtered)
}

pub async fn get_pull_request(State(ctx): State<SharedState>, Path(id): Path<String>) -> Response {
    let state = ctx.read().await;
    match state.pull_requests.iter().find(|pr| pr.id == id) {
        Some(pr) => Json(pr.clone()).into_response(),
        None => not_found("pull request not found"),
    }
}

#[derive(Debug, Deserialize)]
pub struct CommentRequest {
    pub body: String,
}

pub async fn add_comment(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Path(id): Path<String>,
    Json(body): Json<CommentRequest>,
) -> Response {
    let mut state = ctx.write().await;
    let Some(pr) = state.pull_requests.iter_mut().find(|pr| pr.id == id) else {
        return not_found("pull request not found");
    };

    let comment = PrComment {
        id: format!("local-{}", pr.comments.len() + 1),
        author: user.name.clone(),
        author_initials: user.initials.clone(),
        timestamp: "just now".to_string(),
        body: body.body,
    };
    pr.comments.push(comment);
    let pr_id = pr.id.clone();
    let pr_title = pr.title.clone();

    state.record_audit(
        &user.name,
        "commented on pull request",
        &format!("#{pr_id} {pr_title}"),
    );

    let pr = state
        .pull_requests
        .iter()
        .find(|pr| pr.id == id)
        .unwrap()
        .clone();
    let pull_requests = state.pull_requests.clone();
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "pull_requests", &pull_requests).await;
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    Json(pr).into_response()
}

pub async fn get_access_entries(
    State(ctx): State<SharedState>,
) -> Json<HashMap<String, Vec<AccessEntry>>> {
    let state = ctx.read().await;
    Json(state.access_entries.clone())
}

#[derive(Debug, Deserialize)]
pub struct PermissionToggleRequest {
    pub path: String,
    pub principal: String,
    pub level: PermissionLevel,
}

pub async fn toggle_permission(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Json(body): Json<PermissionToggleRequest>,
) -> Response {
    let mut state = ctx.write().await;
    let Some(entries) = state.access_entries.get_mut(&body.path) else {
        return not_found("no access entries for this path");
    };
    let Some(entry) = entries.iter_mut().find(|e| e.principal == body.principal) else {
        return not_found("principal not found on this path");
    };

    if let Some(pos) = entry.permissions.iter().position(|p| *p == body.level) {
        entry.permissions.remove(pos);
    } else {
        entry.permissions.push(body.level);
    }

    state.record_audit(&user.name, "updated permissions on", &body.path);

    let entries = state.access_entries.get(&body.path).unwrap().clone();
    let access_entries = state.access_entries.clone();
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "access_entries", &access_entries).await;
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    Json(entries).into_response()
}

/// Merges a partial access-control map from LoreForge Server Admin's
/// node-editor "Apply" action into `state.access_entries` — insert-or-
/// overwrite per path key, leaving any path not present in the body
/// untouched. A full-replace would wipe demo paths the Server Admin graph
/// doesn't model (e.g. "Source").
pub async fn apply_access_entries(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Json(body): Json<HashMap<String, Vec<AccessEntry>>>,
) -> Response {
    if !is_owner_or_admin(&user) {
        let mut denied_paths: Vec<String> = body.keys().cloned().collect();
        denied_paths.sort();
        return deny_role_action(
            &ctx,
            &user,
            "apply access control configuration",
            &denied_paths.join(", "),
        )
        .await;
    }

    let mut state = ctx.write().await;
    let mut applied_paths: Vec<String> = body.keys().cloned().collect();
    applied_paths.sort();
    for (path, entries) in body {
        state.access_entries.insert(path, entries);
    }

    state.record_audit(
        &user.name,
        "applied access control configuration from Server Admin",
        &applied_paths.join(", "),
    );

    let access_entries = state.access_entries.clone();
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "access_entries", &access_entries).await;
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    Json(access_entries).into_response()
}

pub async fn list_members(State(ctx): State<SharedState>) -> Json<Vec<OrgMember>> {
    let state = ctx.read().await;
    Json(state.org_members.clone())
}

#[derive(Debug, Deserialize)]
pub struct RoleUpdateRequest {
    pub role: MemberRole,
}

pub async fn update_member_role(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Path(email): Path<String>,
    Json(body): Json<RoleUpdateRequest>,
) -> Response {
    // "Caller must be Owner or Admin, full stop" — deliberately not
    // conditioned on which `email` is being changed (not even the caller's
    // own), so there's no path by which a `Member` can self-escalate.
    if !is_owner_or_admin(&user) {
        return deny_role_action(&ctx, &user, "change member role for", &email).await;
    }

    let mut state = ctx.write().await;
    let Some(member) = state.org_members.iter_mut().find(|m| m.email == email) else {
        return not_found("member not found");
    };
    member.role = body.role;
    let member = member.clone();

    state.record_audit(&user.name, "changed role for", &email);

    let org_members = state.org_members.clone();
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "org_members", &org_members).await;
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    Json(member).into_response()
}

pub async fn get_storage(State(ctx): State<SharedState>) -> Json<StorageUsage> {
    let state = ctx.read().await;
    Json(state.storage.clone())
}

pub async fn get_audit_log(State(ctx): State<SharedState>) -> Json<Vec<AuditLogEntry>> {
    let state = ctx.read().await;
    Json(state.audit_log.clone())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// Minimum acceptable length for a new password set via
/// `POST /api/auth/change-password`. This project doesn't need a full
/// password-strength meter, but rejecting a trivially weak password
/// server-side (not just in the frontend form) is a real control since the
/// client can't be trusted to enforce it.
const MIN_PASSWORD_LEN: usize = 8;

/// How long an admin-issued invite (`POST /api/org/invites`) stays
/// acceptable before `GET /api/auth/invite/{token}` / `POST /api/auth/
/// accept-invite` start treating it as gone.
const INVITE_TTL_SECS: i64 = 7 * 24 * 60 * 60; // 7 days

/// How long a forgot-password reset link stays valid. Much shorter than an
/// invite since it's a higher-stakes secret — it grants immediate account
/// takeover on an *existing* account, not just initial signup.
const PASSWORD_RESET_TTL_SECS: i64 = 60 * 60; // 1 hour

/// `POST /api/auth/change-password` — self-service password rotation for the
/// already-authenticated caller. Unlike `login`, this sits behind
/// `require_auth`, so `user` is taken from the request extension rather than
/// a body field — there is no "whose password" question to leak, since a
/// caller can only ever change their own.
///
/// Re-verifying `current_password` here is the actual security control, even
/// though the request is already authenticated: it protects against a
/// hijacked-but-still-logged-in session (e.g. someone walking up to an
/// unlocked laptop) silently taking over the account by setting a new
/// password only they know.
///
/// On success, every session and refresh token belonging to this account is
/// invalidated — including the caller's own current one, deliberately not
/// special-cased. If the password is being changed because a credential
/// leaked, any session an attacker already established (and the caller's own
/// current tab) must not survive it; the frontend is expected to redirect to
/// `/login` afterwards.
pub async fn change_password(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Json(body): Json<ChangePasswordRequest>,
) -> Response {
    let mut state = ctx.write().await;

    let Some(hash) = state.credentials.get(&user.email).cloned() else {
        return unauthorized();
    };
    if !auth::verify_password(&body.current_password, &hash) {
        return unauthorized();
    }

    if body.new_password.len() < MIN_PASSWORD_LEN {
        return bad_request(&format!(
            "new password must be at least {MIN_PASSWORD_LEN} characters"
        ));
    }
    if body.new_password == body.current_password {
        return bad_request("new password must be different from the current password");
    }

    let new_hash = auth::hash_password(&body.new_password);
    state.credentials.insert(user.email.clone(), new_hash);

    // Kill every existing session/refresh token for this account — see the
    // function doc comment for why this deliberately isn't limited to
    // "every session except the caller's own".
    state.sessions.retain(|_, entry| entry.email != user.email);
    state
        .refresh_tokens
        .retain(|_, entry| entry.email != user.email);

    state.record_audit(&user.name, "changed password for", &user.email);

    let credentials = state.credentials.clone();
    let sessions = state.sessions.clone();
    let refresh_tokens = state.refresh_tokens.clone();
    let audit_log = state.audit_log.clone();
    drop(state);

    crate::db::save_blob(&ctx.db, "credentials", &credentials).await;
    crate::db::save_blob(&ctx.db, "sessions", &sessions).await;
    crate::db::save_blob(&ctx.db, "refresh_tokens", &refresh_tokens).await;
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    StatusCode::NO_CONTENT.into_response()
}

/// Human-readable label for a `MemberRole`, used in invite-email copy.
/// Deliberately separate from `MemberRole`'s serde representation (which is
/// lowercase, see `models.rs`) — this is for a plain-text email body a human
/// reads, not JSON.
fn role_label(role: MemberRole) -> &'static str {
    match role {
        MemberRole::Owner => "Owner",
        MemberRole::Admin => "Admin",
        MemberRole::Member => "Member",
    }
}

/// Derives the initials `AppState::seed()` would have hand-written for
/// `name`: the uppercase first letter of each of the first two
/// whitespace-separated words (e.g. "Aiko Tanaka" -> "AT"). Used by
/// `accept_invite` to compute `OrgMember::initials` for a brand-new member,
/// matching the existing seeded convention rather than inventing a new one.
fn initials_for_name(name: &str) -> String {
    name.split_whitespace()
        .take(2)
        .filter_map(|word| word.chars().next())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// Reads `LOREHUB_WEB_ORIGIN`, falling back to the Next.js dev server
/// default — mirrors `main.rs::DEFAULT_CORS_ORIGIN`'s default. Handlers
/// don't have `RouterConfig` in scope (it's assembled once in `main.rs` and
/// only used to build the router/middleware stack), so invite/reset links
/// re-read the env var directly here rather than threading `RouterConfig`
/// through `AppContext` for this one string.
fn web_origin() -> String {
    std::env::var("LOREHUB_WEB_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".into())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteRequest {
    pub email: String,
    pub name: String,
    pub role: MemberRole,
    pub teams: Vec<String>,
}

/// `POST /api/org/invites` — Owner/Admin only. Creates a pending invite
/// (see `InviteEntry`) and best-effort emails the invite link to
/// `body.email`. The link is also returned directly in the response body
/// (`inviteUrl`) as the manual-fallback distribution path when no SMTP relay
/// is configured or delivery fails; this is safe specifically because only
/// an already-authenticated Owner/Admin can reach this handler.
pub async fn create_invite(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Json(body): Json<InviteRequest>,
) -> Response {
    if !is_owner_or_admin(&user) {
        return deny_role_action(&ctx, &user, "invite member", &body.email).await;
    }

    let mut state = ctx.write().await;
    if state.org_members.iter().any(|m| m.email == body.email) {
        return bad_request("a member with this email already exists");
    }

    // Overwrite any prior pending invite for this email — re-inviting is
    // just "extend/replace", not an error. `invites` is keyed by token, so
    // an old token for the same email has to be explicitly dropped or two
    // different tokens would both remain acceptable for the same signup.
    state.invites.retain(|_, entry| entry.email != body.email);

    let token = auth::generate_token();
    let now = auth::current_unix_time();
    let expires_at = now + INVITE_TTL_SECS;
    state.invites.insert(
        token.clone(),
        InviteEntry {
            email: body.email.clone(),
            name: body.name.clone(),
            role: body.role,
            teams: body.teams.clone(),
            invited_by: user.name.clone(),
            expires_at,
        },
    );

    state.record_audit(&user.name, "invited", &body.email);

    let invites = state.invites.clone();
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "invites", &invites).await;
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    let invite_url = format!("{}/accept-invite?token={token}", web_origin());
    crate::email::send_email(
        ctx.email_config.as_ref(),
        &body.email,
        "You've been invited to LoreHub",
        &format!(
            "You've been invited to join LoreHub as {}. Accept your invite here: {invite_url}",
            role_label(body.role)
        ),
    )
    .await;

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "email": body.email,
            "name": body.name,
            "role": body.role,
            "teams": body.teams,
            "expiresAt": expires_at,
            "inviteUrl": invite_url,
        })),
    )
        .into_response()
}

/// `GET /api/org/invites` — Owner/Admin only. Lists currently non-expired
/// pending invites. Deliberately omits the raw token (`inviteUrl` from
/// `create_invite`'s response) — an admin re-listing pending invites doesn't
/// need the secret link re-exposed; to resend it they revoke and recreate.
pub async fn list_invites(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
) -> Response {
    if !is_owner_or_admin(&user) {
        return deny_role_action(&ctx, &user, "list invites", "org/invites").await;
    }

    let state = ctx.read().await;
    let now = auth::current_unix_time();
    let pending: Vec<serde_json::Value> = state
        .invites
        .values()
        .filter(|entry| entry.expires_at > now)
        .map(|entry| {
            serde_json::json!({
                "email": entry.email,
                "name": entry.name,
                "role": entry.role,
                "teams": entry.teams,
                "invitedBy": entry.invited_by,
                "expiresAt": entry.expires_at,
            })
        })
        .collect();

    Json(pending).into_response()
}

/// `DELETE /api/org/invites/{email}` — Owner/Admin only. Idempotent: removes
/// any pending invite(s) for `email` (there should be at most one, per
/// `create_invite`'s overwrite-on-re-invite behavior) and returns `204`
/// whether or not one existed.
pub async fn revoke_invite(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Path(email): Path<String>,
) -> Response {
    if !is_owner_or_admin(&user) {
        return deny_role_action(&ctx, &user, "revoke invite for", &email).await;
    }

    let mut state = ctx.write().await;
    state.invites.retain(|_, entry| entry.email != email);

    state.record_audit(&user.name, "revoked invite for", &email);

    let invites = state.invites.clone();
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "invites", &invites).await;
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    StatusCode::NO_CONTENT.into_response()
}

/// `GET /api/auth/invite/{token}` — PUBLIC, no auth required (the whole
/// point is letting someone who isn't a member yet preview the invite before
/// they have any credentials). Returns just enough for an accept-invite page
/// to render "you're joining as {name} with role {role}" — deliberately
/// omits `invitedBy`/`teams`, which the preview doesn't need.
pub async fn get_invite(State(ctx): State<SharedState>, Path(token): Path<String>) -> Response {
    let state = ctx.read().await;
    let now = auth::current_unix_time();
    match state.invites.get(&token) {
        Some(entry) if entry.expires_at > now => Json(serde_json::json!({
            "email": entry.email,
            "name": entry.name,
            "role": entry.role,
        }))
        .into_response(),
        _ => not_found("invite not found or expired"),
    }
}

#[derive(Debug, Deserialize)]
pub struct AcceptInviteRequest {
    pub token: String,
    pub password: String,
}

/// `POST /api/auth/accept-invite` — PUBLIC, no auth required (the caller
/// doesn't have credentials yet). Validates the token exactly like
/// `get_invite`, then turns the invite into a real `OrgMember` + credential
/// and logs the new member straight in — issuing a session/refresh-token
/// pair and setting both cookies, the same shape `login` produces, so
/// there's no separate login step after accepting.
pub async fn accept_invite(
    State(ctx): State<SharedState>,
    Json(body): Json<AcceptInviteRequest>,
) -> Response {
    let mut state = ctx.write().await;
    let now = auth::current_unix_time();

    let Some(entry) = state.invites.get(&body.token).cloned() else {
        return not_found("invite not found or expired");
    };
    if entry.expires_at <= now {
        return not_found("invite not found or expired");
    }

    if body.password.len() < MIN_PASSWORD_LEN {
        return bad_request(&format!(
            "password must be at least {MIN_PASSWORD_LEN} characters"
        ));
    }

    // Guard against a race where the email became a member between
    // invite-creation and acceptance — don't silently overwrite an existing
    // account.
    if state.org_members.iter().any(|m| m.email == entry.email) {
        return bad_request("a member with this email already exists");
    }

    let new_member = OrgMember {
        name: entry.name.clone(),
        initials: initials_for_name(&entry.name),
        email: entry.email.clone(),
        role: entry.role,
        teams: entry.teams.clone(),
        joined_at: "just now".to_string(),
    };
    state.org_members.push(new_member.clone());
    state
        .credentials
        .insert(entry.email.clone(), auth::hash_password(&body.password));
    state.invites.remove(&body.token);

    let access_token = auth::generate_token();
    let refresh_token = auth::generate_token();
    state.sessions.insert(
        access_token.clone(),
        SessionEntry {
            email: entry.email.clone(),
            expires_at: now + auth::ACCESS_TOKEN_TTL_SECS,
        },
    );
    state.refresh_tokens.insert(
        refresh_token.clone(),
        SessionEntry {
            email: entry.email.clone(),
            expires_at: now + auth::REFRESH_TOKEN_TTL_SECS,
        },
    );

    state.record_audit(&new_member.name, "accepted invite for", &new_member.email);

    let org_members = state.org_members.clone();
    let credentials = state.credentials.clone();
    let invites = state.invites.clone();
    let sessions = state.sessions.clone();
    let refresh_tokens = state.refresh_tokens.clone();
    let audit_log = state.audit_log.clone();
    drop(state);

    crate::db::save_blob(&ctx.db, "org_members", &org_members).await;
    crate::db::save_blob(&ctx.db, "credentials", &credentials).await;
    crate::db::save_blob(&ctx.db, "invites", &invites).await;
    crate::db::save_blob(&ctx.db, "sessions", &sessions).await;
    crate::db::save_blob(&ctx.db, "refresh_tokens", &refresh_tokens).await;
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        auth::session_cookie(&access_token).parse().unwrap(),
    );
    headers.append(
        header::SET_COOKIE,
        auth::refresh_cookie(&refresh_token).parse().unwrap(),
    );

    (headers, Json(serde_json::json!({ "user": new_member }))).into_response()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgotPasswordRequest {
    pub email: String,
}

/// `POST /api/auth/forgot-password` — PUBLIC, no auth required. ALWAYS
/// returns `204 No Content`, regardless of whether `email` matches an
/// existing account — a deliberate anti-enumeration control. If this
/// endpoint's response (404 vs. 204) told a caller whether an email has a
/// LoreHub account, it could be used to enumerate valid accounts wholesale;
/// see `authz.rs`/`change_password`'s doc comments for this codebase's habit
/// of writing out non-obvious security decisions like this one. For the same
/// reason, an unknown email is never audit-logged here — logging the
/// attempted address would just move the enumeration oracle from the HTTP
/// response into whatever the audit log/server logs, which an operator might
/// expose. A *successful* token issuance isn't audited either — there's no
/// authenticated actor to attribute it to, since this is a public,
/// unauthenticated action.
pub async fn forgot_password(
    State(ctx): State<SharedState>,
    Json(body): Json<ForgotPasswordRequest>,
) -> Response {
    let now = auth::current_unix_time();
    let mut state = ctx.write().await;

    let exists = state.org_members.iter().any(|m| m.email == body.email);
    let mut issued_token = None;
    if exists {
        // Remove any other pre-existing reset token for this email first —
        // an old, still-valid link a user requested earlier must not remain
        // replayable after they requested (and will use) a newer one.
        state
            .password_resets
            .retain(|_, entry| entry.email != body.email);

        let token = auth::generate_token();
        state.password_resets.insert(
            token.clone(),
            PasswordResetEntry {
                email: body.email.clone(),
                expires_at: now + PASSWORD_RESET_TTL_SECS,
            },
        );
        issued_token = Some(token);
    }

    let password_resets = state.password_resets.clone();
    drop(state);

    if let Some(token) = issued_token {
        crate::db::save_blob(&ctx.db, "password_resets", &password_resets).await;

        let reset_link = format!("{}/reset-password?token={token}", web_origin());
        crate::email::send_email(
            ctx.email_config.as_ref(),
            &body.email,
            "Reset your LoreHub password",
            &format!(
                "A password reset was requested for your LoreHub account. Reset it here: \
                 {reset_link}\n\nIf you didn't request this, you can safely ignore this email."
            ),
        )
        .await;
    }

    StatusCode::NO_CONTENT.into_response()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

/// `POST /api/auth/reset-password` — PUBLIC, no auth required (the reset
/// token itself, not a session, is what proves the caller controls the
/// account). A bad/expired/already-used token is `401` — the same "you're
/// not who you claim to be" shape as every other unauthenticated-identity
/// failure in this codebase (reuses `unauthorized()`).
///
/// On success, every session and refresh token belonging to this account is
/// invalidated — the exact same `retain` pattern and rationale as
/// `change_password` (see its doc comment): a password reset is precisely
/// the "I lost/forgot my password, possibly because it leaked" scenario, so
/// any session an attacker already established must not survive it.
pub async fn reset_password(
    State(ctx): State<SharedState>,
    Json(body): Json<ResetPasswordRequest>,
) -> Response {
    let mut state = ctx.write().await;
    let now = auth::current_unix_time();

    let Some(entry) = state.password_resets.get(&body.token).cloned() else {
        return unauthorized();
    };
    if entry.expires_at <= now {
        return unauthorized();
    }

    if body.new_password.len() < MIN_PASSWORD_LEN {
        return bad_request(&format!(
            "new password must be at least {MIN_PASSWORD_LEN} characters"
        ));
    }

    state
        .credentials
        .insert(entry.email.clone(), auth::hash_password(&body.new_password));

    state.sessions.retain(|_, s| s.email != entry.email);
    state.refresh_tokens.retain(|_, s| s.email != entry.email);

    // Remove the used token AND any other outstanding reset token for the
    // same email — prevents replay of a second still-valid link after one
    // has been used.
    state.password_resets.retain(|_, e| e.email != entry.email);

    state.record_audit(&entry.email, "reset password for", &entry.email);

    let credentials = state.credentials.clone();
    let sessions = state.sessions.clone();
    let refresh_tokens = state.refresh_tokens.clone();
    let password_resets = state.password_resets.clone();
    let audit_log = state.audit_log.clone();
    drop(state);

    crate::db::save_blob(&ctx.db, "credentials", &credentials).await;
    crate::db::save_blob(&ctx.db, "sessions", &sessions).await;
    crate::db::save_blob(&ctx.db, "refresh_tokens", &refresh_tokens).await;
    crate::db::save_blob(&ctx.db, "password_resets", &password_resets).await;
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    StatusCode::NO_CONTENT.into_response()
}
