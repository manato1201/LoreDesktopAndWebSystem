use axum::http::StatusCode;

use super::{
    DEMO_EMAIL, cookie_max_age, cookie_pair, json_request, login, plain_request, send, set_cookies,
    test_app,
};
use crate::auth::{ACCESS_TOKEN_TTL_SECS, REFRESH_COOKIE, REFRESH_TOKEN_TTL_SECS, SESSION_COOKIE};

#[tokio::test]
async fn login_success_sets_two_distinct_cookies_with_correct_max_ages() {
    let app = test_app().await;
    let (_combined, access, refresh) = login(&app, DEMO_EMAIL).await;

    let (access_name, access_value) = cookie_pair(&access);
    let (refresh_name, refresh_value) = cookie_pair(&refresh);

    assert_eq!(access_name, SESSION_COOKIE);
    assert_eq!(refresh_name, REFRESH_COOKIE);
    assert_ne!(
        access_value, refresh_value,
        "access and refresh tokens must be distinct"
    );

    assert_eq!(cookie_max_age(&access), ACCESS_TOKEN_TTL_SECS);
    assert_eq!(cookie_max_age(&refresh), REFRESH_TOKEN_TTL_SECS);
}

/// Confirms the real login flow wires `auth::session_cookie`/
/// `refresh_cookie` up to actually include `Secure` when
/// `LOREHUB_INSECURE_COOKIES` is unset (the default). This deliberately
/// does NOT try to also exercise the opt-out (`LOREHUB_INSECURE_COOKIES=true`)
/// path here: that flag is read once into a process-wide `OnceLock` (see
/// `auth::cookies_secure`), so a test that set the env var would race every
/// other test in this binary that also builds a cookie — effectively all of
/// them, via the `login` helper. The opt-out path is covered instead by a
/// narrower unit test of the pure cookie-string builder in `auth.rs`
/// (`cookie_string_omits_secure_when_not_requested`), which takes the
/// `secure` flag as a plain argument and never touches the env var at all.
#[tokio::test]
async fn login_cookies_are_secure_by_default() {
    let app = test_app().await;
    let (_combined, access, refresh) = login(&app, DEMO_EMAIL).await;

    assert!(
        access.contains("; Secure"),
        "access cookie should carry Secure by default: {access}"
    );
    assert!(
        refresh.contains("; Secure"),
        "refresh cookie should carry Secure by default: {refresh}"
    );
}

#[tokio::test]
async fn login_wrong_password_is_401() {
    let app = test_app().await;
    let req = json_request(
        "POST",
        "/api/auth/login",
        None,
        serde_json::json!({ "email": DEMO_EMAIL, "password": "not-the-right-password" }),
    );
    let resp = send(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_unknown_email_is_401() {
    let app = test_app().await;
    let req = json_request(
        "POST",
        "/api/auth/login",
        None,
        serde_json::json!({ "email": "nobody@nebula.studio", "password": "lorehub" }),
    );
    let resp = send(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn protected_endpoint_without_any_cookie_is_401() {
    let app = test_app().await;
    let resp = send(&app, plain_request("GET", "/api/auth/me", None)).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn protected_endpoint_with_garbage_cookie_is_401() {
    let app = test_app().await;
    let resp = send(
        &app,
        plain_request(
            "GET",
            "/api/auth/me",
            Some("lorehub_token=this-token-does-not-exist"),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_rotates_tokens_and_old_refresh_token_is_rejected_on_reuse() {
    let app = test_app().await;
    let (_combined, _access, refresh) = login(&app, DEMO_EMAIL).await;
    let (_name, old_refresh_value) = cookie_pair(&refresh);

    // First use of the refresh token: should succeed and hand back a brand
    // new access+refresh pair.
    let refresh_req = plain_request(
        "POST",
        "/api/auth/refresh",
        Some(&format!("{REFRESH_COOKIE}={old_refresh_value}")),
    );
    let resp = send(&app, refresh_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let new_cookies = set_cookies(&resp);
    assert_eq!(new_cookies.len(), 2);
    let new_access = new_cookies
        .iter()
        .find(|c| c.starts_with(&format!("{SESSION_COOKIE}=")))
        .expect("new access cookie present");
    let new_refresh = new_cookies
        .iter()
        .find(|c| c.starts_with(&format!("{REFRESH_COOKIE}=")))
        .expect("new refresh cookie present");
    let (_, new_refresh_value) = cookie_pair(new_refresh);
    assert_ne!(
        new_refresh_value, old_refresh_value,
        "refresh should rotate to a brand new refresh token"
    );

    // The new access token should actually authenticate a protected route.
    let (_, new_access_value) = cookie_pair(new_access);
    let me_cookie = format!("{SESSION_COOKIE}={new_access_value}");
    let me_resp = send(&app, plain_request("GET", "/api/auth/me", Some(&me_cookie))).await;
    assert_eq!(me_resp.status(), StatusCode::OK);

    // Reusing the OLD refresh token must now be rejected — it was
    // invalidated the instant it was used above.
    let reuse_req = plain_request(
        "POST",
        "/api/auth/refresh",
        Some(&format!("{REFRESH_COOKIE}={old_refresh_value}")),
    );
    let reuse_resp = send(&app, reuse_req).await;
    assert_eq!(reuse_resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_without_a_refresh_cookie_is_401() {
    let app = test_app().await;
    let resp = send(&app, plain_request("POST", "/api/auth/refresh", None)).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn logout_clears_both_cookies_and_invalidates_the_session() {
    let app = test_app().await;
    let (combined, _access, _refresh) = login(&app, DEMO_EMAIL).await;

    let logout_req = plain_request("POST", "/api/auth/logout", Some(&combined));
    let resp = send(&app, logout_req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let cleared = set_cookies(&resp);
    assert_eq!(cleared.len(), 2);
    for c in &cleared {
        assert_eq!(
            cookie_max_age(c),
            0,
            "logout cookies must expire immediately"
        );
    }

    // The now-logged-out session cookie should no longer authenticate.
    let me_resp = send(&app, plain_request("GET", "/api/auth/me", Some(&combined))).await;
    assert_eq!(me_resp.status(), StatusCode::UNAUTHORIZED);
}
