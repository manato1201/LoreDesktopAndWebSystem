use axum::http::StatusCode;
use sqlx::Row;

use super::{
    DEMO_EMAIL, SEEDED_SLUG, body_json, json_request, login_cookie, plain_request, send, test_app,
    test_app_with_state,
};

/// Base64-encodes `content` and POSTs it to `slug`'s upload endpoint,
/// asserting success. Shared by the new tests below that need real,
/// uploaded content behind a path before staging/committing it.
async fn upload(app: &axum::Router, cookie: &str, slug: &str, path: &str, content: &[u8]) {
    use base64::Engine;
    let resp = send(
        app,
        json_request(
            "POST",
            &format!("/api/repositories/{slug}/upload"),
            Some(cookie),
            serde_json::json!({
                "path": path,
                "contentBase64": base64::engine::general_purpose::STANDARD.encode(content),
            }),
        ),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "upload of {path} should succeed"
    );
}

/// Stages `path` with `change_type` ("added"/"modified"/"deleted") in
/// `slug`, asserting the stage call itself succeeded.
async fn stage(app: &axum::Router, cookie: &str, slug: &str, path: &str, change_type: &str) {
    let resp = send(
        app,
        json_request(
            "POST",
            &format!("/api/repositories/{slug}/tree/stage"),
            Some(cookie),
            serde_json::json!({ "path": path, "changeType": change_type, "staged": true }),
        ),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "staging {path} should succeed"
    );
}

/// Commits whatever is currently staged in `slug`, asserting the commit
/// succeeded, and returns the parsed `Commit` JSON.
async fn commit(app: &axum::Router, cookie: &str, slug: &str, message: &str) -> serde_json::Value {
    let resp = send(
        app,
        json_request(
            "POST",
            &format!("/api/repositories/{slug}/commits"),
            Some(cookie),
            serde_json::json!({ "message": message }),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED, "commit should succeed");
    body_json(resp).await
}

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

    // Upload real content for the path first — under the new
    // content-addressed validation, staging a path with no uploaded content
    // and then trying to commit it is rejected with 400 (see
    // `committing_without_uploaded_content_is_400`). A real client always
    // uploads before staging, so this is the realistic sequence.
    use base64::Engine;
    let content = b"hello from a brand new file\n";
    let upload_resp = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/upload"),
            Some(&cookie),
            serde_json::json!({
                "path": "Source/NewFile.txt",
                "contentBase64": base64::engine::general_purpose::STANDARD.encode(content),
            }),
        ),
    )
    .await;
    assert_eq!(upload_resp.status(), StatusCode::CREATED);

    // Stage the (now-uploaded) change, then commit it.
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
    // The new commit's own changed_files should carry a real, computed
    // sizeDeltaLabel, not the pre-redesign hardcoded "—".
    let changed_files = commit["changedFiles"].as_array().unwrap();
    let new_file_change = changed_files
        .iter()
        .find(|f| f["path"] == "Source/NewFile.txt")
        .expect("new file present in changed_files");
    assert_ne!(new_file_change["sizeDeltaLabel"], serde_json::json!("—"));

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

    // GET /tree reflects the newly committed file.
    let tree = body_json(
        send(
            &app,
            plain_request(
                "GET",
                &format!("/api/repositories/{SEEDED_SLUG}/tree"),
                Some(&cookie),
            ),
        )
        .await,
    )
    .await;
    assert!(
        tree_contains_path(&tree, "Source/NewFile.txt"),
        "GET /tree should show the newly committed file: {tree:#}"
    );
}

