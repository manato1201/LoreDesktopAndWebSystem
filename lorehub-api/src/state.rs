use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::auth;
use crate::image_assets;
use crate::models::*;

pub type SharedState = Arc<AppContext>;

/// Wraps the in-memory `AppState` (still the hot path every handler reads
/// and writes) with the SQLite pool used to persist it. `read`/`write`
/// delegate to the inner lock so existing `ctx.read().await` /
/// `ctx.write().await` call sites don't need to know persistence exists;
/// handlers that mutate state additionally call `crate::db::save_blob`
/// with `ctx.db` to flush the changed piece to disk.
pub struct AppContext {
    state: RwLock<AppState>,
    pub db: SqlitePool,
}

impl AppContext {
    pub fn new(state: AppState, db: SqlitePool) -> Self {
        Self {
            state: RwLock::new(state),
            db,
        }
    }

    pub async fn read(&self) -> RwLockReadGuard<'_, AppState> {
        self.state.read().await
    }

    pub async fn write(&self) -> RwLockWriteGuard<'_, AppState> {
        self.state.write().await
    }
}

pub struct AppState {
    pub repositories: Vec<Repository>,
    /// Slugs of repositories that have seeded tree/commit/branch demo data.
    /// `tree`/`commits`/`branches` below are keyed by repository slug; every
    /// slug in this set gets its own clone of the shared demo dataset at
    /// seed time. Repos created later via `POST /api/repositories` are
    /// intentionally left out of this set (and out of the three maps) so
    /// their Code/Commits tabs report an empty (but valid, non-404)
    /// tree/history instead of inheriting the demo data.
    pub seeded_repo_slugs: HashSet<String>,
    /// Per-repository file tree, keyed by repository slug.
    pub tree: HashMap<String, Vec<TreeNode>>,
    pub file_contents: HashMap<String, String>,
    pub image_content: HashMap<String, String>,
    pub image_content_before: HashMap<String, String>,
    pub audio_content: HashMap<String, Vec<u8>>,
    /// Per-repository commit history (oldest first within each vec), keyed
    /// by repository slug.
    pub commits: HashMap<String, Vec<Commit>>,
    /// Per-repository branch list, keyed by repository slug.
    pub branches: HashMap<String, Vec<Branch>>,
    /// Repository slug -> currently checked-out branch name. Absent means
    /// "main" (the default branch in every seeded repo).
    pub current_branch: HashMap<String, String>,
    /// Repository slug -> staged-but-not-committed file changes.
    pub pending_changes: HashMap<String, Vec<FileChange>>,
    pub pull_requests: Vec<PullRequest>,
    pub access_entries: HashMap<String, Vec<AccessEntry>>,
    pub org_members: Vec<OrgMember>,
    pub storage: StorageUsage,
    pub audit_log: Vec<AuditLogEntry>,
    /// email -> argon2 password hash. Every demo account shares the same
    /// password ("lorehub") for convenience — see README/login page copy.
    pub credentials: HashMap<String, String>,
    /// session token -> email.
    pub sessions: HashMap<String, String>,
}

impl AppState {
    pub fn record_audit(&mut self, actor: &str, action: &str, target: &str) {
        self.audit_log.insert(
            0,
            AuditLogEntry {
                id: format!("a{}", self.audit_log.len() + 1),
                actor: actor.to_string(),
                action: action.to_string(),
                target: target.to_string(),
                timestamp: "just now".to_string(),
            },
        );
    }

    /// Recursively sets `lockedBy` on the node at `path` within `slug`'s
    /// tree. Returns `true` if a matching node was found.
    pub fn set_lock(&mut self, slug: &str, path: &str, locked_by: Option<String>) -> bool {
        fn walk(nodes: &mut [TreeNode], path: &str, locked_by: &Option<String>) -> bool {
            for node in nodes.iter_mut() {
                if node.path() == path {
                    match node {
                        TreeNode::Text { locked_by: lb, .. }
                        | TreeNode::Image { locked_by: lb, .. }
                        | TreeNode::Model3d { locked_by: lb, .. }
                        | TreeNode::Audio { locked_by: lb, .. }
                        | TreeNode::Binary { locked_by: lb, .. } => {
                            *lb = locked_by.clone();
                            return true;
                        }
                        TreeNode::Directory { .. } => return false,
                    }
                }
                if let TreeNode::Directory { children, .. } = node
                    && walk(children, path, locked_by)
                {
                    return true;
                }
            }
            false
        }

        let Some(tree) = self.tree.get_mut(slug) else {
            return false;
        };
        walk(tree, path, &locked_by)
    }
}

