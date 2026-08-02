//! Integration tests for the forgot/reset-password flow:
//! `POST /api/auth/forgot-password` and `POST /api/auth/reset-password`.
//! Driven end-to-end through the real router (see `tests/mod.rs`).
//!
//! `forgot_password`'s response never returns the generated reset token
//! (that's a deliberate anti-enumeration choice — see its doc comment in
//! `handlers.rs`), so these tests use [`test_app_with_state`] to reach into
//! `SharedState` directly and fish the token back out of
//! `AppState::password_resets`, then drive the rest of the flow over HTTP
//! exactly like a real client would with a real emailed link.
use axum::http::StatusCode;

use super::{
    DEMO_EMAIL, DEMO_PASSWORD, json_request, login_cookie, plain_request, send, test_app_with_state,
};
use crate::state::SharedState;

const NEW_PASSWORD: &str = "a-brand-new-password";
const UNKNOWN_EMAIL: &str = "nobody.here@nebula.studio";

/// Drives `POST /api/auth/forgot-password` for `email` through the real
/// router, asserts it returned `204`, then reads the generated token
/// directly out of `ctx`. Panics if no token was created — callers that
/// intentionally test the "unknown email" path should assert against
/// `ctx.read().await.password_resets` themselves instead of using this
/// helper.
async fn request_reset_and_get_token(app: &axum::Router, ctx: &SharedState, email: &str) -> String {
    let resp = send(
        app,
        json_request(
            "POST",
            "/api/auth/forgot-password",
            None,
            serde_json::json!({ "email": email }),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let state = ctx.read().await;
    state
        .password_resets
        .iter()
        .find(|(_, entry)| entry.email == email)
        .map(|(token, _)| token.clone())
        .expect("a reset token should have been created for this email")
}

/// Proves the anti-enumeration property: the HTTP response is identical
/// (`204`) for a known and an unknown email, but a real token was only
/// actually created for the known one — confirmed via direct state access
/// since the response itself carries no signal either way.
#[tokio::test]
async fn forgot_password_always_returns_204_for_known_and_unknown_email() {
    let (app, ctx) = test_app_with_state().await;

    let known = send(
        &app,
        json_request(
            "POST",
            "/api/auth/forgot-password",
            None,
            serde_json::json!({ "email": DEMO_EMAIL }),
        ),
    )
    .await;
    assert_eq!(known.status(), StatusCode::NO_CONTENT);

    let unknown = send(
        &app,
        json_request(
            "POST",
            "/api/auth/forgot-password",
            None,
            serde_json::json!({ "email": UNKNOWN_EMAIL }),
        ),
    )
    .await;
    assert_eq!(unknown.status(), StatusCode::NO_CONTENT);

    let state = ctx.read().await;
    assert!(
        state
            .password_resets
            .values()
            .any(|entry| entry.email == DEMO_EMAIL),
        "expected a reset token to exist for the known email"
    );
    assert!(
        !state
            .password_resets
            .values()
            .any(|entry| entry.email == UNKNOWN_EMAIL),
        "no reset token should exist for an unknown email"
    );
}

#[tokio::test]
async fn reset_password_with_valid_token_changes_password() {
    let (app, ctx) = test_app_with_state().await;
    let token = request_reset_and_get_token(&app, &ctx, DEMO_EMAIL).await;

    let resp = send(
        &app,
        json_request(
            "POST",
            "/api/auth/reset-password",
            None,
            serde_json::json!({ "token": token, "newPassword": NEW_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let old_login = send(
        &app,
        json_request(
            "POST",
            "/api/auth/login",
            None,
            serde_json::json!({ "email": DEMO_EMAIL, "password": DEMO_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(
        old_login.status(),
        StatusCode::UNAUTHORIZED,
        "the old password must no longer work after a successful reset"
    );

    let new_login = send(
        &app,
        json_request(
            "POST",
            "/api/auth/login",
            None,
            serde_json::json!({ "email": DEMO_EMAIL, "password": NEW_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(
        new_login.status(),
        StatusCode::OK,
        "the new password must work after a successful reset"
    );
}

/// Same proof pattern as `change_password.rs`'s
/// `other_pre_existing_session_is_killed_by_a_password_change`: a session
/// established *before* the reset must not survive it.
#[tokio::test]
async fn reset_password_kills_pre_existing_sessions() {
    let (app, ctx) = test_app_with_state().await;

    let pre_existing_cookie = login_cookie(&app, DEMO_EMAIL).await;
    let me_before = send(
        &app,
        plain_request("GET", "/api/auth/me", Some(&pre_existing_cookie)),
    )
    .await;
    assert_eq!(me_before.status(), StatusCode::OK);

    let token = request_reset_and_get_token(&app, &ctx, DEMO_EMAIL).await;
    let reset_resp = send(
        &app,
        json_request(
            "POST",
            "/api/auth/reset-password",
            None,
            serde_json::json!({ "token": token, "newPassword": NEW_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(reset_resp.status(), StatusCode::NO_CONTENT);

    let me_after = send(
        &app,
        plain_request("GET", "/api/auth/me", Some(&pre_existing_cookie)),
    )
    .await;
    assert_eq!(
        me_after.status(),
        StatusCode::UNAUTHORIZED,
        "a session that existed before the password reset must not survive it"
    );
}

#[tokio::test]
async fn reset_password_with_unknown_token_is_401() {
    let (app, _ctx) = test_app_with_state().await;

    let resp = send(
        &app,
        json_request(
            "POST",
            "/api/auth/reset-password",
            None,
            serde_json::json!({ "token": "not-a-real-token", "newPassword": NEW_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// Proves the reset token is genuinely single-use: the first call succeeds,
/// a second call with the exact same token 401s.
#[tokio::test]
async fn reset_password_token_is_single_use() {
    let (app, ctx) = test_app_with_state().await;
    let token = request_reset_and_get_token(&app, &ctx, DEMO_EMAIL).await;

    let first = send(
        &app,
        json_request(
            "POST",
            "/api/auth/reset-password",
            None,
            serde_json::json!({ "token": token, "newPassword": NEW_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(first.status(), StatusCode::NO_CONTENT);

    let second = send(
        &app,
        json_request(
            "POST",
            "/api/auth/reset-password",
            None,
            serde_json::json!({ "token": token, "newPassword": "yet-another-password" }),
        ),
    )
    .await;
    assert_eq!(
        second.status(),
        StatusCode::UNAUTHORIZED,
        "an already-used reset token must not be reusable"
    );
}

#[tokio::test]
async fn reset_password_with_too_short_new_password_is_400_and_does_not_mutate_password() {
    let (app, ctx) = test_app_with_state().await;
    let token = request_reset_and_get_token(&app, &ctx, DEMO_EMAIL).await;

    let resp = send(
        &app,
        json_request(
            "POST",
            "/api/auth/reset-password",
            None,
            serde_json::json!({ "token": token, "newPassword": "short7x" }),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let old_login = send(
        &app,
        json_request(
            "POST",
            "/api/auth/login",
            None,
            serde_json::json!({ "email": DEMO_EMAIL, "password": DEMO_PASSWORD }),
        ),
    )
    .await;
    assert_eq!(
        old_login.status(),
        StatusCode::OK,
        "a rejected reset request must not partially mutate the password"
    );
}
