//! SQL-backed operations against the "real history" half of the Phase 1
//! schema (see `migrations/0001_vcs_schema.sql`): `branches`,
//! `current_branch`, `commits`, `commit_files`, `tree_entries`, and
//! `pending_changes`.
//!
//! This is the Phase 4 replacement for `AppState.tree`/`commits`/`branches`/
//! `current_branch`/`pending_changes` (all removed — see `state.rs`) and for
//! the fake-hash `create_commit` handler logic that used to live entirely in
//! `handlers.rs`. `file_locks` is deliberately NOT in this module — the
//! schema's own comment on that table ("Independent of commit history") is
//! why it gets its own `lock_store.rs` instead of being folded in here,
//! mirroring how `repo_store.rs` (repository identity) and
//! `blob_meta_store.rs` (blob metadata) are already split by concern rather
//! than merged into one giant "VCS" module. Everything else above — branches,
//! commits, the tree, and staging — is kept in one module because the single
//! most important operation in this whole redesign, [`create_commit`], has to
//! touch all of them together inside one transaction; splitting that
//! transaction's steps across module boundaries would only add indirection.
//!
//! No `AppState` lock is needed for anything in this module, same as
//! `repo_store.rs`/`blob_meta_store.rs` — every table here is read/written
//! directly against the `SqlitePool` (or an open `Transaction`, for
//! [`create_commit`]).

use std::collections::{BTreeMap, HashMap};

use sqlx::sqlite::SqliteRow;
use sqlx::{Executor, Row, Sqlite, SqlitePool};

use crate::content_hash;
use crate::models::{Branch, Commit, FileChange, FileChangeType, TreeNode};

fn change_type_to_str(t: FileChangeType) -> &'static str {
    match t {
        FileChangeType::Added => "added",
        FileChangeType::Modified => "modified",
        FileChangeType::Deleted => "deleted",
    }
}

/// Inverse of [`change_type_to_str`]. `pending_changes.change_type` and
/// `commit_files.change_type` both carry a `CHECK (change_type IN
/// ('added','modified','deleted'))` constraint (see the migration), so any
/// value that made it into either table is guaranteed to be one of these
/// three — an unrecognized value would mean the schema constraint itself was
/// bypassed.
fn change_type_from_str(s: &str) -> FileChangeType {
    match s {
        "added" => FileChangeType::Added,
        "modified" => FileChangeType::Modified,
        "deleted" => FileChangeType::Deleted,
        other => unreachable!("change_type CHECK constraint allowed {other:?}"),
    }
}

/// Infers `tree_entries.file_kind` (and, by extension, which `TreeNode`
/// variant a path renders as) from its extension — the same kind of
/// extension-sniffing `handlers::content_type_for_path`/`upload_kind_for_path`
/// already do for HTTP content-type/upload routing, generalized to the two
/// `TreeNode` kinds (`Model3d`/`Binary`) those two don't need to distinguish.
/// Anything unrecognized falls back to `"binary"` — the safest default for a
/// file type this app has no special handling for.
pub fn file_kind_for_path(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "txt" | "md" | "json" | "yaml" | "yml" | "toml" | "cpp" | "h" | "hpp" | "c" | "cs"
        | "rs" => "text",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => "image",
        "fbx" | "obj" | "gltf" | "glb" => "model3d",
        "wav" | "mp3" | "ogg" => "audio",
        // `.uasset` (Unreal Engine's generic serialized-asset container) can
        // hold anything — terrain data, materials, blueprints — not
        // specifically a 3D model, so it's classified as opaque `binary`
        // rather than `model3d`. Matches the pre-redesign seed data's own
        // classification of `hollow_keep_terrain.uasset`.
        "uasset" => "binary",
        _ => "binary",
    }
}

/// Formats a byte count the same way the pre-redesign seeded data did:
/// whole bytes under 1 KiB (`"512 B"`), one decimal place from KiB upward
/// (`"12.8 KB"`, `"42.1 MB"`, `"1.2 GB"`). `bytes` is treated as a magnitude
/// (see [`format_size_delta_label`] for the signed version built on top of
/// this).
pub fn format_size_label(bytes: i64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b < KB {
        format!("{bytes} B")
    } else if b < MB {
        format!("{:.1} KB", b / KB)
    } else if b < GB {
        format!("{:.1} MB", b / MB)
    } else {
        format!("{:.1} GB", b / GB)
    }
}

/// The real `sizeDeltaLabel` computation this phase replaces the hardcoded
/// `"—"` with — `prev`/`new` are `None` when the path didn't exist before
/// (added) or doesn't exist after (deleted) respectively, both treated as a
/// size of 0. Matches the old hand-seeded convention exactly: `"±0 B"` for no
/// change, otherwise a signed [`format_size_label`] of the magnitude (e.g.
/// `"+1.2 MB"`, `"-320.0 MB"`).
pub fn format_size_delta_label(prev: Option<i64>, new: Option<i64>) -> String {
    let delta = new.unwrap_or(0) - prev.unwrap_or(0);
    if delta == 0 {
        return "±0 B".to_string();
    }
    let sign = if delta > 0 { "+" } else { "-" };
    format!("{sign}{}", format_size_label(delta.abs()))
}