/// The single demo file tree shared as a starting point by every seeded
/// repository. Factored out of `seed()` so `db.rs` can rebuild the same
/// per-slug map as a backward-compat fallback when an old on-disk save has
/// the pre-refactor bare-`Vec` shape for the `"tree"` blob.
pub fn demo_tree() -> Vec<TreeNode> {
    vec![
        TreeNode::Directory {
            path: "Assets".into(),
            name: "Assets".into(),
            children: vec![
                TreeNode::Directory {
                    path: "Assets/Characters".into(),
                    name: "Characters".into(),
                    children: vec![
                        TreeNode::Model3d {
                            path: "Assets/Characters/hero_rig.fbx".into(),
                            name: "hero_rig.fbx".into(),
                            size_label: "42.1 MB".into(),
                            updated_at: "2h ago".into(),
                            locked_by: Some("Aiko Tanaka".into()),
                        },
                        TreeNode::Image {
                            path: "Assets/Characters/hero_diffuse.png".into(),
                            name: "hero_diffuse.png".into(),
                            size_label: "18.4 MB".into(),
                            updated_at: "1d ago".into(),
                            locked_by: None,
                        },
                    ],
                },
                TreeNode::Directory {
                    path: "Assets/Environments".into(),
                    name: "Environments".into(),
                    children: vec![
                        TreeNode::Binary {
                            path: "Assets/Environments/hollow_keep_terrain.uasset".into(),
                            name: "hollow_keep_terrain.uasset".into(),
                            size_label: "1.2 GB".into(),
                            updated_at: "6h ago".into(),
                            locked_by: None,
                        },
                        TreeNode::Image {
                            path: "Assets/Environments/skybox_dusk.png".into(),
                            name: "skybox_dusk.png".into(),
                            size_label: "64.0 MB".into(),
                            updated_at: "3d ago".into(),
                            locked_by: None,
                        },
                    ],
                },
                TreeNode::Directory {
                    path: "Assets/Audio".into(),
                    name: "Audio".into(),
                    children: vec![TreeNode::Audio {
                        path: "Assets/Audio/theme_main.wav".into(),
                        name: "theme_main.wav".into(),
                        size_label: "96.3 MB".into(),
                        updated_at: "5d ago".into(),
                        locked_by: Some("Marco Silva".into()),
                    }],
                },
            ],
        },
        TreeNode::Directory {
            path: "Source".into(),
            name: "Source".into(),
            children: vec![
                TreeNode::Text {
                    path: "Source/Game.cpp".into(),
                    name: "Game.cpp".into(),
                    size_label: "12.8 KB".into(),
                    updated_at: "2h ago".into(),
                    locked_by: None,
                },
                TreeNode::Text {
                    path: "Source/Game.h".into(),
                    name: "Game.h".into(),
                    size_label: "3.1 KB".into(),
                    updated_at: "2h ago".into(),
                    locked_by: None,
                },
            ],
        },
        TreeNode::Text {
            path: "README.md".into(),
            name: "README.md".into(),
            size_label: "2.4 KB".into(),
            updated_at: "1w ago".into(),
            locked_by: None,
        },
    ]
}

