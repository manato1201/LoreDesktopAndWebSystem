//! Integration tests for `authz::check_path_permission` and the RBAC role
//! gates added in handlers.rs, driven end-to-end through the real router
//! (see `tests/mod.rs`).
//!
//! Seeded fixture recap (`state::seed()`):
//! - Aiko Tanaka (`DEMO_EMAIL`) — Owner
//! - Marco Silva (`ADMIN_EMAIL`) — Admin, team "Environment Artists"
//! - Priya Desai (`CHARACTER_ARTIST_EMAIL`) — Member, team "Character Artists"
//! - Diego Fernandez (`QA_CONTRACTOR_EMAIL`) — Member, team "QA Contractors"
//!
//! Seeded `access_entries`: "Assets" (Environment Artists RW, Character
//! Artists RW, QA Contractors R), "Assets/Characters" (Character Artists
//! RWL, Aiko Tanaka user RWL), "Assets/Environments" (Environment Artists
//! RWL), "Source" (Engineering RWL, QA Contractors R). No entries at all for
//! "Assets/Audio" or "README.md".
use axum::http::StatusCode;

use super::{
    DEMO_EMAIL, OTHER_SEEDED_SLUG, SEEDED_SLUG, body_json, json_request, login_cookie,
    plain_request, send, test_app,
};

const ADMIN_EMAIL: &str = "marco.silva@nebula.studio";
const CHARACTER_ARTIST_EMAIL: &str = "priya.desai@nebula.studio";
const QA_CONTRACTOR_EMAIL: &str = "diego.fernandez@nebula.studio";

// ---------------------------------------------------------------------
// Read: get_image / get_file_content / get_audio / get_image_before
// ---------------------------------------------------------------------

