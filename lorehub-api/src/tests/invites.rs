//! Integration tests for the admin invite flow: `POST`/`GET
//! /api/org/invites`, `DELETE /api/org/invites/{email}`, `GET /api/auth/
//! invite/{token}`, and `POST /api/auth/accept-invite`. Driven end-to-end
//! through the real router (see `tests/mod.rs`).
use axum::http::StatusCode;

use super::{
    DEMO_EMAIL, body_json, cookie_pair, json_request, login_cookie, plain_request, send,
    set_cookies, test_app,
};

/// Seeded `Member`-role account (no Owner/Admin RBAC bar to clear) — see
/// `state.rs::seed()` / `tests/authz.rs`'s fixture recap.
const MEMBER_EMAIL: &str = "diego.fernandez@nebula.studio";

const NEW_MEMBER_EMAIL: &str = "new.hire@nebula.studio";
const NEW_MEMBER_NAME: &str = "New Hire";
const INVITE_ACCEPT_PASSWORD: &str = "a-brand-new-password";

/// Creates an invite for `NEW_MEMBER_EMAIL` as `owner_cookie` and returns the
/// parsed `201` response body (`{ email, name, role, teams, expiresAt,
/// inviteUrl }`).
async fn create_invite(app: &axum::Router, owner_cookie: &str) -> serde_json::Value {
    let req = json_request(
        "POST",
        "/api/org/invites",
        Some(owner_cookie),
        serde_json::json!({
            "email": NEW_MEMBER_EMAIL,
            "name": NEW_MEMBER_NAME,
            "role": "member",
            "teams": ["QA Contractors"],
        }),
    );
    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    body_json(resp).await
}

/// Pulls the `token` query param back out of an `inviteUrl`.
fn extract_token(invite_url: &str) -> String {
    invite_url
        .split("token=")
        .nth(1)
        .expect("inviteUrl has a token query param")
        .to_string()
}

#[tokio::test]
async fn owner_can_create_invite_and_response_includes_invite_url_with_token() {
    let app = test_app().await;
    let owner_cookie = login_cookie(&app, DEMO_EMAIL).await;

    let body = create_invite(&app, &owner_cookie).await;
    assert_eq!(body["email"], NEW_MEMBER_EMAIL);
    assert_eq!(body["name"], NEW_MEMBER_NAME);
    assert_eq!(body["role"], "member");
    let invite_url = body["inviteUrl"].as_str().expect("inviteUrl is a string");
    assert!(
        invite_url.contains("token="),
        "expected a token query param in {invite_url}"
    );
}