/// Formats a unix-seconds timestamp relative to `now` using the bucket
/// convention the pre-redesign seed data used by hand (`"just now"` /
/// `"Xm ago"` / `"Xh ago"` / `"Xd ago"` / `"Xw ago"`), replacing what used to
/// be a fixed literal (`"just now"`) or a hand-picked seed string with a
/// value actually computed from the stored `created_at` column.
pub fn format_relative_time(now: i64, then: i64) -> String {
    let diff = (now - then).max(0);
    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86_400 {
        format!("{}h ago", diff / 3600)
    } else if diff < 604_800 {
        format!("{}d ago", diff / 86_400)
    } else {
        format!("{}w ago", diff / 604_800)
    }
}

// ---------------------------------------------------------------------
// Branches / current_branch
// ---------------------------------------------------------------------

fn row_to_branch(row: &SqliteRow) -> Branch {
    Branch {
        name: row.get("name"),
        head: row
            .get::<Option<String>, _>("head_commit_hash")
            .unwrap_or_default(),
        is_default: row.get::<i64, _>("is_default") != 0,
    }
}

pub async fn list_branches(pool: &SqlitePool, repo_slug: &str) -> Vec<Branch> {
    sqlx::query(
        "SELECT name, head_commit_hash, is_default FROM branches
         WHERE repo_slug = ?1 ORDER BY is_default DESC, name",
    )
    .bind(repo_slug)
    .fetch_all(pool)
    .await
    .expect("query branches")
    .iter()
    .map(row_to_branch)
    .collect()
}

pub async fn branch_exists(pool: &SqlitePool, repo_slug: &str, name: &str) -> bool {
    sqlx::query("SELECT 1 FROM branches WHERE repo_slug = ?1 AND name = ?2")
        .bind(repo_slug)
        .bind(name)
        .fetch_optional(pool)
        .await
        .expect("query branch existence")
        .is_some()
}

/// `Some(head_commit_hash)` if the branch exists (`head_commit_hash` itself
/// may be `None` — an unborn branch with no commits yet); `None` if no such
/// branch row exists at all.
async fn branch_head_row<'e, E>(executor: E, repo_slug: &str, name: &str) -> Option<Option<String>>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query("SELECT head_commit_hash FROM branches WHERE repo_slug = ?1 AND name = ?2")
        .bind(repo_slug)
        .bind(name)
        .fetch_optional(executor)
        .await
        .expect("query branch head")
        .map(|row| row.get::<Option<String>, _>("head_commit_hash"))
}

/// Idempotently inserts the initial `main` branch row (`head_commit_hash =
/// NULL`, `is_default = 1`) for a repository that doesn't have one yet.
/// Called once per repository at creation time (both `handlers::
/// create_repository` for user-created repos and `db::
/// migrate_or_seed_repositories` for seeded/imported ones) — every
/// repository must have at least this row for `GET /tree`/checkout/commit to
/// have a current branch to resolve against. `ON CONFLICT DO NOTHING` makes
/// re-running this against an already-initialized repository a no-op.
pub async fn init_main_branch(pool: &SqlitePool, repo_slug: &str) {
    sqlx::query(
        "INSERT INTO branches (repo_slug, name, head_commit_hash, is_default)
         VALUES (?1, 'main', NULL, 1)
         ON CONFLICT(repo_slug, name) DO NOTHING",
    )
    .bind(repo_slug)
    .execute(pool)
    .await
    .expect("insert initial main branch");
}