/// The negative case that proves enforcement is real, not just present:
/// Diego (QA Contractors) is not on the "Character Artists" team and has no
/// personal grant on "Assets/Characters", so reading a file under it must
/// be denied — and the denial itself must be visible in the audit log.
#[tokio::test]
async fn read_denied_for_member_without_matching_team_or_user_entry() {
    let app = test_app().await;
    let cookie = login_cookie(&app, QA_CONTRACTOR_EMAIL).await;

    let resp = send(
        &app,
        plain_request(
            "GET",
            &format!("/api/repositories/{SEEDED_SLUG}/image/Assets/Characters/hero_diffuse.png"),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Denials are audited too, not just successes.
    let owner_cookie = login_cookie(&app, DEMO_EMAIL).await;
    let audit = body_json(
        send(
            &app,
            plain_request("GET", "/api/org/audit-log", Some(&owner_cookie)),
        )
        .await,
    )
    .await;
    let entries = audit.as_array().unwrap();
    assert!(entries.iter().any(|e| {
        e["actor"] == "Diego Fernandez"
            && e["action"] == "denied read access to"
            && e["target"] == "Assets/Characters/hero_diffuse.png"
    }));
}

/// The positive case: Priya *is* on "Character Artists", which has Read on
/// "Assets/Characters" — same path as above, but must succeed.
#[tokio::test]
async fn read_allowed_for_member_with_matching_team_entry() {
    let app = test_app().await;
    let cookie = login_cookie(&app, CHARACTER_ARTIST_EMAIL).await;

    let resp = send(
        &app,
        plain_request(
            "GET",
            &format!("/api/repositories/{SEEDED_SLUG}/image/Assets/Characters/hero_diffuse.png"),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
}

/// `Owner`/`Admin` bypass ACL entirely: Marco (Admin) has no team or user
/// grant on "Assets/Characters" at all, yet must still succeed.
#[tokio::test]
async fn owner_and_admin_bypass_acl_entirely() {
    let app = test_app().await;
    let cookie = login_cookie(&app, ADMIN_EMAIL).await;

    let resp = send(
        &app,
        plain_request(
            "GET",
            &format!("/api/repositories/{SEEDED_SLUG}/image/Assets/Characters/hero_diffuse.png"),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
}

/// Critical default-allow case: "README.md" has no ancestor with any
/// configured `AccessEntry` at all, so it must stay openly readable by any
/// authenticated member — getting this backwards would break every
/// existing unconfigured repo/path.
#[tokio::test]
async fn path_with_no_configured_entries_is_open_by_default() {
    let app = test_app().await;
    let cookie = login_cookie(&app, QA_CONTRACTOR_EMAIL).await;

    let resp = send(
        &app,
        plain_request(
            "GET",
            &format!("/api/repositories/{SEEDED_SLUG}/content/README.md"),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------
// User-principal matching (as opposed to Team-principal matching)
// ---------------------------------------------------------------------

/// Proves the `PrincipalType::User` branch of the match, isolated from any
/// team-based grant: "Assets/Audio" starts with zero configured entries, so
/// an Admin grants Diego personally a Write-only entry there. Diego's only
/// team (QA Contractors) has nothing above Read anywhere, so the only way
/// this Write succeeds is via the personal grant.
#[tokio::test]
async fn write_allowed_for_member_with_matching_user_principal_entry() {
    let app = test_app().await;
    let admin_cookie = login_cookie(&app, ADMIN_EMAIL).await;

    let put_resp = send(
        &app,
        json_request(
            "PUT",
            "/api/access-control/entries",
            Some(&admin_cookie),
            serde_json::json!({
                "Assets/Audio": [
                    {
                        "principal": "Diego Fernandez",
                        "principalType": "user",
                        "permissions": ["write"]
                    }
                ]
            }),
        ),
    )
    .await;
    assert_eq!(put_resp.status(), StatusCode::OK);

    let diego_cookie = login_cookie(&app, QA_CONTRACTOR_EMAIL).await;
    let stage_resp = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/tree/stage"),
            Some(&diego_cookie),
            serde_json::json!({
                "path": "Assets/Audio/theme_main.wav",
                "changeType": "modified",
                "staged": true
            }),
        ),
    )
    .await;
    assert_eq!(stage_resp.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------
// Write: tree/stage and upload
// ---------------------------------------------------------------------

#[tokio::test]
async fn write_permission_enforced_on_stage() {
    let app = test_app().await;

    let diego_cookie = login_cookie(&app, QA_CONTRACTOR_EMAIL).await;
    let denied = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/tree/stage"),
            Some(&diego_cookie),
            serde_json::json!({
                "path": "Assets/Characters/new_file.png",
                "changeType": "added",
                "staged": true
            }),
        ),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let priya_cookie = login_cookie(&app, CHARACTER_ARTIST_EMAIL).await;
    let allowed = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/tree/stage"),
            Some(&priya_cookie),
            serde_json::json!({
                "path": "Assets/Characters/new_file.png",
                "changeType": "added",
                "staged": true
            }),
        ),
    )
    .await;
    assert_eq!(allowed.status(), StatusCode::OK);
}

#[tokio::test]
async fn write_permission_enforced_on_upload() {
    use base64::Engine;
    let app = test_app().await;
    let b64 = base64::engine::general_purpose::STANDARD.encode(b"hello");

    let diego_cookie = login_cookie(&app, QA_CONTRACTOR_EMAIL).await;
    let denied = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/upload"),
            Some(&diego_cookie),
            serde_json::json!({ "path": "Assets/Characters/new_upload.png", "contentBase64": b64 }),
        ),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let priya_cookie = login_cookie(&app, CHARACTER_ARTIST_EMAIL).await;
    let allowed = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/upload"),
            Some(&priya_cookie),
            serde_json::json!({ "path": "Assets/Characters/new_upload.png", "contentBase64": b64 }),
        ),
    )
    .await;
    assert_eq!(allowed.status(), StatusCode::CREATED);
}

// ---------------------------------------------------------------------
// Lock: tree/lock
// ---------------------------------------------------------------------

#[tokio::test]
async fn lock_permission_enforced_on_toggle_lock() {
    let app = test_app().await;

    let diego_cookie = login_cookie(&app, QA_CONTRACTOR_EMAIL).await;
    let denied = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/tree/lock"),
            Some(&diego_cookie),
            serde_json::json!({ "path": "Assets/Characters/hero_diffuse.png", "lock": true }),
        ),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let priya_cookie = login_cookie(&app, CHARACTER_ARTIST_EMAIL).await;
    let allowed = send(
        &app,
        json_request(
            "POST",
            &format!("/api/repositories/{SEEDED_SLUG}/tree/lock"),
            Some(&priya_cookie),
            serde_json::json!({ "path": "Assets/Characters/hero_diffuse.png", "lock": true }),
        ),
    )
    .await;
    assert_eq!(allowed.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------
// RBAC: role-gated org-level actions (Member never passes, regardless of
// team membership; Admin/Owner always do)
// ---------------------------------------------------------------------

#[tokio::test]
async fn delete_repository_requires_admin_or_owner() {
    let app = test_app().await;

    let diego_cookie = login_cookie(&app, QA_CONTRACTOR_EMAIL).await;
    let denied = send(
        &app,
        plain_request(
            "DELETE",
            &format!("/api/repositories/{OTHER_SEEDED_SLUG}"),
            Some(&diego_cookie),
        ),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    // Still there — the denied attempt didn't leak through.
    let admin_cookie = login_cookie(&app, ADMIN_EMAIL).await;
    let still_there = send(
        &app,
        plain_request(
            "GET",
            &format!("/api/repositories/{OTHER_SEEDED_SLUG}"),
            Some(&admin_cookie),
        ),
    )
    .await;
    assert_eq!(still_there.status(), StatusCode::OK);

    let allowed = send(
        &app,
        plain_request(
            "DELETE",
            &format!("/api/repositories/{OTHER_SEEDED_SLUG}"),
            Some(&admin_cookie),
        ),
    )
    .await;
    assert_eq!(allowed.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn apply_access_entries_requires_admin_or_owner() {
    let app = test_app().await;
    let body = serde_json::json!({
        "Assets/Characters": [
            {
                "principal": "New Grant",
                "principalType": "team",
                "permissions": ["read"]
            }
        ]
    });

    let diego_cookie = login_cookie(&app, QA_CONTRACTOR_EMAIL).await;
    let denied = send(
        &app,
        json_request(
            "PUT",
            "/api/access-control/entries",
            Some(&diego_cookie),
            body.clone(),
        ),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let admin_cookie = login_cookie(&app, ADMIN_EMAIL).await;
    let allowed = send(
        &app,
        json_request(
            "PUT",
            "/api/access-control/entries",
            Some(&admin_cookie),
            body,
        ),
    )
    .await;
    assert_eq!(allowed.status(), StatusCode::OK);
}

/// Also covers the self-escalation angle explicitly: Diego attempting to
/// promote *himself* to Admin must be denied by the same unconditional
/// "caller must be Owner or Admin" rule, not by some path-dependent check
/// that a different target email might slip past.
#[tokio::test]
async fn update_member_role_requires_admin_or_owner_and_blocks_self_escalation() {
    let app = test_app().await;

    let diego_cookie = login_cookie(&app, QA_CONTRACTOR_EMAIL).await;
    let denied = send(
        &app,
        json_request(
            "PATCH",
            &format!("/api/org/members/{QA_CONTRACTOR_EMAIL}"),
            Some(&diego_cookie),
            serde_json::json!({ "role": "admin" }),
        ),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let admin_cookie = login_cookie(&app, ADMIN_EMAIL).await;
    let allowed = send(
        &app,
        json_request(
            "PATCH",
            &format!("/api/org/members/{QA_CONTRACTOR_EMAIL}"),
            Some(&admin_cookie),
            serde_json::json!({ "role": "admin" }),
        ),
    )
    .await;
    assert_eq!(allowed.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------
// access_entries scope: documents the confirmed-intentional current
// behavior (see authz.rs's doc comment on `check_path_permission` for the
// full reasoning) — it is a single flat, org-wide path table, not
// repo-slug-keyed like tree/commits/branches. This test exists so a future
// change to that scope (e.g. if the two clients ever start sending a repo
// slug) has to consciously update or remove it rather than silently
// regressing either way.
// ---------------------------------------------------------------------

#[tokio::test]
async fn access_entries_apply_identically_across_repositories_by_current_design() {
    let app = test_app().await;
    let cookie = login_cookie(&app, QA_CONTRACTOR_EMAIL).await;

    let on_seeded = send(
        &app,
        plain_request(
            "GET",
            &format!("/api/repositories/{SEEDED_SLUG}/image/Assets/Characters/hero_diffuse.png"),
            Some(&cookie),
        ),
    )
    .await;
    let on_other = send(
        &app,
        plain_request(
            "GET",
            &format!(
                "/api/repositories/{OTHER_SEEDED_SLUG}/image/Assets/Characters/hero_diffuse.png"
            ),
            Some(&cookie),
        ),
    )
    .await;

    assert_eq!(on_seeded.status(), StatusCode::FORBIDDEN);
    assert_eq!(on_other.status(), StatusCode::FORBIDDEN);
}
