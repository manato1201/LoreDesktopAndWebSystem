use axum::http::StatusCode;

use super::{DEMO_EMAIL, body_json, json_request, login_cookie, plain_request, send, test_app};

#[tokio::test]
async fn list_repositories_includes_seeded_repos() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let resp = send(
        &app,
        plain_request("GET", "/api/repositories", Some(&cookie)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let repos = body_json(resp).await;
    let slugs: Vec<&str> = repos
        .as_array()
        .expect("repositories response is a JSON array")
        .iter()
        .map(|r| r["slug"].as_str().expect("slug is a string"))
        .collect();
    assert!(slugs.contains(&"starforge-vfx"));
    assert!(slugs.contains(&"hollow-keep-env"));
}

#[tokio::test]
async fn create_repository_then_independently_fetchable() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let create_req = json_request(
        "POST",
        "/api/repositories",
        Some(&cookie),
        serde_json::json!({
            "name": "Brand New Repo",
            "description": "created by a test",
            "visibility": "private",
        }),
    );
    let create_resp = send(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = body_json(create_resp).await;
    let slug = created["slug"]
        .as_str()
        .expect("slug is a string")
        .to_string();
    assert_eq!(created["name"], "Brand New Repo");

    let get_resp = send(
        &app,
        plain_request("GET", &format!("/api/repositories/{slug}"), Some(&cookie)),
    )
    .await;
    assert_eq!(get_resp.status(), StatusCode::OK);
    let fetched = body_json(get_resp).await;
    assert_eq!(fetched["slug"], serde_json::json!(slug));
    assert_eq!(fetched["description"], "created by a test");
}

#[tokio::test]
async fn patch_updates_name_and_visibility() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let patch_req = json_request(
        "PATCH",
        "/api/repositories/starforge-vfx",
        Some(&cookie),
        serde_json::json!({ "name": "Renamed Repo", "visibility": "public" }),
    );
    let resp = send(&app, patch_req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let updated = body_json(resp).await;
    assert_eq!(updated["name"], "Renamed Repo");
    assert_eq!(updated["visibility"], "public");

    let get_resp = send(
        &app,
        plain_request("GET", "/api/repositories/starforge-vfx", Some(&cookie)),
    )
    .await;
    let fetched = body_json(get_resp).await;
    assert_eq!(fetched["name"], "Renamed Repo");
}

#[tokio::test]
async fn patch_rejects_empty_after_trim_name_with_400() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let patch_req = json_request(
        "PATCH",
        "/api/repositories/starforge-vfx",
        Some(&cookie),
        serde_json::json!({ "name": "   " }),
    );
    let resp = send(&app, patch_req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn patch_unknown_slug_is_404() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let patch_req = json_request(
        "PATCH",
        "/api/repositories/does-not-exist",
        Some(&cookie),
        serde_json::json!({ "name": "New Name" }),
    );
    let resp = send(&app, patch_req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_removes_repository_and_subsequent_get_is_404() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let delete_resp = send(
        &app,
        plain_request("DELETE", "/api/repositories/starforge-vfx", Some(&cookie)),
    )
    .await;
    assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

    let get_resp = send(
        &app,
        plain_request("GET", "/api/repositories/starforge-vfx", Some(&cookie)),
    )
    .await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    // And it should have dropped out of the list too.
    let list_resp = send(
        &app,
        plain_request("GET", "/api/repositories", Some(&cookie)),
    )
    .await;
    let repos = body_json(list_resp).await;
    let slugs: Vec<&str> = repos
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["slug"].as_str().unwrap())
        .collect();
    assert!(!slugs.contains(&"starforge-vfx"));
}

#[tokio::test]
async fn delete_unknown_slug_is_404() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let resp = send(
        &app,
        plain_request("DELETE", "/api/repositories/does-not-exist", Some(&cookie)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