/// Creates branch `name` in `repo_slug`, copying `head_commit_hash` from the
/// `source` branch as it stands at this moment (a snapshot, not a live
/// link — `source` advancing afterwards does not move `name`). If `source`
/// doesn't name an existing branch, the new branch is created with a `NULL`
/// head rather than erroring — mirrors the pre-redesign in-memory behavior
/// (`branches.get(...).map(...).unwrap_or_default()`), which never rejected
/// an unknown `from`.
pub async fn create_branch_from(
    pool: &SqlitePool,
    repo_slug: &str,
    name: &str,
    source: &str,
    is_default: bool,
) -> Branch {
    let head = branch_head_row(pool, repo_slug, source).await.flatten();

    sqlx::query(
        "INSERT INTO branches (repo_slug, name, head_commit_hash, is_default)
         VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(repo_slug)
    .bind(name)
    .bind(&head)
    .bind(is_default)
    .execute(pool)
    .await
    .expect("insert branch");

    Branch {
        name: name.to_string(),
        head: head.unwrap_or_default(),
        is_default,
    }
}

/// Repository slug -> currently checked-out branch name, defaulting to
/// `"main"` when no `current_branch` row exists yet (a repository that has
/// never had `POST /checkout` called against it).
pub async fn get_current_branch(pool: &SqlitePool, repo_slug: &str) -> String {
    sqlx::query("SELECT branch_name FROM current_branch WHERE repo_slug = ?1")
        .bind(repo_slug)
        .fetch_optional(pool)
        .await
        .expect("query current_branch")
        .map(|row| row.get::<String, _>("branch_name"))
        .unwrap_or_else(|| "main".to_string())
}

pub async fn set_current_branch(pool: &SqlitePool, repo_slug: &str, branch_name: &str) {
    sqlx::query(
        "INSERT INTO current_branch (repo_slug, branch_name) VALUES (?1, ?2)
         ON CONFLICT(repo_slug) DO UPDATE SET branch_name = excluded.branch_name",
    )
    .bind(repo_slug)
    .bind(branch_name)
    .execute(pool)
    .await
    .expect("upsert current_branch");
}

// ---------------------------------------------------------------------
// Tree resolution (tree_entries + pending_changes overlay)
// ---------------------------------------------------------------------

/// One resolved file entry — either a committed `tree_entries` row or a
/// staged-but-uncommitted `pending_changes` overlay entry — ready to be
/// assembled into the nested `TreeNode` shape by [`build_tree`], or written
/// back out as a fresh `tree_entries` row by [`create_commit`].
#[derive(Clone)]
struct FlatEntry {
    path: String,
    content_hash: String,
    size_bytes: i64,
    file_kind: String,
    updated_at: i64,
}

/// Fetches the full `tree_entries` snapshot for `commit_hash` into a
/// `path -> FlatEntry` map. Generic over `Executor` so both the read-only
/// call sites (`resolve_current_files`, below) and [`create_commit`]'s open
/// transaction can share this without duplicating the query.
async fn tree_entries_map<'e, E>(
    executor: E,
    repo_slug: &str,
    commit_hash: &str,
) -> BTreeMap<String, FlatEntry>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query(
        "SELECT path, content_hash, size_bytes, file_kind, updated_at
         FROM tree_entries WHERE repo_slug = ?1 AND commit_hash = ?2",
    )
    .bind(repo_slug)
    .bind(commit_hash)
    .fetch_all(executor)
    .await
    .expect("query tree_entries")
    .iter()
    .map(|row| {
        let path: String = row.get("path");
        (
            path.clone(),
            FlatEntry {
                path,
                content_hash: row.get("content_hash"),
                size_bytes: row.get("size_bytes"),
                file_kind: row.get("file_kind"),
                updated_at: row.get("updated_at"),
            },
        )
    })
    .collect()
}

/// A raw `pending_changes` row.
struct PendingRow {
    path: String,
    change_type: FileChangeType,
    content_hash: Option<String>,
    size_bytes: Option<i64>,
}

async fn fetch_pending<'e, E>(executor: E, repo_slug: &str) -> Vec<PendingRow>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query(
        "SELECT path, change_type, content_hash, size_bytes
         FROM pending_changes WHERE repo_slug = ?1",
    )
    .bind(repo_slug)
    .fetch_all(executor)
    .await
    .expect("query pending_changes")
    .iter()
    .map(|row| PendingRow {
        path: row.get("path"),
        change_type: change_type_from_str(&row.get::<String, _>("change_type")),
        content_hash: row.get("content_hash"),
        size_bytes: row.get("size_bytes"),
    })
    .collect()
}

/// Resolves every path currently visible in `repo_slug`'s working copy:
/// `current_branch`'s head commit's `tree_entries`, with `pending_changes`
/// overlaid on top (added/modified entries show the staged content's size;
/// deleted entries are removed). This is the read side of the "staging area"
/// concept — [`create_commit`] is what actually turns an overlay like this
/// into a new real commit.
async fn resolve_current_files(pool: &SqlitePool, repo_slug: &str) -> Vec<FlatEntry> {
    let branch_name = get_current_branch(pool, repo_slug).await;
    let head = branch_head_row(pool, repo_slug, &branch_name)
        .await
        .flatten();

    let mut files: BTreeMap<String, FlatEntry> = match &head {
        Some(commit_hash) => tree_entries_map(pool, repo_slug, commit_hash).await,
        None => BTreeMap::new(),
    };

    for pending in fetch_pending(pool, repo_slug).await {
        match pending.change_type {
            FileChangeType::Deleted => {
                files.remove(&pending.path);
            }
            FileChangeType::Added | FileChangeType::Modified => {
                // A path staged as added/modified but never actually
                // uploaded (content_hash still NULL) has no bytes to show
                // yet — `create_commit` rejects committing it (see below),
                // but until then it simply doesn't appear in the tree.
                let (Some(content_hash), Some(size_bytes)) =
                    (pending.content_hash, pending.size_bytes)
                else {
                    continue;
                };
                let file_kind = file_kind_for_path(&pending.path).to_string();
                files.insert(
                    pending.path.clone(),
                    FlatEntry {
                        path: pending.path,
                        content_hash,
                        size_bytes,
                        file_kind,
                        updated_at: crate::auth::current_unix_time(),
                    },
                );
            }
        }
    }

    files.into_values().collect()
}

