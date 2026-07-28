use axum::http::StatusCode;

use super::{
    DEMO_EMAIL, SEEDED_SLUG, body_json, json_request, login_cookie, plain_request, send, test_app,
};

#[tokio::test]
async fn staged_change_appears_in_pending() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let stage_req = json_request(
        "POST",
        &format!("/api/repositories/{SEEDED_SLUG}/tree/stage"),
        Some(&cookie),
        serde_json::json!({ "path": "Source/NewFile.txt", "changeType": "added", "staged": true }),
    );
    let stage_resp = send(&app, stage_req).await;
    assert_eq!(stage_resp.status(), StatusCode::OK);

    let pending_resp = send(
        &app,
        plain_request(
            "GET",
            &format!("/api/repositories/{SEEDED_SLUG}/pending"),
            Some(&cookie),
        ),
    )
    .await;
    let pending = body_json(pending_resp).await;
    let paths: Vec<&str> = pending
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["path"].as_str().unwrap())
        .collect();
    assert!(paths.contains(&"Source/NewFile.txt"));
}

#[tokio::test]
async fn unstaging_removes_the_pending_entry() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;
    let uri = format!("/api/repositories/{SEEDED_SLUG}/tree/stage");

    send(
        &app,
        json_request(
            "POST",
            &uri,
            Some(&cookie),
            serde_json::json!({ "path": "Source/NewFile.txt", "changeType": "added", "staged": true }),
        ),
    )
    .await;
    send(
        &app,
        json_request(
            "POST",
            &uri,
            Some(&cookie),
            serde_json::json!({ "path": "Source/NewFile.txt", "changeType": "added", "staged": false }),
        ),
    )
    .await;

    let pending_resp = send(
        &app,
        plain_request(
            "GET",
            &format!("/api/repositories/{SEEDED_SLUG}/pending"),
            Some(&cookie),
        ),
    )
    .await;
    let pending = body_json(pending_resp).await;
    assert_eq!(pending.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn commit_with_no_pending_changes_is_400() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let commit_req = json_request(
        "POST",
        &format!("/api/repositories/{SEEDED_SLUG}/commits"),
        Some(&cookie),
        serde_json::json!({ "message": "nothing staged" }),
    );
    let resp = send(&app, commit_req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn commit_creates_commit_clears_pending_and_updates_branch_head() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    // Read the current branch head before committing, so we can assert the
    // new commit's `parents` and the branch's post-commit `head` both match
    // it.
    let branches_before = body_json(
        send(
            &app,
            plain_request(
                "GET",
                &format!("/api/repositories/{SEEDED_SLUG}/branches"),
                Some(&cookie),
            ),
        )
        .await,
    )
    .await;
    let main_before = branches_before
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == "main")
        .expect("main branch present");
    let old_head = main_before["head"].as_str().unwrap().to_string();

    // Stage a change, then commit it.
    send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/tree/stage"),
            Some(&cookie),
            serde_json::json!({ "path": "Source/NewFile.txt", "changeType": "added", "staged": true }),
        ),
    )
    .await;

    let commit_resp = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/commits"),
            Some(&cookie),
            serde_json::json!({ "message": "Add NewFile.txt", "description": "" }),
        ),
    )
    .await;
    assert_eq!(commit_resp.status(), StatusCode::CREATED);
    let commit = body_json(commit_resp).await;
    assert_eq!(commit["branch"], "main");
    assert_eq!(commit["parents"], serde_json::json!([old_head]));
    let new_hash = commit["hash"].as_str().unwrap().to_string();

    // The new commit shows up in the repo's commit list.
    let commits = body_json(
        send(
            &app,
            plain_request(
                "GET",
                &format!("/api/repositories/{SEEDED_SLUG}/commits"),
                Some(&cookie),
            ),
        )
        .await,
    )
    .await;
    let hashes: Vec<&str> = commits
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["hash"].as_str().unwrap())
        .collect();
    assert!(hashes.contains(&new_hash.as_str()));

    // Pending changes are cleared.
    let pending = body_json(
        send(
            &app,
            plain_request(
                "GET",
                &format!("/api/repositories/{SEEDED_SLUG}/pending"),
                Some(&cookie),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(pending.as_array().unwrap().len(), 0);

    // main's head advanced to the new commit.
    let branches_after = body_json(
        send(
            &app,
            plain_request(
                "GET",
                &format!("/api/repositories/{SEEDED_SLUG}/branches"),
                Some(&cookie),
            ),
        )
        .await,
    )
    .await;
    let main_after = branches_after
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == "main")
        .unwrap();
    assert_eq!(main_after["head"], serde_json::json!(new_hash));
}

#[tokio::test]
async fn branch_create_and_checkout_changes_current_branch() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let current_before = body_json(
        send(
            &app,
            plain_request(
                "GET",
                &format!("/api/repositories/{SEEDED_SLUG}/branches/current"),
                Some(&cookie),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(current_before["branch"], "main");

    let create_resp = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/branches"),
            Some(&cookie),
            serde_json::json!({ "name": "feature/test-branch" }),
        ),
    )
    .await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let checkout_resp = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/checkout"),
            Some(&cookie),
            serde_json::json!({ "branch": "feature/test-branch" }),
        ),
    )
    .await;
    assert_eq!(checkout_resp.status(), StatusCode::OK);

    let current_after = body_json(
        send(
            &app,
            plain_request(
                "GET",
                &format!("/api/repositories/{SEEDED_SLUG}/branches/current"),
                Some(&cookie),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(current_after["branch"], "feature/test-branch");
}

#[tokio::test]
async fn checkout_unknown_branch_is_404() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    let resp = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/checkout"),
            Some(&cookie),
            serde_json::json!({ "branch": "does-not-exist" }),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