/// Recursively searches a `GET /tree` JSON response (a `Vec<TreeNode>`,
/// `Directory` nodes carrying a `children` array) for a leaf node whose
/// `path` equals `path`.
fn tree_contains_path(tree: &serde_json::Value, path: &str) -> bool {
    let Some(nodes) = tree.as_array() else {
        return false;
    };
    nodes.iter().any(|node| {
        if node["path"] == path {
            return true;
        }
        node.get("children")
            .is_some_and(|children| tree_contains_path(children, path))
    })
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

/// The single most important new test this phase adds: proves branch
/// checkout genuinely changes what `GET /tree` returns, rather than being a
/// no-op that just renames the currently-displayed (shared/static) tree the
/// way it was before this redesign. Builds two branches with real,
/// self-contained diverging content (rather than relying on `vcs_seed`'s
/// exact seeded paths staying the same forever) and checks out each in turn.
#[tokio::test]
async fn checkout_different_branch_changes_get_tree_result() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    // A file added directly on main, before the branches diverge — should
    // be visible from both branches afterwards.
    upload(
        &app,
        &cookie,
        SEEDED_SLUG,
        "BranchTest/on-both.txt",
        b"shared base content\n",
    )
    .await;
    stage(
        &app,
        &cookie,
        SEEDED_SLUG,
        "BranchTest/on-both.txt",
        "added",
    )
    .await;
    commit(
        &app,
        &cookie,
        SEEDED_SLUG,
        "Add a file common to both branches",
    )
    .await;

    // Branch off main into "diverge-a", then add a file that only exists on
    // that branch.
    let create_resp = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/branches"),
            Some(&cookie),
            serde_json::json!({ "name": "diverge-a" }),
        ),
    )
    .await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let checkout_a = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/checkout"),
            Some(&cookie),
            serde_json::json!({ "branch": "diverge-a" }),
        ),
    )
    .await;
    assert_eq!(checkout_a.status(), StatusCode::OK);

    upload(
        &app,
        &cookie,
        SEEDED_SLUG,
        "BranchTest/only-on-diverge-a.txt",
        b"only on diverge-a\n",
    )
    .await;
    stage(
        &app,
        &cookie,
        SEEDED_SLUG,
        "BranchTest/only-on-diverge-a.txt",
        "added",
    )
    .await;
    commit(&app, &cookie, SEEDED_SLUG, "Add a file only on diverge-a").await;

    let tree_on_diverge_a = body_json(
        send(
            &app,
            plain_request(
                "GET",
                &format!("/api/repositories/{SEEDED_SLUG}/tree"),
                Some(&cookie),
            ),
        )
        .await,
    )
    .await;
    assert!(
        tree_contains_path(&tree_on_diverge_a, "BranchTest/only-on-diverge-a.txt"),
        "diverge-a's tree should contain its own file: {tree_on_diverge_a:#}"
    );
    assert!(
        tree_contains_path(&tree_on_diverge_a, "BranchTest/on-both.txt"),
        "diverge-a's tree should still contain the pre-divergence shared file"
    );

    // Checking back out to main must show a GENUINELY DIFFERENT tree: the
    // shared file is still there, but diverge-a's own file must not be.
    let checkout_main = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/checkout"),
            Some(&cookie),
            serde_json::json!({ "branch": "main" }),
        ),
    )
    .await;
    assert_eq!(checkout_main.status(), StatusCode::OK);

    let tree_on_main = body_json(
        send(
            &app,
            plain_request(
                "GET",
                &format!("/api/repositories/{SEEDED_SLUG}/tree"),
                Some(&cookie),
            ),
        )
        .await,
    )
    .await;
    assert!(
        tree_contains_path(&tree_on_main, "BranchTest/on-both.txt"),
        "main's tree should contain the pre-divergence shared file"
    );
    assert!(
        !tree_contains_path(&tree_on_main, "BranchTest/only-on-diverge-a.txt"),
        "main's tree must NOT contain diverge-a's own file — trees must genuinely differ: {tree_on_main:#}"
    );
    assert_ne!(
        tree_on_diverge_a, tree_on_main,
        "GET /tree must return genuinely different content across branches"
    );
}