pub async fn path_exists(pool: &SqlitePool, repo_slug: &str, path: &str) -> bool {
    resolve_current_files(pool, repo_slug)
        .await
        .iter()
        .any(|e| e.path == path)
}

/// Builds the nested `TreeNode` structure `GET /tree` returns from a flat
/// list of resolved files (see [`resolve_current_files`]) plus a
/// `path -> locked_by` overlay (see `lock_store::list_locks`). Directories
/// are synthesized generically from path segments — not hardcoded to any
/// particular repository's layout — mirroring the shape the old hand-written
/// `state::demo_tree()` produced, just built from real data instead of a
/// literal `vec![...]`.
///
/// Ordering: each directory's children are directories-then-files,
/// alphabetical within each group. The pre-redesign hand-authored demo tree
/// wasn't alphabetical (it followed narrative order), but nothing in the
/// wire contract depends on node order, so a clean deterministic order here
/// is the safer choice over trying to replicate arbitrary hand-picked
/// ordering.
fn build_tree(entries: Vec<FlatEntry>, locks: &HashMap<String, String>, now: i64) -> Vec<TreeNode> {
    enum Trie {
        Dir(BTreeMap<String, Trie>),
        File(FlatEntry),
    }

    /// Recursive insert instead of an iterative mutable-cursor walk —
    /// re-borrowing `&mut BTreeMap` across loop iterations to descend into a
    /// freshly-inserted child runs into an NLL borrow-checker limitation, so
    /// recursion (a fresh, naturally-scoped `&mut` per call) sidesteps it.
    fn insert_path(root: &mut BTreeMap<String, Trie>, segments: &[&str], entry: FlatEntry) {
        let (seg, rest) = segments
            .split_first()
            .expect("a path always has at least one segment");
        if rest.is_empty() {
            root.insert(seg.to_string(), Trie::File(entry));
            return;
        }
        let node = root
            .entry(seg.to_string())
            .or_insert_with(|| Trie::Dir(BTreeMap::new()));
        if let Trie::Dir(children) = node {
            insert_path(children, rest, entry);
        }
        // A path collision between a file and a directory at the same
        // segment can't happen from real tree_entries data (every path is a
        // distinct full string, and a commit can't stage both "Foo" as a
        // file and "Foo/Bar" as a path in the same tree), but silently
        // dropping the entry here is safer than panicking this read path if
        // it somehow did.
    }

    let mut root: BTreeMap<String, Trie> = BTreeMap::new();
    for entry in entries {
        let path = entry.path.clone();
        let segments: Vec<&str> = path.split('/').collect();
        insert_path(&mut root, &segments, entry);
    }

    fn convert(
        trie: BTreeMap<String, Trie>,
        prefix: &str,
        locks: &HashMap<String, String>,
        now: i64,
    ) -> Vec<TreeNode> {
        let mut dirs = Vec::new();
        let mut files = Vec::new();
        for (name, node) in trie {
            let path = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            match node {
                Trie::Dir(children) => {
                    dirs.push(TreeNode::Directory {
                        path: path.clone(),
                        name,
                        children: convert(children, &path, locks, now),
                    });
                }
                Trie::File(entry) => {
                    let size_label = format_size_label(entry.size_bytes);
                    let updated_at = format_relative_time(now, entry.updated_at);
                    let locked_by = locks.get(&path).cloned();
                    let node = match entry.file_kind.as_str() {
                        "text" => TreeNode::Text {
                            path: path.clone(),
                            name,
                            size_label,
                            updated_at,
                            locked_by,
                        },
                        "image" => TreeNode::Image {
                            path: path.clone(),
                            name,
                            size_label,
                            updated_at,
                            locked_by,
                        },
                        "model3d" => TreeNode::Model3d {
                            path: path.clone(),
                            name,
                            size_label,
                            updated_at,
                            locked_by,
                        },
                        "audio" => TreeNode::Audio {
                            path: path.clone(),
                            name,
                            size_label,
                            updated_at,
                            locked_by,
                        },
                        _ => TreeNode::Binary {
                            path: path.clone(),
                            name,
                            size_label,
                            updated_at,
                            locked_by,
                        },
                    };
                    files.push(node);
                }
            }
        }
        dirs.extend(files);
        dirs
    }

    convert(root, "", locks, now)
}

