use axum::http::StatusCode;

use super::{DEMO_EMAIL, body_json, json_request, login_cookie, plain_request, send, test_app};

#[tokio::test]
async fn put_entries_merges_by_path_leaving_other_paths_untouched() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let before = body_json(
        send(
            &app,
            plain_request("GET", "/api/access-control/entries", Some(&cookie)),
        )
        .await,
    )
    .await;
    // Sanity check on the seeded fixture: both of these paths exist before
    // the PUT below only touches "Assets".
    assert!(before.get("Assets").is_some());
    assert!(before.get("Source").is_some());
    let source_before = before["Source"].clone();

    let put_req = json_request(
        "PUT",
        "/api/access-control/entries",
        Some(&cookie),
        serde_json::json!({
            "Assets": [
                {
                    "principal": "Solo Tester",
                    "principalType": "user",
                    "permissions": ["read"]
                }
            ]
        }),
    );
    let resp = send(&app, put_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let after = body_json(
        send(
            &app,
            plain_request("GET", "/api/access-control/entries", Some(&cookie)),
        )
        .await,
    )
    .await;

    // "Assets" was replaced with exactly what we sent.
    let assets_after = after["Assets"].as_array().unwrap();
    assert_eq!(assets_after.len(), 1);
    assert_eq!(assets_after[0]["principal"], "Solo Tester");

    // "Source" — not mentioned in the PUT body — is byte-for-byte untouched.
    assert_eq!(after["Source"], source_before);
}

#[tokio::test]
async fn put_entries_can_add_a_brand_new_path() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let put_req = json_request(
        "PUT",
        "/api/access-control/entries",
        Some(&cookie),
        serde_json::json!({
            "Assets/BrandNewFolder": [
                {
                    "principal": "New Team",
                    "principalType": "team",
                    "permissions": ["read", "write"]
                }
            ]
        }),
    );
    send(&app, put_req).await;

    let after = body_json(
        send(
            &app,
            plain_request("GET", "/api/access-control/entries", Some(&cookie)),
        )
        .await,
    )
    .await;
    assert!(after.get("Assets/BrandNewFolder").is_some());
}

#[tokio::test]
async fn toggle_permission_flips_it_on_an_existing_entry() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    // Seeded fixture: "Assets" / "Environment Artists" starts with
    // [read, write] — toggling "write" should remove it.
    let toggle_req = json_request(
        "POST",
        "/api/access-control/entries/toggle",
        Some(&cookie),
        serde_json::json!({ "path": "Assets", "principal": "Environment Artists", "level": "write" }),
    );
    let resp = send(&app, toggle_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let entries = body_json(resp).await;
    let entry = entries
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["principal"] == "Environment Artists")
        .expect("principal present");
    let permissions: Vec<&str> = entry["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    assert!(!permissions.contains(&"write"));
    assert!(permissions.contains(&"read"));

    // Toggling again should add it back.
    let toggle_again = json_request(
        "POST",
        "/api/access-control/entries/toggle",
        Some(&cookie),
        serde_json::json!({ "path": "Assets", "principal": "Environment Artists", "level": "write" }),
    );
    let entries_again = body_json(send(&app, toggle_again).await).await;
    let entry_again = entries_again
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["principal"] == "Environment Artists")
        .unwrap();
    let permissions_again: Vec<&str> = entry_again["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    assert!(permissions_again.contains(&"write"));
}

#[tokio::test]
async fn toggle_permission_unknown_path_is_404() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let resp = send(
        &app,
        json_request(
            "POST",
            "/api/access-control/entries/toggle",
            Some(&cookie),
            serde_json::json!({ "path": "Does/Not/Exist", "principal": "Anyone", "level": "read" }),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