#[tokio::test]
async fn member_role_cannot_create_invite() {
    let app = test_app().await;
    let member_cookie = login_cookie(&app, MEMBER_EMAIL).await;

    let req = json_request(
        "POST",
        "/api/org/invites",
        Some(&member_cookie),
        serde_json::json!({
            "email": NEW_MEMBER_EMAIL,
            "name": NEW_MEMBER_NAME,
            "role": "member",
            "teams": [],
        }),
    );
    let resp = send(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn inviting_an_existing_member_email_is_400() {
    let app = test_app().await;
    let owner_cookie = login_cookie(&app, DEMO_EMAIL).await;

    let req = json_request(
        "POST",
        "/api/org/invites",
        Some(&owner_cookie),
        serde_json::json!({
            "email": DEMO_EMAIL,
            "name": "Someone Else",
            "role": "member",
            "teams": [],
        }),
    );
    let resp = send(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_invite_preview_requires_no_auth_and_returns_email_name_role() {
    let app = test_app().await;
    let owner_cookie = login_cookie(&app, DEMO_EMAIL).await;
    let created = create_invite(&app, &owner_cookie).await;
    let token = extract_token(created["inviteUrl"].as_str().unwrap());

    // No `Cookie` header at all — this endpoint must be reachable pre-login.
    let resp = send(
        &app,
        plain_request("GET", &format!("/api/auth/invite/{token}"), None),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["email"], NEW_MEMBER_EMAIL);
    assert_eq!(body["name"], NEW_MEMBER_NAME);
    assert_eq!(body["role"], "member");
}

#[tokio::test]
async fn get_invite_preview_with_unknown_token_is_404() {
    let app = test_app().await;
    let resp = send(
        &app,
        plain_request("GET", "/api/auth/invite/not-a-real-token", None),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn accept_invite_with_valid_token_logs_in_and_creates_member() {
    let app = test_app().await;
    let owner_cookie = login_cookie(&app, DEMO_EMAIL).await;
    let created = create_invite(&app, &owner_cookie).await;
    let token = extract_token(created["inviteUrl"].as_str().unwrap());

    let resp = send(
        &app,
        json_request(
            "POST",
            "/api/auth/accept-invite",
            None,
            serde_json::json!({ "token": token, "password": INVITE_ACCEPT_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let cookies = set_cookies(&resp);
    assert_eq!(
        cookies.len(),
        2,
        "accept-invite should set exactly two cookies, same as login"
    );
    let combined = cookies
        .iter()
        .map(|c| {
            let (name, value) = cookie_pair(c);
            format!("{name}={value}")
        })
        .collect::<Vec<_>>()
        .join("; ");

    let body = body_json(resp).await;
    assert_eq!(body["user"]["email"], NEW_MEMBER_EMAIL);
    assert_eq!(body["user"]["role"], "member");

    // The returned cookies really do work — a follow-up `/api/auth/me` shows
    // the newly created member, no separate login step required.
    let me_resp = send(&app, plain_request("GET", "/api/auth/me", Some(&combined))).await;
    assert_eq!(me_resp.status(), StatusCode::OK);
    let me = body_json(me_resp).await;
    assert_eq!(me["email"], NEW_MEMBER_EMAIL);
    assert_eq!(me["name"], NEW_MEMBER_NAME);
}

#[tokio::test]
async fn accept_invite_with_too_short_password_is_400_and_does_not_create_member() {
    let app = test_app().await;
    let owner_cookie = login_cookie(&app, DEMO_EMAIL).await;
    let created = create_invite(&app, &owner_cookie).await;
    let token = extract_token(created["inviteUrl"].as_str().unwrap());

    let resp = send(
        &app,
        json_request(
            "POST",
            "/api/auth/accept-invite",
            None,
            serde_json::json!({ "token": token, "password": "short7x" }),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Nothing was consumed — the invite is still pending.
    let list = body_json(
        send(
            &app,
            plain_request("GET", "/api/org/invites", Some(&owner_cookie)),
        )
        .await,
    )
    .await;
    let entries = list.as_array().expect("invites list is a JSON array");
    assert!(
        entries.iter().any(|e| e["email"] == NEW_MEMBER_EMAIL),
        "expected the invite to still be pending after a rejected accept, got: {entries:?}"
    );
}

#[tokio::test]
async fn accept_invite_with_unknown_token_is_404() {
    let app = test_app().await;
    let resp = send(
        &app,
        json_request(
            "POST",
            "/api/auth/accept-invite",
            None,
            serde_json::json!({ "token": "not-a-real-token", "password": INVITE_ACCEPT_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// Proves the invite token is genuinely single-use: accepting it once
/// succeeds, and a second attempt with the exact same token 404s.
#[tokio::test]
async fn accept_invite_token_is_single_use() {
    let app = test_app().await;
    let owner_cookie = login_cookie(&app, DEMO_EMAIL).await;
    let created = create_invite(&app, &owner_cookie).await;
    let token = extract_token(created["inviteUrl"].as_str().unwrap());

    let first = send(
        &app,
        json_request(
            "POST",
            "/api/auth/accept-invite",
            None,
            serde_json::json!({ "token": token, "password": INVITE_ACCEPT_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);

    let second = send(
        &app,
        json_request(
            "POST",
            "/api/auth/accept-invite",
            None,
            serde_json::json!({ "token": token, "password": INVITE_ACCEPT_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(
        second.status(),
        StatusCode::NOT_FOUND,
        "an already-accepted invite token must not be reusable"
    );
}

#[tokio::test]
async fn owner_can_revoke_invite_and_member_role_cannot() {
    let app = test_app().await;
    let owner_cookie = login_cookie(&app, DEMO_EMAIL).await;
    let created = create_invite(&app, &owner_cookie).await;
    let token = extract_token(created["inviteUrl"].as_str().unwrap());

    let member_cookie = login_cookie(&app, MEMBER_EMAIL).await;
    let denied = send(
        &app,
        plain_request(
            "DELETE",
            &format!("/api/org/invites/{NEW_MEMBER_EMAIL}"),
            Some(&member_cookie),
        ),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let deleted = send(
        &app,
        plain_request(
            "DELETE",
            &format!("/api/org/invites/{NEW_MEMBER_EMAIL}"),
            Some(&owner_cookie),
        ),
    )
    .await;
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);

    let preview = send(
        &app,
        plain_request("GET", &format!("/api/auth/invite/{token}"), None),
    )
    .await;
    assert_eq!(
        preview.status(),
        StatusCode::NOT_FOUND,
        "a revoked invite's token must stop resolving"
    );
}

#[tokio::test]
async fn list_invites_does_not_expose_raw_token() {
    let app = test_app().await;
    let owner_cookie = login_cookie(&app, DEMO_EMAIL).await;
    create_invite(&app, &owner_cookie).await;

    let list = body_json(
        send(
            &app,
            plain_request("GET", "/api/org/invites", Some(&owner_cookie)),
        )
        .await,
    )
    .await;
    let entries = list.as_array().expect("invites list is a JSON array");
    assert!(!entries.is_empty());
    for entry in entries {
        let obj = entry.as_object().expect("invite entry is a JSON object");
        assert!(
            !obj.contains_key("token"),
            "list response leaked a raw token field: {entry}"
        );
        assert!(
            !obj.contains_key("inviteUrl"),
            "list response leaked an inviteUrl field: {entry}"
        );
    }
}
