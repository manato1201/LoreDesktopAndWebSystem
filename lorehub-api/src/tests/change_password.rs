//! Integration tests for `POST /api/auth/change-password`
//! (`handlers::change_password`), driven end-to-end through the real router
//! (see `tests/mod.rs`).
//!
//! Beyond status-code coverage, two tests prove the actual security
//! properties this endpoint exists for:
//! - the old password stops working for `/api/auth/login` after a
//!   successful change, and the new one works;
//! - a session established *before* the change (simulating a second
//!   device/tab) is rejected on a protected endpoint *after* the change —
//!   i.e. changing the password really does kill every other session, not
//!   just update the credential.
use axum::http::StatusCode;

use super::{
    DEMO_EMAIL, DEMO_PASSWORD, body_json, json_request, login, login_cookie, plain_request, send,
    test_app,
};

const NEW_PASSWORD: &str = "a-brand-new-password";

#[tokio::test]
async fn wrong_current_password_is_401() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let req = json_request(
        "POST",
        "/api/auth/change-password",
        Some(&cookie),
        serde_json::json!({
            "currentPassword": "definitely-not-the-current-password",
            "newPassword": NEW_PASSWORD,
        }),
    );
    let resp = send(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn weak_new_password_is_400() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let req = json_request(
        "POST",
        "/api/auth/change-password",
        Some(&cookie),
        serde_json::json!({
            "currentPassword": DEMO_PASSWORD,
            "newPassword": "short7x",
        }),
    );
    let resp = send(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn empty_new_password_is_400() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let req = json_request(
        "POST",
        "/api/auth/change-password",
        Some(&cookie),
        serde_json::json!({
            "currentPassword": DEMO_PASSWORD,
            "newPassword": "",
        }),
    );
    let resp = send(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn new_password_same_as_current_is_400() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let req = json_request(
        "POST",
        "/api/auth/change-password",
        Some(&cookie),
        serde_json::json!({
            "currentPassword": DEMO_PASSWORD,
            "newPassword": DEMO_PASSWORD,
        }),
    );
    let resp = send(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn change_password_without_a_session_is_401() {
    let app = test_app().await;

    let req = json_request(
        "POST",
        "/api/auth/change-password",
        None,
        serde_json::json!({
            "currentPassword": DEMO_PASSWORD,
            "newPassword": NEW_PASSWORD,
        }),
    );
    let resp = send(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn successful_change_returns_204_and_is_audited() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let req = json_request(
        "POST",
        "/api/auth/change-password",
        Some(&cookie),
        serde_json::json!({
            "currentPassword": DEMO_PASSWORD,
            "newPassword": NEW_PASSWORD,
        }),
    );
    let resp = send(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // The change itself killed this session too — but we need a fresh
    // session to check the audit log, so log in again with the new password.
    let fresh_cookie = login_cookie_with_password(&app, DEMO_EMAIL, NEW_PASSWORD).await;
    let audit = body_json(
        send(
            &app,
            plain_request("GET", "/api/org/audit-log", Some(&fresh_cookie)),
        )
        .await,
    )
    .await;
    let entries = audit.as_array().expect("audit log is a JSON array");
    let found = entries
        .iter()
        .any(|e| e["action"] == "changed password for" && e["target"] == DEMO_EMAIL);
    assert!(
        found,
        "expected an audit entry for the password change, got: {entries:?}"
    );
}

/// Proves the first core security property: after a successful change, the
/// *old* password no longer authenticates via `/api/auth/login`, and the
/// *new* one does.
#[tokio::test]
async fn old_password_is_dead_and_new_password_works_after_change() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let change_req = json_request(
        "POST",
        "/api/auth/change-password",
        Some(&cookie),
        serde_json::json!({
            "currentPassword": DEMO_PASSWORD,
            "newPassword": NEW_PASSWORD,
        }),
    );
    let change_resp = send(&app, change_req).await;
    assert_eq!(change_resp.status(), StatusCode::NO_CONTENT);

    // Old password: must now fail.
    let old_login_req = json_request(
        "POST",
        "/api/auth/login",
        None,
        serde_json::json!({ "email": DEMO_EMAIL, "password": DEMO_PASSWORD }),
    );
    let old_login_resp = send(&app, old_login_req).await;
    assert_eq!(
        old_login_resp.status(),
        StatusCode::UNAUTHORIZED,
        "the old password must no longer work after a successful change"
    );

    // New password: must now succeed.
    let new_login_req = json_request(
        "POST",
        "/api/auth/login",
        None,
        serde_json::json!({ "email": DEMO_EMAIL, "password": NEW_PASSWORD }),
    );
    let new_login_resp = send(&app, new_login_req).await;
    assert_eq!(
        new_login_resp.status(),
        StatusCode::OK,
        "the new password must work after a successful change"
    );
}

/// Proves the second core security property: a session established
/// *before* the password change (simulating a second device/tab that logged
/// in independently) is rejected on a protected endpoint *after* the
/// change — i.e. every existing session for the account is killed, not just
/// the one that performed the change.
#[tokio::test]
async fn other_pre_existing_session_is_killed_by_a_password_change() {
    let app = test_app().await;

    // "Device A" and "Device B" both log in independently as the same user.
    let cookie_a = login_cookie(&app, DEMO_EMAIL).await;
    let (cookie_b, _access_b, _refresh_b) = login(&app, DEMO_EMAIL).await;

    // Sanity check: both sessions currently work.
    let me_a_before = send(&app, plain_request("GET", "/api/auth/me", Some(&cookie_a))).await;
    assert_eq!(me_a_before.status(), StatusCode::OK);
    let me_b_before = send(&app, plain_request("GET", "/api/auth/me", Some(&cookie_b))).await;
    assert_eq!(me_b_before.status(), StatusCode::OK);

    // Device A changes the password.
    let change_req = json_request(
        "POST",
        "/api/auth/change-password",
        Some(&cookie_a),
        serde_json::json!({
            "currentPassword": DEMO_PASSWORD,
            "newPassword": NEW_PASSWORD,
        }),
    );
    let change_resp = send(&app, change_req).await;
    assert_eq!(change_resp.status(), StatusCode::NO_CONTENT);

    // Device B's previously-valid access token must now be dead too.
    let me_b_after = send(&app, plain_request("GET", "/api/auth/me", Some(&cookie_b))).await;
    assert_eq!(
        me_b_after.status(),
        StatusCode::UNAUTHORIZED,
        "a session that existed before the password change must not survive it"
    );

    // Device A's own session (the one that performed the change) must also
    // be dead — the endpoint deliberately does not special-case the caller.
    let me_a_after = send(&app, plain_request("GET", "/api/auth/me", Some(&cookie_a))).await;
    assert_eq!(
        me_a_after.status(),
        StatusCode::UNAUTHORIZED,
        "the calling session itself must not survive its own password change"
    );
}

/// Async helper mirroring `super::login`/`login_cookie`, but for a password
/// other than the seeded default `DEMO_PASSWORD` — needed here because after
/// a successful change the demo account's password is no longer `"lorehub"`.
async fn login_cookie_with_password(app: &axum::Router, email: &str, password: &str) -> String {
    let req = json_request(
        "POST",
        "/api/auth/login",
        None,
        serde_json::json!({ "email": email, "password": password }),
    );
    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "login should succeed");
    let cookies = super::set_cookies(&resp);
    cookies
        .iter()
        .map(|c| {
            let (name, value) = super::cookie_pair(c);
            format!("{name}={value}")
        })
        .collect::<Vec<_>>()
        .join("; ")
}