/// Direct-SQL proof that deleting a repository doesn't just remove the
/// `repositories` row but really does cascade every VCS table down to zero
/// rows for that slug (`ON DELETE CASCADE`, see migrations/
/// 0001_vcs_schema.sql), AND cleans up the filesystem blob directory the SQL
/// cascade can't reach on its own (see `handlers::delete_repository`).
#[tokio::test]
async fn deleting_repository_cascades_all_vcs_tables_to_zero_rows() {
    let (app, ctx) = test_app_with_state().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    // Populate every table this test asserts against: a real commit (which
    // exercises commits/commit_files/tree_entries/branches), a lock
    // (file_locks), and a still-pending, uncommitted stage (pending_changes
    // + staged_content).
    upload(
        &app,
        &cookie,
        SEEDED_SLUG,
        "DeleteTest/committed.txt",
        b"will be committed\n",
    )
    .await;
    stage(
        &app,
        &cookie,
        SEEDED_SLUG,
        "DeleteTest/committed.txt",
        "added",
    )
    .await;
    commit(
        &app,
        &cookie,
        SEEDED_SLUG,
        "Add a file before deleting the repo",
    )
    .await;

    upload(
        &app,
        &cookie,
        SEEDED_SLUG,
        "DeleteTest/still-pending.txt",
        b"never committed\n",
    )
    .await;
    stage(
        &app,
        &cookie,
        SEEDED_SLUG,
        "DeleteTest/still-pending.txt",
        "added",
    )
    .await;

    let lock_resp = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/tree/lock"),
            Some(&cookie),
            serde_json::json!({ "path": "DeleteTest/committed.txt", "lock": true }),
        ),
    )
    .await;
    assert_eq!(lock_resp.status(), StatusCode::OK);

    let blob_dir = ctx.blob_base_dir.join("blobs").join(SEEDED_SLUG);
    assert!(
        tokio::fs::metadata(&blob_dir).await.is_ok(),
        "blob directory should exist before delete"
    );

    let delete_resp = send(
        &app,
        plain_request(
            "DELETE",
            &format!("/api/repositories/{SEEDED_SLUG}"),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

    for table in [
        "commits",
        "branches",
        "tree_entries",
        "file_locks",
        "pending_changes",
        "staged_content",
        "commit_files",
        "file_blobs",
    ] {
        let row = sqlx::query(&format!(
            "SELECT COUNT(*) as count FROM {table} WHERE repo_slug = ?1"
        ))
        .bind(SEEDED_SLUG)
        .fetch_one(&ctx.db)
        .await
        .unwrap_or_else(|err| panic!("query {table}: {err}"));
        let count: i64 = row.get("count");
        assert_eq!(
            count, 0,
            "{table} should have zero rows for {SEEDED_SLUG} after delete"
        );
    }

    assert!(
        tokio::fs::metadata(&blob_dir).await.is_err(),
        "blob directory should be removed from disk after delete"
    );
}

/// The fix this whole redesign exists for: staging a path that was never
/// actually uploaded must not be committable — the pre-redesign backend let
/// this "succeed" over content that was never written anywhere.
#[tokio::test]
async fn committing_without_uploaded_content_is_400() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    // Stage a path with no prior upload at all.
    stage(
        &app,
        &cookie,
        SEEDED_SLUG,
        "Ghost/never-uploaded.bin",
        "added",
    )
    .await;

    let commit_resp = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/commits"),
            Some(&cookie),
            serde_json::json!({ "message": "should be rejected" }),
        ),
    )
    .await;
    assert_eq!(commit_resp.status(), StatusCode::BAD_REQUEST);

    let body = body_json(commit_resp).await;
    let error = body["error"].as_str().expect("error message present");
    assert!(
        error.contains("Ghost/never-uploaded.bin"),
        "400 error should name the offending path, got: {error}"
    );
}

/// Locking a path should be visible in `GET /tree`'s `lockedBy` field for the
/// matching node, and clear again after unlocking.
#[tokio::test]
async fn locking_and_unlocking_shows_up_in_get_tree() {
    let app = test_app().await;
    let cookie = login_cookie(&app, DEMO_EMAIL).await;

    // README.md is added in the very first seeded commit and never deleted
    // afterwards (see `vcs_seed`), so it's always present on main's head.
    let lock_resp = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/tree/lock"),
            Some(&cookie),
            serde_json::json!({ "path": "README.md", "lock": true }),
        ),
    )
    .await;
    assert_eq!(lock_resp.status(), StatusCode::OK);

    let tree_after_lock = body_json(
        send(
            &app,
            plain_request(
                "GET",
                &format!("/api/repositories/{SEEDED_SLUG}/tree"),
                Some(&cookie),
            ),
        )
        .await,
    )
    .await;
    let readme = find_tree_node(&tree_after_lock, "README.md")
        .unwrap_or_else(|| panic!("README.md node present in tree: {tree_after_lock:#}"));
    assert_eq!(readme["lockedBy"], serde_json::json!("Aiko Tanaka"));

    let unlock_resp = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/tree/lock"),
            Some(&cookie),
            serde_json::json!({ "path": "README.md", "lock": false }),
        ),
    )
    .await;
    assert_eq!(unlock_resp.status(), StatusCode::OK);

    let tree_after_unlock = body_json(
        send(
            &app,
            plain_request(
                "GET",
                &format!("/api/repositories/{SEEDED_SLUG}/tree"),
                Some(&cookie),
            ),
        )
        .await,
    )
    .await;
    let readme = find_tree_node(&tree_after_unlock, "README.md")
        .unwrap_or_else(|| panic!("README.md node present in tree: {tree_after_unlock:#}"));
    assert_eq!(readme["lockedBy"], serde_json::Value::Null);
}

/// Recursively finds the node at `path` within a `GET /tree` JSON response.
/// See [`tree_contains_path`] for the existence-only variant.
fn find_tree_node<'a>(tree: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let nodes = tree.as_array()?;
    for node in nodes {
        if node["path"] == path {
            return Some(node);
        }
        if let Some(children) = node.get("children")
            && let Some(found) = find_tree_node(children, path)
        {
            return Some(found);
        }
    }
    None
}
