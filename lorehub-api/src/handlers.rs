use axum::Json;
use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::collections::HashMap;

use crate::auth;
use crate::models::*;
use crate::state::SharedState;

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

/// Resolves the session cookie to the authenticated `OrgMember` and inserts
/// it into request extensions so downstream handlers can pull it out via
/// `Extension<OrgMember>` instead of re-checking the session themselves.
pub async fn require_auth(
    State(ctx): State<SharedState>,
    mut request: Request,
    next: Next,
) -> Response {
    let token = auth::extract_session_token(request.headers());
    let state_guard = ctx.read().await;
    let email = token.and_then(|t| state_guard.sessions.get(&t).cloned());
    let Some(email) = email else {
        return unauthorized();
    };
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

    let token = auth::generate_token();
    state.sessions.insert(token.clone(), body.email);
    let sessions = state.sessions.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "sessions", &sessions).await;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        auth::session_cookie(&token).parse().unwrap(),
    );

    (headers, Json(serde_json::json!({ "user": user }))).into_response()
}

pub async fn logout(State(ctx): State<SharedState>, headers: HeaderMap) -> Response {
    if let Some(token) = auth::extract_session_token(&headers) {
        let mut state = ctx.write().await;
        state.sessions.remove(&token);
        let sessions = state.sessions.clone();
        drop(state);
        crate::db::save_blob(&ctx.db, "sessions", &sessions).await;
    }

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::SET_COOKIE,
        auth::cleared_session_cookie().parse().unwrap(),
    );
    (response_headers, StatusCode::NO_CONTENT).into_response()
}

pub async fn me(axum::Extension(user): axum::Extension<OrgMember>) -> Json<OrgMember> {
    Json(user)
}

pub async fn list_repositories(State(ctx): State<SharedState>) -> Json<Vec<Repository>> {
    let state = ctx.read().await;
    Json(state.repositories.clone())
}

pub async fn get_repository(State(ctx): State<SharedState>, Path(slug): Path<String>) -> Response {
    let state = ctx.read().await;
    match state.repositories.iter().find(|r| r.slug == slug) {
        Some(repo) => Json(repo.clone()).into_response(),
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
fn unique_slug(base: &str, existing: &[Repository]) -> String {
    if !existing.iter().any(|r| r.slug == base) {
        return base.to_string();
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{base}-{suffix}");
        if !existing.iter().any(|r| r.slug == candidate) {
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

    let mut state = ctx.write().await;
    let slug = unique_slug(&slugify(&name), &state.repositories);

    let repository = Repository {
        slug: slug.clone(),
        name,
        organization: "Nebula Studios".to_string(),
        description: body.description.trim().to_string(),
        updated_at: "just now".to_string(),
        size_label: "0 B".to_string(),
        locked_file_count: 0,
        visibility: body.visibility,
    };
    state.repositories.push(repository.clone());

    state.record_audit(&user.name, "created repository", &slug);

    let repositories = state.repositories.clone();
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "repositories", &repositories).await;
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
    let mut state = ctx.write().await;
    let Some(repo) = state.repositories.iter_mut().find(|r| r.slug == slug) else {
        return not_found("repository not found");
    };

    if let Some(name) = body.name {
        let name = name.trim().to_string();
        if name.is_empty() {
            return bad_request("name cannot be empty");
        }
        repo.name = name;
    }
    if let Some(description) = body.description {
        repo.description = description.trim().to_string();
    }
    if let Some(visibility) = body.visibility {
        repo.visibility = visibility;
    }
    repo.updated_at = "just now".to_string();
    let repo = repo.clone();

    state.record_audit(&user.name, "updated settings for", &slug);

    let repositories = state.repositories.clone();
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "repositories", &repositories).await;
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    Json(repo).into_response()
}

pub async fn delete_repository(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Path(slug): Path<String>,
) -> Response {
    let mut state = ctx.write().await;
    let before = state.repositories.len();
    state.repositories.retain(|r| r.slug != slug);
    if state.repositories.len() == before {
        return not_found("repository not found");
    }
    state.seeded_repo_slugs.remove(&slug);
    state.pull_requests.retain(|pr| pr.repo_slug != slug);

    state.record_audit(&user.name, "deleted repository", &slug);

    let repositories = state.repositories.clone();
    let seeded_repo_slugs = state.seeded_repo_slugs.clone();
    let pull_requests = state.pull_requests.clone();
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "repositories", &repositories).await;
    crate::db::save_blob(&ctx.db, "seeded_repo_slugs", &seeded_repo_slugs).await;
    crate::db::save_blob(&ctx.db, "pull_requests", &pull_requests).await;
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    StatusCode::NO_CONTENT.into_response()
}

pub async fn get_file_content(
    State(ctx): State<SharedState>,
    Path((slug, path)): Path<(String, String)>,
) -> Response {
    let state = ctx.read().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }
    match state.file_contents.get(&path) {
        Some(content) => Json(serde_json::json!({ "content": content })).into_response(),
        None => not_found("no text content for this file"),
    }
}