pub async fn get_tree(
    pool: &SqlitePool,
    repo_slug: &str,
    locks: &HashMap<String, String>,
) -> Vec<TreeNode> {
    let entries = resolve_current_files(pool, repo_slug).await;
    build_tree(entries, locks, crate::auth::current_unix_time())
}

// ---------------------------------------------------------------------
// Pending changes (staging area) CRUD
// ---------------------------------------------------------------------

pub async fn upsert_pending_change(
    pool: &SqlitePool,
    repo_slug: &str,
    path: &str,
    change_type: FileChangeType,
    content_hash: Option<&str>,
    size_bytes: Option<i64>,
) {
    sqlx::query(
        "INSERT INTO pending_changes (repo_slug, path, change_type, content_hash, size_bytes, staged_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(repo_slug, path) DO UPDATE SET
             change_type = excluded.change_type,
             content_hash = excluded.content_hash,
             size_bytes = excluded.size_bytes,
             staged_at = excluded.staged_at",
    )
    .bind(repo_slug)
    .bind(path)
    .bind(change_type_to_str(change_type))
    .bind(content_hash)
    .bind(size_bytes)
    .bind(crate::auth::current_unix_time())
    .execute(pool)
    .await
    .expect("upsert pending_changes row");
}

pub async fn delete_pending_change(pool: &SqlitePool, repo_slug: &str, path: &str) {
    sqlx::query("DELETE FROM pending_changes WHERE repo_slug = ?1 AND path = ?2")
        .bind(repo_slug)
        .bind(path)
        .execute(pool)
        .await
        .expect("delete pending_changes row");
}

