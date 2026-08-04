//! Synthesizes real, content-addressed demo commit history for every seeded
//! repository (see `repo_store::seeded_slugs`) at startup, the first time it
//! has none.
//!
//! Before this phase, every seeded repository shared one hand-authored,
//! never-actually-hashed `demo_tree()`/`demo_commits()`/`demo_branches()`
//! dataset (see the removed functions in `state.rs`'s git history). This
//! module rebuilds the same general shape — the same file paths/names, the
//! same rough commit narrative, the same branch names — but as genuine
//! commits: real bytes written through `blob_store`/`blob_meta_store`, real
//! SHA-256 `content_hash`es, and real commits built via `vcs_store::
//! create_commit` (the exact same transactional path `POST /commits` uses
//! live — nothing here duplicates that logic, it just calls it with
//! backdated timestamps instead of "now").
//!
//! ## The dropped merge commit
//!
//! The old demo history had one commit with **two** parents ("Merge branch
//! 'feature/dusk-skybox' into main"). The new schema is single-parent-only
//! (`commits.parent_hash` is a single nullable column, not a list — see
//! `migrations/0001_vcs_schema.sql`), so a merge commit literally cannot be
//! represented here. Per the approved redesign plan, this is reconstructed
//! as two independently diverging single-parent branches instead:
//! `feature/dusk-skybox` forks off `main` and gets its skybox-adding commit,
//! but **never merges back** — `main` continues on its own straight past the
//! fork point with the "fix world tick order" commit. The result covers the
//! same demo narrative (a feature branch that diverged and did its own
//! thing) without pretending to be a DAG this VCS doesn't support.
//!
//! Idempotent per repository: `seed_demo_history` skips any slug that
//! already has at least one row in `commits` (checked via
//! `vcs_store::commit_count`), so restarting a real server against its
//! existing `lorehub.db` never re-seeds or duplicates history — this is what
//! makes it safe to call unconditionally on every startup (see `main.rs`).

use std::path::Path;

use sqlx::SqlitePool;

use crate::models::FileChangeType;
use crate::{auth, blob_meta_store, blob_store, content_hash, repo_store, vcs_store};