/// The single demo commit history shared as a starting point by every
/// seeded repository. See [`demo_tree`] for why this is factored out.
///
/// Chronological order (oldest first): main has a linear base, then
/// feature/dusk-skybox branches off and merges back in, while
/// feature/hero-rig-retarget branches off and is still open.
pub fn demo_commits() -> Vec<Commit> {
    vec![
        Commit {
            hash: "0f4a7b0c3d6e9f2a5b8c1d4e7f0a3b6c9d2e5f8a".into(),
            short_hash: "0f4a7b0".into(),
            message: "Document sparse checkout workflow".into(),
            description: None,
            author: "Aiko Tanaka".into(),
            author_initials: "AT".into(),
            timestamp: "1w ago".into(),
            changed_files: vec![FileChange {
                path: "README.md".into(),
                change_type: FileChangeType::Modified,
                size_delta_label: "+0.6 KB".into(),
            }],
            branch: "main".into(),
            parents: vec![],
        },
        Commit {
            hash: "9b1e4f7a0c3d6e9f2a5b8c1d4e7f0a3b6c9d2e5f".into(),
            short_hash: "9b1e4f7".into(),
            message: "Record updated main theme mix".into(),
            description: None,
            author: "Priya Desai".into(),
            author_initials: "PD".into(),
            timestamp: "5d ago".into(),
            changed_files: vec![FileChange {
                path: "Assets/Audio/theme_main.wav".into(),
                change_type: FileChangeType::Modified,
                size_delta_label: "+3.1 MB".into(),
            }],
            branch: "main".into(),
            parents: vec!["0f4a7b0c3d6e9f2a5b8c1d4e7f0a3b6c9d2e5f8a".into()],
        },
        Commit {
            hash: "3c6d9e2f5a8b1c4d7e0f3a6b9c2d5e8f1a4b7c0d".into(),
            short_hash: "3c6d9e2".into(),
            message: "Remove deprecated terrain LOD tier".into(),
            description: None,
            author: "Marco Silva".into(),
            author_initials: "MS".into(),
            timestamp: "2d ago".into(),
            changed_files: vec![FileChange {
                path: "Assets/Environments/hollow_keep_terrain.uasset".into(),
                change_type: FileChangeType::Modified,
                size_delta_label: "-320.0 MB".into(),
            }],
            branch: "main".into(),
            parents: vec!["9b1e4f7a0c3d6e9f2a5b8c1d4e7f0a3b6c9d2e5f".into()],
        },
        Commit {
            hash: "7d2b9c1e4f68a0b3d5c7e9f1a2b4c6d8e0f1a2b3".into(),
            short_hash: "7d2b9c1".into(),
            message: "Add dusk skybox for Hollow Keep exteriors".into(),
            description: None,
            author: "Marco Silva".into(),
            author_initials: "MS".into(),
            timestamp: "1d ago".into(),
            changed_files: vec![FileChange {
                path: "Assets/Environments/skybox_dusk.png".into(),
                change_type: FileChangeType::Added,
                size_delta_label: "+64.0 MB".into(),
            }],
            branch: "feature/dusk-skybox".into(),
            parents: vec!["3c6d9e2f5a8b1c4d7e0f3a6b9c2d5e8f1a4b7c0d".into()],
        },
        Commit {
            hash: "f2a9c81b5e3d6a9c2f5b8e1d4a7c0f3b6e9a2d5f".into(),
            short_hash: "f2a9c81".into(),
            message: "Merge branch 'feature/dusk-skybox' into main".into(),
            description: None,
            author: "Marco Silva".into(),
            author_initials: "MS".into(),
            timestamp: "20h ago".into(),
            changed_files: vec![],
            branch: "main".into(),
            parents: vec![
                "3c6d9e2f5a8b1c4d7e0f3a6b9c2d5e8f1a4b7c0d".into(),
                "7d2b9c1e4f68a0b3d5c7e9f1a2b4c6d8e0f1a2b3".into(),
            ],
        },
        Commit {
            hash: "e5f8a1b4c7d0e3f6a9b2c5d8e1f4a7b0c3d6e9f2".into(),
            short_hash: "e5f8a1b".into(),
            message: "Fix world tick order for late-joining actors".into(),
            description: Some(
                "Renderer.Submit was picking up stale draw calls when actors joined mid-tick."
                    .into(),
            ),
            author: "Priya Desai".into(),
            author_initials: "PD".into(),
            timestamp: "6h ago".into(),
            changed_files: vec![
                FileChange {
                    path: "Source/Game.cpp".into(),
                    change_type: FileChangeType::Modified,
                    size_delta_label: "+0.4 KB".into(),
                },
                FileChange {
                    path: "Source/Game.h".into(),
                    change_type: FileChangeType::Modified,
                    size_delta_label: "±0 B".into(),
                },
            ],
            branch: "main".into(),
            parents: vec!["f2a9c81b5e3d6a9c2f5b8e1d4a7c0f3b6e9a2d5f".into()],
        },
        Commit {
            hash: "a1c4e7f92b3d5e6081247fa9c0d8b3e6f2a1c47".into(),
            short_hash: "a1c4e7f".into(),
            message: "Retarget hero rig to updated skeleton".into(),
            description: None,
            author: "Aiko Tanaka".into(),
            author_initials: "AT".into(),
            timestamp: "2h ago".into(),
            changed_files: vec![FileChange {
                path: "Assets/Characters/hero_rig.fbx".into(),
                change_type: FileChangeType::Modified,
                size_delta_label: "+1.2 MB".into(),
            }],
            branch: "feature/hero-rig-retarget".into(),
            parents: vec!["3c6d9e2f5a8b1c4d7e0f3a6b9c2d5e8f1a4b7c0d".into()],
        },
    ]
}