/// The current repo-wide staging list as wire-shaped `FileChange`s, with a
/// real, computed `sizeDeltaLabel` (previous size from the current branch's
/// head `tree_entries`, if the path exists there; new size from the pending
/// row itself) — replacing the pre-redesign hardcoded `"—"`.
pub async fn pending_file_changes(pool: &SqlitePool, repo_slug: &str) -> Vec<FileChange> {
    let branch_name = get_current_branch(pool, repo_slug).await;
    let head = branch_head_row(pool, repo_slug, &branch_name)
        .await
        .flatten();
    let head_tree: BTreeMap<String, FlatEntry> = match &head {
        Some(commit_hash) => tree_entries_map(pool, repo_slug, commit_hash).await,
        None => BTreeMap::new(),
    };

    fetch_pending(pool, repo_slug)
        .await
        .into_iter()
        .map(|p| {
            let prev_size = head_tree.get(&p.path).map(|e| e.size_bytes);
            let new_size = match p.change_type {
                FileChangeType::Deleted => None,
                _ => p.size_bytes,
            };
            FileChange {
                path: p.path,
                change_type: p.change_type,
                size_delta_label: format_size_delta_label(prev_size, new_size),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------
// Commit creation (the core transactional write)
// ---------------------------------------------------------------------

pub struct NewCommit<'a> {
    pub repo_slug: &'a str,
    pub branch_name: &'a str,
    pub author_email: &'a str,
    pub author_name: &'a str,
    pub author_initials: &'a str,
    pub message: &'a str,
    pub description: Option<&'a str>,
    pub created_at: i64,
}

#[derive(Debug)]
pub enum CommitError {
    /// `pending_changes` is empty for this repo — nothing staged.
    NothingStaged,
    /// One or more staged `added`/`modified` paths have no uploaded content
    /// (`content_hash IS NULL`) — the fix for the "committing fake content"
    /// bug this whole redesign exists to close. Carries the offending paths
    /// so the 400 response can name them.
    MissingContent(Vec<String>),
    /// `branch_name` doesn't name an existing `branches` row for this repo.
    /// Shouldn't happen in practice (every repo gets a `main` row at
    /// creation — see `init_main_branch` — and `create_branch`/`checkout`
    /// both validate against real rows), kept as an explicit variant rather
    /// than a panic so a caller-side bug degrades to a clean error instead
    /// of an `unwrap` panic mid-transaction.
    BranchNotFound,
}

/// The canonicalization this phase's one piece of real cryptographic
/// identity is built from: `repo_slug`, `parent_hash` (empty string if
/// none), `branch_name`, `author_email`, `message`, `description` (empty
/// string if none), then every staged `(path, change_type, content_hash)`
/// triple **sorted by path** (so staging order never changes the resulting
/// hash) each on its own line, and finally `created_at`. Fields are
/// newline-separated and the per-file triples use `\x1f` (ASCII unit
/// separator) between their three parts, so no legal field value can forge a
/// delimiter collision the way a plain comma/colon might. The exact
/// separator scheme has no external meaning — this hash only ever needs to
/// be internally deterministic and content-derived, not reproduce some other
/// system's format — but it's spelled out here so it stays reproducible.
fn compute_commit_hash(
    new: &NewCommit<'_>,
    parent_hash: Option<&str>,
    mut file_entries: Vec<(String, &'static str, Option<String>)>,
) -> String {
    file_entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut buf = String::new();
    buf.push_str(new.repo_slug);
    buf.push('\n');
    buf.push_str(parent_hash.unwrap_or(""));
    buf.push('\n');
    buf.push_str(new.branch_name);
    buf.push('\n');
    buf.push_str(new.author_email);
    buf.push('\n');
    buf.push_str(new.message);
    buf.push('\n');
    buf.push_str(new.description.unwrap_or(""));
    buf.push('\n');
    for (path, change_type, content_hash) in &file_entries {
        buf.push_str(path);
        buf.push('\x1f');
        buf.push_str(change_type);
        buf.push('\x1f');
        buf.push_str(content_hash.as_deref().unwrap_or(""));
        buf.push('\n');
    }
    buf.push_str(&new.created_at.to_string());

    content_hash::sha256_hex(buf.as_bytes())
}

/// The core write path of this whole redesign: turns `repo_slug`'s current
/// `pending_changes` into a real, content-addressed commit, inside a single
/// transaction. Used identically by the live `POST /commits` handler
/// (`created_at = now`) and by `vcs_seed`'s startup demo-history synthesis
/// (`created_at` backdated per seeded commit) — factored out here
/// specifically so neither call site duplicates this transaction's logic.
///
/// Steps (see the plan doc / module docs for the full rationale):
/// 1. `pending_changes` empty -> [`CommitError::NothingStaged`].
/// 2. Any `added`/`modified` row with `content_hash IS NULL` (staged but
///    never uploaded) -> [`CommitError::MissingContent`] — the fix for the
///    pre-redesign bug where a commit could "succeed" over content that was
///    never actually written anywhere.
/// 3. Copy the parent commit's `tree_entries` (empty if this is the first
///    commit on this history), apply the pending changes on top. Paths
///    copied forward unchanged keep their original `updated_at`; only
///    touched paths get `new.created_at` — so a file's "last updated" time
///    reflects when it actually changed, not just the most recent commit
///    that happened to include it in its snapshot.
/// 4. Compute `commit_hash` (see [`compute_commit_hash`]).
/// 5. Insert `commits`/`commit_files`/`tree_entries` rows, advance
///    `branches.head_commit_hash`, clear the committed paths out of
///    `pending_changes`/`staged_content`.
pub async fn create_commit(pool: &SqlitePool, new: NewCommit<'_>) -> Result<Commit, CommitError> {
    let mut tx = pool.begin().await.expect("begin commit transaction");

    let Some(parent_hash) = branch_head_row(&mut *tx, new.repo_slug, new.branch_name).await else {
        return Err(CommitError::BranchNotFound);
    };

    let pending = fetch_pending(&mut *tx, new.repo_slug).await;
    if pending.is_empty() {
        return Err(CommitError::NothingStaged);
    }

    let missing: Vec<String> = pending
        .iter()
        .filter(|p| {
            matches!(
                p.change_type,
                FileChangeType::Added | FileChangeType::Modified
            ) && p.content_hash.is_none()
        })
        .map(|p| p.path.clone())
        .collect();
    if !missing.is_empty() {
        return Err(CommitError::MissingContent(missing));
    }

    let mut tree_map: BTreeMap<String, FlatEntry> = match &parent_hash {
        Some(commit_hash) => tree_entries_map(&mut *tx, new.repo_slug, commit_hash).await,
        None => BTreeMap::new(),
    };

    struct CommitFileEntry {
        path: String,
        change_type: FileChangeType,
        content_hash: Option<String>,
        size_bytes: Option<i64>,
        previous_content_hash: Option<String>,
        previous_size_bytes: Option<i64>,
    }

    let mut commit_files = Vec::with_capacity(pending.len());
    let mut hash_entries: Vec<(String, &'static str, Option<String>)> =
        Vec::with_capacity(pending.len());

    for p in &pending {
        let previous = tree_map.get(&p.path).cloned();
        hash_entries.push((
            p.path.clone(),
            change_type_to_str(p.change_type),
            p.content_hash.clone(),
        ));

        match p.change_type {
            FileChangeType::Added | FileChangeType::Modified => {
                let content_hash = p.content_hash.clone().expect("validated non-null above");
                let size_bytes = p.size_bytes.expect("validated non-null above");
                tree_map.insert(
                    p.path.clone(),
                    FlatEntry {
                        path: p.path.clone(),
                        content_hash: content_hash.clone(),
                        size_bytes,
                        file_kind: file_kind_for_path(&p.path).to_string(),
                        updated_at: new.created_at,
                    },
                );
                commit_files.push(CommitFileEntry {
                    path: p.path.clone(),
                    change_type: p.change_type,
                    content_hash: Some(content_hash),
                    size_bytes: Some(size_bytes),
                    previous_content_hash: previous.as_ref().map(|e| e.content_hash.clone()),
                    previous_size_bytes: previous.as_ref().map(|e| e.size_bytes),
                });
            }
            FileChangeType::Deleted => {
                tree_map.remove(&p.path);
                commit_files.push(CommitFileEntry {
                    path: p.path.clone(),
                    change_type: FileChangeType::Deleted,
                    content_hash: None,
                    size_bytes: None,
                    previous_content_hash: previous.as_ref().map(|e| e.content_hash.clone()),
                    previous_size_bytes: previous.as_ref().map(|e| e.size_bytes),
                });
            }
        }
    }

    let hash = compute_commit_hash(&new, parent_hash.as_deref(), hash_entries);
    let short_hash = hash[..12.min(hash.len())].to_string();

    sqlx::query(
        "INSERT INTO commits
            (repo_slug, hash, parent_hash, message, description,
             author_email, author_name, author_initials, branch_name, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )
    .bind(new.repo_slug)
    .bind(&hash)
    .bind(&parent_hash)
    .bind(new.message)
    .bind(new.description)
    .bind(new.author_email)
    .bind(new.author_name)
    .bind(new.author_initials)
    .bind(new.branch_name)
    .bind(new.created_at)
    .execute(&mut *tx)
    .await
    .expect("insert commits row");

    let mut changed_files = Vec::with_capacity(commit_files.len());
    for cf in &commit_files {
        sqlx::query(
            "INSERT INTO commit_files
                (repo_slug, commit_hash, path, change_type, content_hash, size_bytes, previous_content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(new.repo_slug)
        .bind(&hash)
        .bind(&cf.path)
        .bind(change_type_to_str(cf.change_type))
        .bind(&cf.content_hash)
        .bind(cf.size_bytes)
        .bind(&cf.previous_content_hash)
        .execute(&mut *tx)
        .await
        .expect("insert commit_files row");

        changed_files.push(FileChange {
            path: cf.path.clone(),
            change_type: cf.change_type,
            size_delta_label: format_size_delta_label(cf.previous_size_bytes, cf.size_bytes),
        });
    }

    for entry in tree_map.values() {
        sqlx::query(
            "INSERT INTO tree_entries
                (repo_slug, commit_hash, path, content_hash, size_bytes, file_kind, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(new.repo_slug)
        .bind(&hash)
        .bind(&entry.path)
        .bind(&entry.content_hash)
        .bind(entry.size_bytes)
        .bind(&entry.file_kind)
        .bind(entry.updated_at)
        .execute(&mut *tx)
        .await
        .expect("insert tree_entries row");
    }

    sqlx::query("UPDATE branches SET head_commit_hash = ?1 WHERE repo_slug = ?2 AND name = ?3")
        .bind(&hash)
        .bind(new.repo_slug)
        .bind(new.branch_name)
        .execute(&mut *tx)
        .await
        .expect("advance branch head");

    for p in &pending {
        sqlx::query("DELETE FROM pending_changes WHERE repo_slug = ?1 AND path = ?2")
            .bind(new.repo_slug)
            .bind(&p.path)
            .execute(&mut *tx)
            .await
            .expect("clear pending_changes row");
        sqlx::query("DELETE FROM staged_content WHERE repo_slug = ?1 AND path = ?2")
            .bind(new.repo_slug)
            .bind(&p.path)
            .execute(&mut *tx)
            .await
            .expect("clear staged_content row");
    }

    tx.commit().await.expect("commit transaction");

    Ok(Commit {
        hash: hash.clone(),
        short_hash,
        message: new.message.to_string(),
        description: new.description.map(|d| d.to_string()),
        author: new.author_name.to_string(),
        author_initials: new.author_initials.to_string(),
        timestamp: format_relative_time(crate::auth::current_unix_time(), new.created_at),
        changed_files,
        branch: new.branch_name.to_string(),
        parents: parent_hash.into_iter().collect(),
    })
}

// ---------------------------------------------------------------------
// Commit history reads
// ---------------------------------------------------------------------

async fn commit_changed_files(
    pool: &SqlitePool,
    repo_slug: &str,
    commit_hash: &str,
) -> Vec<FileChange> {
    sqlx::query(
        "SELECT cf.path AS path, cf.change_type AS change_type, cf.size_bytes AS size_bytes,
                fb.size_bytes AS prev_size_bytes
         FROM commit_files cf
         LEFT JOIN file_blobs fb
             ON fb.repo_slug = cf.repo_slug AND fb.content_hash = cf.previous_content_hash
         WHERE cf.repo_slug = ?1 AND cf.commit_hash = ?2",
    )
    .bind(repo_slug)
    .bind(commit_hash)
    .fetch_all(pool)
    .await
    .expect("query commit_files")
    .iter()
    .map(|row| {
        let change_type = change_type_from_str(&row.get::<String, _>("change_type"));
        let new_size: Option<i64> = row.get("size_bytes");
        let prev_size: Option<i64> = row.get("prev_size_bytes");
        FileChange {
            path: row.get("path"),
            change_type,
            size_delta_label: format_size_delta_label(prev_size, new_size),
        }
    })
    .collect()
}

fn row_to_commit_shell(row: &SqliteRow, now: i64) -> Commit {
    let created_at: i64 = row.get("created_at");
    let hash: String = row.get("hash");
    let short_hash = hash[..12.min(hash.len())].to_string();
    Commit {
        hash,
        short_hash,
        message: row.get("message"),
        description: row.get("description"),
        author: row.get("author_name"),
        author_initials: row.get("author_initials"),
        timestamp: format_relative_time(now, created_at),
        changed_files: Vec::new(),
        branch: row.get("branch_name"),
        parents: row
            .get::<Option<String>, _>("parent_hash")
            .into_iter()
            .collect(),
    }
}

/// All commits for `repo_slug`, oldest first (matches the pre-redesign
/// `AppState.commits`' documented ordering convention).
pub async fn list_commits(pool: &SqlitePool, repo_slug: &str) -> Vec<Commit> {
    let now = crate::auth::current_unix_time();
    let rows = sqlx::query(
        "SELECT hash, parent_hash, message, description, author_name, author_initials,
                branch_name, created_at
         FROM commits WHERE repo_slug = ?1 ORDER BY created_at ASC",
    )
    .bind(repo_slug)
    .fetch_all(pool)
    .await
    .expect("query commits");

    let mut commits = Vec::with_capacity(rows.len());
    for row in &rows {
        let mut commit = row_to_commit_shell(row, now);
        commit.changed_files = commit_changed_files(pool, repo_slug, &commit.hash).await;
        commits.push(commit);
    }
    commits
}

pub async fn get_commit(pool: &SqlitePool, repo_slug: &str, hash: &str) -> Option<Commit> {
    let now = crate::auth::current_unix_time();
    let row = sqlx::query(
        "SELECT hash, parent_hash, message, description, author_name, author_initials,
                branch_name, created_at
         FROM commits WHERE repo_slug = ?1 AND hash = ?2",
    )
    .bind(repo_slug)
    .bind(hash)
    .fetch_optional(pool)
    .await
    .expect("query commit");

    let row = row?;
    let mut commit = row_to_commit_shell(&row, now);
    commit.changed_files = commit_changed_files(pool, repo_slug, &commit.hash).await;
    Some(commit)
}

/// Number of commits recorded for `repo_slug` — used by `vcs_seed` to decide
/// whether a seeded repository already has real history (skip) or still
/// needs it synthesized (first run).
pub async fn commit_count(pool: &SqlitePool, repo_slug: &str) -> i64 {
    sqlx::query("SELECT COUNT(*) AS count FROM commits WHERE repo_slug = ?1")
        .bind(repo_slug)
        .fetch_one(pool)
        .await
        .expect("count commits")
        .get("count")
}

// ---------------------------------------------------------------------
// Diff support (Phase 5) — resolving a path's content across two commits.
// ---------------------------------------------------------------------

/// `true` iff `hash` names a real commit recorded for `repo_slug`. Used by
/// `handlers::get_diff` to tell an actually-unknown/invalid commit hash
/// (404) apart from a valid commit that simply has no `tree_entries` row for
/// one particular path — the latter is a normal "the path didn't exist at
/// that point in history" state (handled by the added/deleted logic in
/// `get_diff`), not an error, and must not also 404.
pub async fn commit_exists(pool: &SqlitePool, repo_slug: &str, hash: &str) -> bool {
    sqlx::query("SELECT 1 FROM commits WHERE repo_slug = ?1 AND hash = ?2")
        .bind(repo_slug)
        .bind(hash)
        .fetch_optional(pool)
        .await
        .expect("query commit existence")
        .is_some()
}

/// Resolves `path`'s `content_hash` in `commit_hash`'s tree snapshot
/// (`tree_entries`), or `None` if that path didn't exist in the tree at that
/// commit. Callers that need to distinguish "path absent" from "commit
/// itself doesn't exist" should check [`commit_exists`] separately — this
/// function alone can't tell the two apart (an unknown commit hash and a
/// valid-but-pathless one both look like zero matching rows here).
pub async fn resolve_content_hash_at_commit(
    pool: &SqlitePool,
    repo_slug: &str,
    commit_hash: &str,
    path: &str,
) -> Option<String> {
    sqlx::query(
        "SELECT content_hash FROM tree_entries
         WHERE repo_slug = ?1 AND commit_hash = ?2 AND path = ?3",
    )
    .bind(repo_slug)
    .bind(commit_hash)
    .bind(path)
    .fetch_optional(pool)
    .await
    .expect("query tree_entries for path at commit")
    .map(|row| row.get("content_hash"))
}