/// `(name, email, initials)` — matches the seeded `OrgMember`s in
/// `state::seed()`, so seeded commit authorship lines up with an org member
/// that actually exists in the demo dataset.
type Author = (&'static str, &'static str, &'static str);
const AIKO: Author = ("Aiko Tanaka", "aiko.tanaka@nebula.studio", "AT");
const MARCO: Author = ("Marco Silva", "marco.silva@nebula.studio", "MS");
const PRIYA: Author = ("Priya Desai", "priya.desai@nebula.studio", "PD");

/// Hashes+saves `content` as a blob and stages it as `path`/`change_type` —
/// the seed-time equivalent of "upload, then stage" a real client does via
/// `POST /upload` + `POST /tree/stage`, minus the HTTP round trip.
async fn stage_file(
    pool: &SqlitePool,
    base_dir: &Path,
    repo_slug: &str,
    path: &str,
    change_type: FileChangeType,
    content: &[u8],
) {
    let hash = content_hash::sha256_hex(content);
    blob_store::save_blob(base_dir, repo_slug, &hash, content)
        .await
        .expect("seed: writing a placeholder blob to disk should never fail");
    blob_meta_store::record_blob(pool, repo_slug, &hash, content.len() as i64).await;
    vcs_store::upsert_pending_change(
        pool,
        repo_slug,
        path,
        change_type,
        Some(&hash),
        Some(content.len() as i64),
    )
    .await;
}

/// Commits whatever is currently staged (via [`stage_file`]) for `repo_slug`
/// on `branch`, backdated to `created_at`. Calls the exact same
/// `vcs_store::create_commit` the live `POST /commits` handler uses.
async fn commit_seed(
    pool: &SqlitePool,
    repo_slug: &str,
    branch: &str,
    author: Author,
    message: &str,
    description: Option<&str>,
    created_at: i64,
) {
    let new = vcs_store::NewCommit {
        repo_slug,
        branch_name: branch,
        author_email: author.1,
        author_name: author.0,
        author_initials: author.2,
        message,
        description,
        created_at,
    };
    vcs_store::create_commit(pool, new)
        .await
        .expect("seed commit has real uploaded content and a valid branch — should never fail");
}

/// Builds the fixed 7-commit / 3-branch demo history described in the module
/// doc comment for a single repository. Every seeded repository gets an
/// identical copy of this history (paths, content, messages) — the same
/// "one shared demo dataset cloned per slug" approach the pre-redesign
/// `state::seeded_tree`/`seeded_commits`/`seeded_branches` used, just backed
/// by real commits now instead of a cloned `Vec`.
async fn seed_one_repository(pool: &SqlitePool, base_dir: &Path, repo_slug: &str, now: i64) {
    const DAY: i64 = 86_400;
    const HOUR: i64 = 3_600;

    // ---- C1 (main): initial import ----
    stage_file(pool, base_dir, repo_slug, "README.md", FileChangeType::Added,
        b"# Demo Repository\n\nSeeded by lorehub-api's content-addressed VCS with real, hashed placeholder data.\n").await;
    stage_file(pool, base_dir, repo_slug, "Source/Game.cpp", FileChangeType::Added,
        b"#include \"Game.h\"\n\nvoid Game::Tick(float deltaSeconds)\n{\n    World.Update(deltaSeconds);\n    Renderer.Submit(World.GetDrawCalls());\n}\n").await;
    stage_file(
        pool,
        base_dir,
        repo_slug,
        "Source/Game.h",
        FileChangeType::Added,
        b"#pragma once\n\nclass Game\n{\npublic:\n    void Tick(float deltaSeconds);\n};\n",
    )
    .await;
    stage_file(
        pool,
        base_dir,
        repo_slug,
        "Assets/Characters/hero_rig.fbx",
        FileChangeType::Added,
        b"FBX_PLACEHOLDER v1 - hero_rig skeleton binding data\n",
    )
    .await;
    stage_file(
        pool,
        base_dir,
        repo_slug,
        "Assets/Characters/hero_diffuse.png",
        FileChangeType::Added,
        b"PNG_PLACEHOLDER - hero diffuse texture data\n",
    )
    .await;
    stage_file(
        pool,
        base_dir,
        repo_slug,
        "Assets/Environments/hollow_keep_terrain.uasset",
        FileChangeType::Added,
        b"UASSET_PLACEHOLDER v1 - hollow keep terrain chunk data with 5-tier LOD\n",
    )
    .await;
    stage_file(
        pool,
        base_dir,
        repo_slug,
        "Assets/Audio/theme_main.wav",
        FileChangeType::Added,
        b"WAV_PLACEHOLDER v1 - main theme arpeggio mix\n",
    )
    .await;
    commit_seed(
        pool,
        repo_slug,
        "main",
        AIKO,
        "Initial import of project assets",
        None,
        now - 8 * DAY,
    )
    .await;

    // ---- C2 (main): document sparse checkout ----
    stage_file(pool, base_dir, repo_slug, "README.md", FileChangeType::Modified,
        b"# Demo Repository\n\nSeeded by lorehub-api's content-addressed VCS with real, hashed placeholder data.\n\nRun `lore sync Assets/Environments` for a sparse checkout.\n").await;
    commit_seed(
        pool,
        repo_slug,
        "main",
        AIKO,
        "Document sparse checkout workflow",
        None,
        now - 7 * DAY,
    )
    .await;

    // ---- C3 (main): remaster main theme mix ----
    stage_file(
        pool,
        base_dir,
        repo_slug,
        "Assets/Audio/theme_main.wav",
        FileChangeType::Modified,
        b"WAV_PLACEHOLDER v2 - main theme arpeggio mix, remastered\n",
    )
    .await;
    commit_seed(
        pool,
        repo_slug,
        "main",
        PRIYA,
        "Record updated main theme mix",
        None,
        now - 5 * DAY,
    )
    .await;

    // ---- C4 (main): drop deprecated terrain LOD tier ----
    stage_file(
        pool,
        base_dir,
        repo_slug,
        "Assets/Environments/hollow_keep_terrain.uasset",
        FileChangeType::Modified,
        b"UASSET_PLACEHOLDER v2 - hollow keep terrain chunk data with 3-tier LOD\n",
    )
    .await;
    commit_seed(
        pool,
        repo_slug,
        "main",
        MARCO,
        "Remove deprecated terrain LOD tier",
        None,
        now - 2 * DAY,
    )
    .await;

    // Both feature branches fork from `main` here, at C4 — matching the old
    // demo data's DAG shape (both diverged from the "Remove deprecated
    // terrain LOD tier" commit). See the module doc comment for why the old
    // merge-back commit is dropped rather than reconstructed.
    vcs_store::create_branch_from(pool, repo_slug, "feature/dusk-skybox", "main", false).await;
    vcs_store::create_branch_from(pool, repo_slug, "feature/hero-rig-retarget", "main", false)
        .await;

    // ---- C5 (main): continues past the fork point on its own ----
    stage_file(pool, base_dir, repo_slug, "Source/Game.cpp", FileChangeType::Modified,
        b"#include \"Game.h\"\n\nvoid Game::Tick(float deltaSeconds)\n{\n    World.Update(deltaSeconds * TimeScale);\n    World.FlushPendingLocks();\n    Renderer.Submit(World.GetDrawCalls());\n}\n").await;
    stage_file(pool, base_dir, repo_slug, "Source/Game.h", FileChangeType::Modified,
        b"#pragma once\n\nclass Game\n{\npublic:\n    void Tick(float deltaSeconds);\n\nprivate:\n    float TimeScale = 1.0f;\n};\n").await;
    commit_seed(
        pool,
        repo_slug,
        "main",
        PRIYA,
        "Fix world tick order for late-joining actors",
        Some("Renderer.Submit was picking up stale draw calls when actors joined mid-tick."),
        now - 6 * HOUR,
    )
    .await;

    // ---- C6 (feature/dusk-skybox): diverges from main, never merges back ----
    stage_file(
        pool,
        base_dir,
        repo_slug,
        "Assets/Environments/skybox_dusk.png",
        FileChangeType::Added,
        b"PNG_PLACEHOLDER - dusk skybox gradient texture data\n",
    )
    .await;
    commit_seed(
        pool,
        repo_slug,
        "feature/dusk-skybox",
        MARCO,
        "Add dusk skybox for Hollow Keep exteriors",
        None,
        now - DAY,
    )
    .await;

    // ---- C7 (feature/hero-rig-retarget): also diverges from main ----
    stage_file(
        pool,
        base_dir,
        repo_slug,
        "Assets/Characters/hero_rig.fbx",
        FileChangeType::Modified,
        b"FBX_PLACEHOLDER v2 - hero_rig retargeted to updated skeleton\n",
    )
    .await;
    commit_seed(
        pool,
        repo_slug,
        "feature/hero-rig-retarget",
        AIKO,
        "Retarget hero rig to updated skeleton",
        None,
        now - 2 * HOUR,
    )
    .await;
}

/// Entry point — called once at startup (see `main.rs`), after
/// `db::migrate_or_seed_repositories` has ensured the `repositories` table
/// (and each repo's initial `main` branch row) exists. Builds real demo
/// history for every `is_seeded` repository that doesn't have any commits
/// yet; a no-op for everything else (including every subsequent restart of
/// an already-running instance).
///
/// `base_dir` is the filesystem root blobs are written under (see
/// `blob_store::blob_path`) — callers pass `AppContext::blob_base_dir` (see
/// its doc comment for why this is a parameter rather than this function
/// reading `blob_store::BASE_DIR` directly: test isolation).
pub async fn seed_demo_history(pool: &SqlitePool, base_dir: &Path) {
    let now = auth::current_unix_time();

    for slug in repo_store::seeded_slugs(pool).await {
        if vcs_store::commit_count(pool, &slug).await > 0 {
            continue;
        }
        seed_one_repository(pool, base_dir, &slug, now).await;
    }
}
