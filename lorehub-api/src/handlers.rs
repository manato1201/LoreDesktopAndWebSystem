use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::collections::HashMap;

use crate::models::*;
use crate::state::SharedState;

fn not_found(message: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": message })),
    )
        .into_response()
}

pub async fn list_repositories(State(state): State<SharedState>) -> Json<Vec<Repository>> {
    let state = state.read().await;
    Json(state.repositories.clone())
}

pub async fn get_repository(
    State(state): State<SharedState>,
    Path(slug): Path<String>,
) -> Response {
    let state = state.read().await;
    match state.repositories.iter().find(|r| r.slug == slug) {
        Some(repo) => Json(repo.clone()).into_response(),
        None => not_found("repository not found"),
    }
}

pub async fn get_file_content(
    State(state): State<SharedState>,
    Path((slug, path)): Path<(String, String)>,
) -> Response {
    let state = state.read().await;
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

pub async fn get_image(
    State(state): State<SharedState>,
    Path((slug, path)): Path<(String, String)>,
) -> Response {
    let state = state.read().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }
    match state.image_content.get(&path) {
        Some(svg) => svg_response(svg),
        None => not_found("no image content for this file"),
    }
}

pub async fn get_image_before(
    State(state): State<SharedState>,
    Path((slug, path)): Path<(String, String)>,
) -> Response {
    let state = state.read().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }
    match state.image_content_before.get(&path) {
        Some(svg) => svg_response(svg),
        None => not_found("no 'before' image for this file"),
    }
}

pub async fn get_audio(
    State(state): State<SharedState>,
    Path((slug, path)): Path<(String, String)>,
) -> Response {
    let state = state.read().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }
    match state.audio_content.get(&path) {
        Some(bytes) => ([(header::CONTENT_TYPE, "audio/wav")], bytes.clone()).into_response(),
        None => not_found("no audio content for this file"),
    }
}

pub async fn get_tree(State(state): State<SharedState>, Path(slug): Path<String>) -> Response {
    let state = state.read().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }
    Json(state.tree.clone()).into_response()
}

#[derive(Debug, Deserialize)]
pub struct LockRequest {
    pub path: String,
    /// `true` to lock as the current user, `false` to unlock.
    pub lock: bool,
}

pub async fn toggle_lock(
    State(state): State<SharedState>,
    Path(slug): Path<String>,
    Json(body): Json<LockRequest>,
) -> Response {
    let mut state = state.write().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }

    let locked_by = if body.lock {
        Some("You".to_string())
    } else {
        None
    };
    let found = state.set_lock(&body.path, locked_by.clone());
    if !found {
        return not_found("file not found in tree");
    }

    let action = if body.lock { "locked" } else { "unlocked" };
    state.record_audit("You", action, &body.path);

    Json(state.tree.clone()).into_response()
}

pub async fn list_commits(State(state): State<SharedState>, Path(slug): Path<String>) -> Response {
    let state = state.read().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }
    Json(state.commits.clone()).into_response()
}

pub async fn list_branches(State(state): State<SharedState>, Path(slug): Path<String>) -> Response {
    let state = state.read().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }
    Json(state.branches.clone()).into_response()
}

pub async fn get_commit(
    State(state): State<SharedState>,
    Path((slug, hash)): Path<(String, String)>,
) -> Response {
    let state = state.read().await;
    if !state.repositories.iter().any(|r| r.slug == slug) {
        return not_found("repository not found");
    }
    match state.commits.iter().find(|c| c.hash == hash) {
        Some(commit) => Json(commit.clone()).into_response(),
        None => not_found("commit not found"),
    }
}

#[derive(Debug, Deserialize)]
pub struct PullsQuery {
    pub status: Option<String>,
}

pub async fn list_pull_requests(
    State(state): State<SharedState>,
    Query(query): Query<PullsQuery>,
) -> Json<Vec<PullRequest>> {
    let state = state.read().await;
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

pub async fn get_pull_request(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Response {
    let state = state.read().await;
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
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<CommentRequest>,
) -> Response {
    let mut state = state.write().await;
    let Some(pr) = state.pull_requests.iter_mut().find(|pr| pr.id == id) else {
        return not_found("pull request not found");
    };

    let comment = PrComment {
        id: format!("local-{}", pr.comments.len() + 1),
        author: "You".to_string(),
        author_initials: "Y".to_string(),
        timestamp: "just now".to_string(),
        body: body.body,
    };
    pr.comments.push(comment);
    let pr_id = pr.id.clone();
    let pr_title = pr.title.clone();

    state.record_audit(
        "You",
        "commented on pull request",
        &format!("#{pr_id} {pr_title}"),
    );

    let pr = state.pull_requests.iter().find(|pr| pr.id == id).unwrap();
    Json(pr.clone()).into_response()
}

pub async fn get_access_entries(
    State(state): State<SharedState>,
) -> Json<HashMap<String, Vec<AccessEntry>>> {
    let state = state.read().await;
    Json(state.access_entries.clone())
}

#[derive(Debug, Deserialize)]
pub struct PermissionToggleRequest {
    pub path: String,
    pub principal: String,
    pub level: PermissionLevel,
}

pub async fn toggle_permission(
    State(state): State<SharedState>,
    Json(body): Json<PermissionToggleRequest>,
) -> Response {
    let mut state = state.write().await;
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

    state.record_audit("You", "updated permissions on", &body.path);

    let entries = state.access_entries.get(&body.path).unwrap();
    Json(entries.clone()).into_response()
}

pub async fn list_members(State(state): State<SharedState>) -> Json<Vec<OrgMember>> {
    let state = state.read().await;
    Json(state.org_members.clone())
}

#[derive(Debug, Deserialize)]
pub struct RoleUpdateRequest {
    pub role: MemberRole,
}

pub async fn update_member_role(
    State(state): State<SharedState>,
    Path(email): Path<String>,
    Json(body): Json<RoleUpdateRequest>,
) -> Response {
    let mut state = state.write().await;
    let Some(member) = state.org_members.iter_mut().find(|m| m.email == email) else {
        return not_found("member not found");
    };
    member.role = body.role;
    let member = member.clone();

    state.record_audit("You", "changed role for", &email);

    Json(member).into_response()
}

pub async fn get_storage(State(state): State<SharedState>) -> Json<StorageUsage> {
    let state = state.read().await;
    Json(state.storage.clone())
}

pub async fn get_audit_log(State(state): State<SharedState>) -> Json<Vec<AuditLogEntry>> {
    let state = state.read().await;
    Json(state.audit_log.clone())
}