fn svg_response(svg: &str) -> Response {
    ([(header::CONTENT_TYPE, "image/svg+xml")], svg.to_string()).into_response()
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

pub async fn get_image(
    State(ctx): State<SharedState>,
    Path((slug, path)): Path<(String, String)>,
) -> Response {
    let state = ctx.read().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }

    // User-uploaded content takes priority over the fixed demo SVGs — an
    // upload to a path that happens to match a demo image should show what
    // the user actually uploaded, not the seeded placeholder.
    if let Some(bytes) = state.uploaded_images.get(&slug).and_then(|m| m.get(&path)) {
        return (
            [(header::CONTENT_TYPE, content_type_for_path(&path))],
            bytes.clone(),
        )
            .into_response();
    }

    match state.image_content.get(&path) {
        Some(svg) => svg_response(svg),
        None => not_found("no image content for this file"),
    }
}

pub async fn get_image_before(
    State(ctx): State<SharedState>,
    Path((slug, path)): Path<(String, String)>,
) -> Response {
    let state = ctx.read().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }
    match state.image_content_before.get(&path) {
        Some(svg) => svg_response(svg),
        None => not_found("no 'before' image for this file"),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadImageRequest {
    pub path: String,
    pub content_base64: String,
}

/// Stores raw uploaded file bytes for `slug`/`path`, served back afterwards
/// by `get_image` (see there for the per-repo-keyed lookup and content-type
/// inference). This is the write side of the Binary Diff Viewer's "Add
/// File" flow — LoreForge Client base64-encodes a local file and posts it
/// here, then stages it via the existing `stage_change` endpoint.
pub async fn upload_image(
    State(ctx): State<SharedState>,
    axum::Extension(user): axum::Extension<OrgMember>,
    Path(slug): Path<String>,
    Json(body): Json<UploadImageRequest>,
) -> Response {
    use base64::Engine;

    let mut state = ctx.write().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }

    let path = body.path.trim().to_string();
    if path.is_empty() {
        return bad_request("path is required");
    }

    let bytes = match base64::engine::general_purpose::STANDARD.decode(&body.content_base64) {
        Ok(b) => b,
        Err(_) => return bad_request("contentBase64 is not valid base64"),
    };
    if bytes.is_empty() {
        return bad_request("uploaded file is empty");
    }

    state
        .uploaded_images
        .entry(slug.clone())
        .or_default()
        .insert(path.clone(), bytes);

    state.record_audit(&user.name, "uploaded", &path);

    let uploaded_images = state.uploaded_images.clone();
    let audit_log = state.audit_log.clone();
    drop(state);
    crate::db::save_blob(&ctx.db, "uploaded_images", &uploaded_images).await;
    crate::db::save_blob(&ctx.db, "audit_log", &audit_log).await;

    StatusCode::CREATED.into_response()
}

pub async fn get_audio(
    State(ctx): State<SharedState>,
    Path((slug, path)): Path<(String, String)>,
) -> Response {
    let state = ctx.read().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }
    match state.audio_content.get(&path) {
        Some(bytes) => ([(header::CONTENT_TYPE, "audio/wav")], bytes.clone()).into_response(),
        None => not_found("no audio content for this file"),
    }
}

pub async fn get_tree(State(ctx): State<SharedState>, Path(slug): Path<String>) -> Response {
    let state = ctx.read().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }
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
    let mut state = ctx.write().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
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
    let state = ctx.read().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }
    Json(state.commits.get(&slug).cloned().unwrap_or_default()).into_response()
}

pub async fn list_branches(State(ctx): State<SharedState>, Path(slug): Path<String>) -> Response {
    let state = ctx.read().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }
    Json(state.branches.get(&slug).cloned().unwrap_or_default()).into_response()
}

pub async fn get_commit(
    State(ctx): State<SharedState>,
    Path((slug, hash)): Path<(String, String)>,
) -> Response {
    let state = ctx.read().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }
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
    let state = ctx.read().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }
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
    let mut state = ctx.write().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }

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
    let mut state = ctx.write().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }

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
    let state = ctx.read().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }
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
    let mut state = ctx.write().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
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
    let mut state = ctx.write().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }

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