/// The single demo branch list shared as a starting point by every seeded
/// repository. See [`demo_tree`] for why this is factored out.
pub fn demo_branches() -> Vec<Branch> {
    vec![
        Branch {
            name: "main".into(),
            head: "e5f8a1b4c7d0e3f6a9b2c5d8e1f4a7b0c3d6e9f2".into(),
            is_default: true,
        },
        Branch {
            name: "feature/dusk-skybox".into(),
            head: "7d2b9c1e4f68a0b3d5c7e9f1a2b4c6d8e0f1a2b3".into(),
            is_default: false,
        },
        Branch {
            name: "feature/hero-rig-retarget".into(),
            head: "a1c4e7f92b3d5e6081247fa9c0d8b3e6f2a1c47".into(),
            is_default: false,
        },
    ]
}

/// Builds the per-slug tree map used both at first-run seed time and as the
/// `db.rs` fallback when an old on-disk save can't be deserialized into the
/// new map shape.
pub fn seeded_tree(slugs: &HashSet<String>) -> HashMap<String, Vec<TreeNode>> {
    slugs.iter().map(|s| (s.clone(), demo_tree())).collect()
}

/// See [`seeded_tree`].
pub fn seeded_commits(slugs: &HashSet<String>) -> HashMap<String, Vec<Commit>> {
    slugs.iter().map(|s| (s.clone(), demo_commits())).collect()
}

/// See [`seeded_tree`].
pub fn seeded_branches(slugs: &HashSet<String>) -> HashMap<String, Vec<Branch>> {
    slugs.iter().map(|s| (s.clone(), demo_branches())).collect()
}

