use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::auth;
use crate::image_assets;
use crate::models::*;

/// A single session or refresh-token record: which user it belongs to, and
/// when it stops being valid (unix seconds). Shared between `AppState::
/// sessions` (access tokens) and `AppState::refresh_tokens` (refresh
/// tokens) — the two maps differ only in TTL and which cookie feeds them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub email: String,
    pub expires_at: i64,
}

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
    /// Resolved SMTP relay config, or `None` for the local-dev fallback (see
    /// `email::send_email`). Startup-time config, not persisted data — that's
    /// why it lives here rather than in `AppState`.
    pub email_config: Option<crate::email::SmtpConfig>,
    /// Filesystem root every blob path resolves against (see
    /// `blob_store::blob_path`) — `handlers::upload_file`/
    /// `resolve_uploaded_content`/`delete_repository` and `vcs_seed::
    /// seed_demo_history` all take this instead of reading `blob_store::
    /// BASE_DIR` directly. In production (`main.rs`) this is always
    /// `blob_store::BASE_DIR` (`"."`, i.e. process CWD — see its doc
    /// comment). Tests (`tests/mod.rs`) instead give every test its own
    /// private temp directory here, the same way `db::connect(":memory:")`
    /// gives every test its own private SQL database — without this, every
    /// parallel test's `vcs_seed` call would write real blob files into the
    /// *same* shared `./blobs/...` directory on disk, racing any other
    /// concurrently-running test that deletes a repository (`DELETE
    /// /api/repositories/{slug}` removes that slug's whole blob directory —
    /// see `handlers::delete_repository`), which produced real, observed
    /// `NotFound` panics under `cargo test`'s default parallel execution
    /// before this field existed.
    pub blob_base_dir: std::path::PathBuf,
}

impl AppContext {
    pub fn new(
        state: AppState,
        db: SqlitePool,
        email_config: Option<crate::email::SmtpConfig>,
        blob_base_dir: std::path::PathBuf,
    ) -> Self {
        Self {
            state: RwLock::new(state),
            db,
            email_config,
            blob_base_dir,
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
    /// Which repository slugs exist at all lives in the SQL `repositories`
    /// table (see `repo_store.rs`). As of Phase 4, the VCS data that used to
    /// live here as `tree`/`commits`/`branches`/`current_branch`/
    /// `pending_changes` `HashMap`s is gone too — that's all real SQL tables
    /// now (`branches`, `commits`, `commit_files`, `tree_entries`,
    /// `pending_changes`, `current_branch`, `file_locks`), read/written
    /// directly by `vcs_store.rs`/`lock_store.rs` with no `AppState` lock
    /// involved at all. `repo_store::seeded_slugs` is the SQL-backed
    /// replacement for what used to be `AppState.seeded_repo_slugs`.
    pub file_contents: HashMap<String, String>,
    pub image_content: HashMap<String, String>,
    pub image_content_before: HashMap<String, String>,
    pub audio_content: HashMap<String, Vec<u8>>,
    pub pull_requests: Vec<PullRequest>,
    pub access_entries: HashMap<String, Vec<AccessEntry>>,
    pub org_members: Vec<OrgMember>,
    pub storage: StorageUsage,
    pub audit_log: Vec<AuditLogEntry>,
    /// email -> argon2 password hash. Every demo account shares the same
    /// password ("lorehub") for convenience — see README/login page copy.
    pub credentials: HashMap<String, String>,
    /// access token -> session entry (email + expiry).
    pub sessions: HashMap<String, SessionEntry>,
    /// refresh token -> session entry (email + expiry). Separate map from
    /// `sessions` because refresh tokens live far longer (7 days vs. 30
    /// minutes) and are rotated (removed-and-reissued) on every use, so
    /// mixing them into one map would make expiry/rotation logic ambiguous
    /// about which kind of token a given key is.
    pub refresh_tokens: HashMap<String, SessionEntry>,
    /// Invite token -> pending invite. A member only exists in `org_members`
    /// once the invite is accepted (`POST /api/auth/accept-invite`); until
    /// then this is the only record of the pending signup.
    pub invites: HashMap<String, InviteEntry>,
    /// Password-reset token -> which account it's for and when it expires.
    /// Separate from `invites` because the TTL and accept-side validation
    /// differ (see `handlers::reset_password`).
    pub password_resets: HashMap<String, PasswordResetEntry>,
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
}

/// The 6 demo repositories every fresh install (or first-ever migration of
/// this VCS redesign) seeds. Factored out of `seed()` so `db::
/// migrate_or_seed_repositories` can insert the exact same dataset into the
/// SQL `repositories` table (see `repo_store.rs`) instead of `seed()`
/// building an in-memory `Vec` — `AppState` no longer carries a
/// `repositories` field at all (see its doc comment). Each seeded slug's
/// real VCS history (tree/commits/branches) is synthesized separately by
/// `vcs_seed::seed_demo_history`, not built here — `seed()` (below) no
/// longer needs this function's return value for anything beyond the
/// `AppState` fields that remain (there's no more per-slug `tree`/`commits`/
/// `branches` map to build from the slug list).
pub fn demo_repositories() -> Vec<Repository> {
    vec![
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
    ]
}

pub fn seed() -> AppState {
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
            // Owner bypasses path ACL checks entirely (see
            // authz::check_path_permission), so this isn't load-bearing for
            // authorization — included for data realism only.
            teams: vec!["Engineering".into()],
            joined_at: "Jan 2023".into(),
        },
        OrgMember {
            name: "Marco Silva".into(),
            initials: "MS".into(),
            email: "marco.silva@nebula.studio".into(),
            role: MemberRole::Admin,
            // Admin also bypasses path ACL checks entirely — team membership
            // here is data realism only, not a security dependency.
            teams: vec!["Environment Artists".into()],
            joined_at: "Mar 2023".into(),
        },
        OrgMember {
            name: "Priya Desai".into(),
            initials: "PD".into(),
            email: "priya.desai@nebula.studio".into(),
            role: MemberRole::Member,
            // Matches the "Character Artists" team principal already used by
            // the seeded `access_entries` on "Assets"/"Assets/Characters" —
            // this is what makes those demo ACL entries actually mean
            // something once path checks are enforced.
            teams: vec!["Character Artists".into()],
            joined_at: "Aug 2023".into(),
        },
        OrgMember {
            name: "Diego Fernandez".into(),
            initials: "DF".into(),
            email: "diego.fernandez@nebula.studio".into(),
            role: MemberRole::Member,
            // Matches the "QA Contractors" team principal on "Assets"/
            // "Source" (read-only) — same reasoning as Priya above.
            teams: vec!["QA Contractors".into()],
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
        file_contents,
        image_content,
        image_content_before,
        audio_content,
        pull_requests,
        access_entries,
        org_members,
        storage,
        audit_log,
        credentials,
        sessions: HashMap::new(),
        refresh_tokens: HashMap::new(),
        invites: HashMap::new(),
        password_resets: HashMap::new(),
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