pub fn seed() -> AppState {
    let repositories = vec![
        Repository {
            slug: "starforge-vfx".into(),
            name: "starforge-vfx".into(),
            organization: "Nebula Studios".into(),
            description: "Particle FX library and Niagara modules for the Starforge campaign."
                .into(),
            updated_at: "2h ago".into(),
            size_label: "184 GB".into(),
            locked_file_count: 3,
            visibility: Visibility::Private,
        },
        Repository {
            slug: "hollow-keep-env".into(),
            name: "hollow-keep-env".into(),
            organization: "Nebula Studios".into(),
            description: "Environment art, terrain chunks, and lighting scenarios for Hollow Keep."
                .into(),
            updated_at: "6h ago".into(),
            size_label: "512 GB".into(),
            locked_file_count: 0,
            visibility: Visibility::Private,
        },
        Repository {
            slug: "character-rigs".into(),
            name: "character-rigs".into(),
            organization: "Nebula Studios".into(),
            description: "Shared character skeletons, rigs, and animation retarget presets.".into(),
            updated_at: "1d ago".into(),
            size_label: "76 GB".into(),
            locked_file_count: 1,
            visibility: Visibility::Internal,
        },
        Repository {
            slug: "audio-master".into(),
            name: "audio-master".into(),
            organization: "Nebula Studios".into(),
            description: "Master audio sessions, foley captures, and mix stems.".into(),
            updated_at: "2d ago".into(),
            size_label: "212 GB".into(),
            locked_file_count: 0,
            visibility: Visibility::Private,
        },
        Repository {
            slug: "cinematics-s2".into(),
            name: "cinematics-s2".into(),
            organization: "Nebula Studios".into(),
            description: "Season 2 cinematic sequences, previs, and camera capture data.".into(),
            updated_at: "3d ago".into(),
            size_label: "1.1 TB".into(),
            locked_file_count: 5,
            visibility: Visibility::Private,
        },
        Repository {
            slug: "shared-materials".into(),
            name: "shared-materials".into(),
            organization: "Nebula Studios".into(),
            description: "Cross-project material library, substance graphs, and texture sets."
                .into(),
            updated_at: "5d ago".into(),
            size_label: "98 GB".into(),
            locked_file_count: 0,
            visibility: Visibility::Public,
        },
    ];

    let seeded_repo_slugs: HashSet<String> = repositories.iter().map(|r| r.slug.clone()).collect();

    let tree = seeded_tree(&seeded_repo_slugs);
    let commits = seeded_commits(&seeded_repo_slugs);
    let branches = seeded_branches(&seeded_repo_slugs);

    let pull_requests = vec![
        PullRequest {
            id: "42".into(),
            title: "Retarget hero rig to updated skeleton".into(),
            description: "Reworks the hero rig's bone hierarchy to match the new mocap skeleton. Also swaps the diffuse texture for the higher-resolution pass.".into(),
            repo_slug: "hollow-keep-env".into(),
            repo_name: "hollow-keep-env".into(),
            status: PrStatus::Open,
            author: "Aiko Tanaka".into(),
            author_initials: "AT".into(),
            created_at: "3h ago".into(),
            updated_at: "1h ago".into(),
            changed_files: vec![
                PrDiffFile::Text {
                    path: "Source/Game.cpp".into(),
                    change_type: FileChangeType::Modified,
                    lines: vec![
                        DiffLine { kind: DiffLineType::Context, text: "void Game::Tick(float deltaSeconds)".into() },
                        DiffLine { kind: DiffLineType::Context, text: "{".into() },
                        DiffLine { kind: DiffLineType::Remove, text: "    World.Update(deltaSeconds);".into() },
                        DiffLine { kind: DiffLineType::Add, text: "    World.Update(deltaSeconds * TimeScale);".into() },
                        DiffLine { kind: DiffLineType::Add, text: "    World.FlushPendingLocks();".into() },
                        DiffLine { kind: DiffLineType::Context, text: "    Renderer.Submit(World.GetDrawCalls());".into() },
                        DiffLine { kind: DiffLineType::Context, text: "}".into() },
                    ],
                },
                PrDiffFile::Model3d {
                    path: "Assets/Characters/hero_rig.fbx".into(),
                    change_type: FileChangeType::Modified,
                },
                PrDiffFile::Image {
                    path: "Assets/Characters/hero_diffuse.png".into(),
                    change_type: FileChangeType::Modified,
                },
            ],
            comments: vec![PrComment {
                id: "c1".into(),
                author: "Marco Silva".into(),
                author_initials: "MS".into(),
                timestamp: "50m ago".into(),
                body: "Rig deformation on the left shoulder looks correct now. Diffuse pass is a nice upgrade.".into(),
            }],
        },
        PullRequest {
            id: "39".into(),
            title: "Add dusk skybox for Hollow Keep exteriors".into(),
            description: "New skybox pass for the exterior courtyard scenes. Replaces the placeholder gradient sky.".into(),
            repo_slug: "hollow-keep-env".into(),
            repo_name: "hollow-keep-env".into(),
            status: PrStatus::Merged,
            author: "Marco Silva".into(),
            author_initials: "MS".into(),
            created_at: "2d ago".into(),
            updated_at: "1d ago".into(),
            changed_files: vec![PrDiffFile::Image {
                path: "Assets/Environments/skybox_dusk.png".into(),
                change_type: FileChangeType::Added,
            }],
            comments: vec![PrComment {
                id: "c2".into(),
                author: "Priya Desai".into(),
                author_initials: "PD".into(),
                timestamp: "1d ago".into(),
                body: "Color grading matches the reference board. Merging.".into(),
            }],
        },
        PullRequest {
            id: "35".into(),
            title: "Revert oversized terrain LOD experiment".into(),
            description: "The experimental LOD tier regressed streaming performance on the courtyard level. Reverting until the chunking strategy is revisited.".into(),
            repo_slug: "hollow-keep-env".into(),
            repo_name: "hollow-keep-env".into(),
            status: PrStatus::Closed,
            author: "Priya Desai".into(),
            author_initials: "PD".into(),
            created_at: "5d ago".into(),
            updated_at: "4d ago".into(),
            changed_files: vec![PrDiffFile::Text {
                path: "README.md".into(),
                change_type: FileChangeType::Modified,
                lines: vec![
                    DiffLine { kind: DiffLineType::Context, text: "## Terrain LOD".into() },
                    DiffLine { kind: DiffLineType::Remove, text: "Experimental 5-tier LOD is enabled by default.".into() },
                    DiffLine { kind: DiffLineType::Add, text: "LOD stays at the standard 3-tier setup for now.".into() },
                ],
            }],
            comments: vec![],
        },
    ];

    let mut access_entries = HashMap::new();
    access_entries.insert(
        "Assets".to_string(),
        vec![
            AccessEntry {
                principal: "Environment Artists".into(),
                principal_type: PrincipalType::Team,
                permissions: vec![PermissionLevel::Read, PermissionLevel::Write],
            },
            AccessEntry {
                principal: "Character Artists".into(),
                principal_type: PrincipalType::Team,
                permissions: vec![PermissionLevel::Read, PermissionLevel::Write],
            },
            AccessEntry {
                principal: "QA Contractors".into(),
                principal_type: PrincipalType::Team,
                permissions: vec![PermissionLevel::Read],
            },
        ],
    );
    access_entries.insert(
        "Assets/Characters".to_string(),
        vec![
            AccessEntry {
                principal: "Character Artists".into(),
                principal_type: PrincipalType::Team,
                permissions: vec![
                    PermissionLevel::Read,
                    PermissionLevel::Write,
                    PermissionLevel::Lock,
                ],
            },
            AccessEntry {
                principal: "Aiko Tanaka".into(),
                principal_type: PrincipalType::User,
                permissions: vec![
                    PermissionLevel::Read,
                    PermissionLevel::Write,
                    PermissionLevel::Lock,
                ],
            },
        ],
    );
    access_entries.insert(
        "Assets/Environments".to_string(),
        vec![AccessEntry {
            principal: "Environment Artists".into(),
            principal_type: PrincipalType::Team,
            permissions: vec![
                PermissionLevel::Read,
                PermissionLevel::Write,
                PermissionLevel::Lock,
            ],
        }],
    );
    access_entries.insert(
        "Source".to_string(),
        vec![
            AccessEntry {
                principal: "Engineering".into(),
                principal_type: PrincipalType::Team,
                permissions: vec![
                    PermissionLevel::Read,
                    PermissionLevel::Write,
                    PermissionLevel::Lock,
                ],
            },
            AccessEntry {
                principal: "QA Contractors".into(),
                principal_type: PrincipalType::Team,
                permissions: vec![PermissionLevel::Read],
            },
        ],
    );

    let org_members = vec![
        OrgMember {
            name: "Aiko Tanaka".into(),
            initials: "AT".into(),
            email: "aiko.tanaka@nebula.studio".into(),
            role: MemberRole::Owner,
            joined_at: "Jan 2023".into(),
        },
        OrgMember {
            name: "Marco Silva".into(),
            initials: "MS".into(),
            email: "marco.silva@nebula.studio".into(),
            role: MemberRole::Admin,
            joined_at: "Mar 2023".into(),
        },
        OrgMember {
            name: "Priya Desai".into(),
            initials: "PD".into(),
            email: "priya.desai@nebula.studio".into(),
            role: MemberRole::Member,
            joined_at: "Aug 2023".into(),
        },
        OrgMember {
            name: "Diego Fernandez".into(),
            initials: "DF".into(),
            email: "diego.fernandez@nebula.studio".into(),
            role: MemberRole::Member,
            joined_at: "Nov 2024".into(),
        },
    ];

    let storage = StorageUsage {
        used_label: "2.18 TB".into(),
        total_label: "5 TB".into(),
        used_percent: 44,
    };

    let audit_log = vec![
        AuditLogEntry {
            id: "a1".into(),
            actor: "Aiko Tanaka".into(),
            action: "locked".into(),
            target: "Assets/Characters/hero_rig.fbx".into(),
            timestamp: "2h ago".into(),
        },
        AuditLogEntry {
            id: "a2".into(),
            actor: "Marco Silva".into(),
            action: "merged pull request #39 into".into(),
            target: "hollow-keep-env".into(),
            timestamp: "1d ago".into(),
        },
        AuditLogEntry {
            id: "a3".into(),
            actor: "Priya Desai".into(),
            action: "updated permissions on".into(),
            target: "Source".into(),
            timestamp: "3d ago".into(),
        },
        AuditLogEntry {
            id: "a4".into(),
            actor: "Aiko Tanaka".into(),
            action: "invited".into(),
            target: "diego.fernandez@nebula.studio".into(),
            timestamp: "8mo ago".into(),
        },
    ];

    let mut file_contents = HashMap::new();
    file_contents.insert(
        "Source/Game.cpp".to_string(),
        "#include \"Game.h\"\n\nvoid Game::Tick(float deltaSeconds)\n{\n    World.Update(deltaSeconds);\n    Renderer.Submit(World.GetDrawCalls());\n}\n".to_string(),
    );
    file_contents.insert(
        "Source/Game.h".to_string(),
        "#pragma once\n\nclass Game\n{\npublic:\n    void Tick(float deltaSeconds);\n};\n"
            .to_string(),
    );
    file_contents.insert(
        "README.md".to_string(),
        "# Hollow Keep\n\nEnvironment art, terrain chunks, and lighting scenarios.\n\nRun `lore sync Assets/Environments` for a sparse checkout.\n".to_string(),
    );

    let mut image_content = HashMap::new();
    image_content.insert(
        "Assets/Characters/hero_diffuse.png".to_string(),
        image_assets::HERO_DIFFUSE_AFTER.to_string(),
    );
    image_content.insert(
        "Assets/Environments/skybox_dusk.png".to_string(),
        image_assets::SKYBOX_DUSK_AFTER.to_string(),
    );

    let mut image_content_before = HashMap::new();
    image_content_before.insert(
        "Assets/Characters/hero_diffuse.png".to_string(),
        image_assets::HERO_DIFFUSE_BEFORE.to_string(),
    );

    let mut audio_content = HashMap::new();
    audio_content.insert(
        "Assets/Audio/theme_main.wav".to_string(),
        generate_theme_wav(),
    );

    let demo_password_hash = auth::hash_password("lorehub");
    let credentials = org_members
        .iter()
        .map(|m| (m.email.clone(), demo_password_hash.clone()))
        .collect();

    AppState {
        repositories,
        seeded_repo_slugs,
        tree,
        file_contents,
        image_content,
        image_content_before,
        audio_content,
        commits,
        branches,
        current_branch: HashMap::new(),
        pending_changes: HashMap::new(),
        pull_requests,
        access_entries,
        org_members,
        storage,
        audit_log,
        credentials,
        sessions: HashMap::new(),
    }
}

/// Synthesizes a short mono 16-bit PCM WAV clip (a decaying sine-wave
/// arpeggio) since there is no real audio asset to stream. Pure std, no
/// audio crate needed.
fn generate_theme_wav() -> Vec<u8> {
    const SAMPLE_RATE: u32 = 22_050;
    const NOTE_SECONDS: f32 = 0.5;
    const NOTES_HZ: [f32; 6] = [261.63, 329.63, 392.00, 523.25, 392.00, 329.63];

    let note_samples = (SAMPLE_RATE as f32 * NOTE_SECONDS) as usize;
    let mut samples: Vec<i16> = Vec::with_capacity(note_samples * NOTES_HZ.len());

    for &freq in &NOTES_HZ {
        for n in 0..note_samples {
            let t = n as f32 / SAMPLE_RATE as f32;
            let envelope = (-t * 3.0).exp();
            let value = (t * freq * std::f32::consts::TAU).sin() * envelope * 0.3;
            samples.push((value * i16::MAX as f32) as i16);
        }
    }

    let data_len = (samples.len() * 2) as u32;
    let byte_rate = SAMPLE_RATE * 2;

    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes()); // block align
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}
