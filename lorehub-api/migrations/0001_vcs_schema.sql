-- New content-addressed VCS schema (see plan doc: lorehub-api VCS backend
-- redesign, Phase 1). Foundation-only: these tables are not yet read from or
-- written to by any handler — that wiring lands in later phases. Copied
-- verbatim from the approved plan's schema section.
--
-- All `repo_slug` foreign keys use `ON DELETE CASCADE` so that
-- `DELETE FROM repositories WHERE slug = ?` atomically removes every related
-- row across every VCS table in one statement, once `PRAGMA foreign_keys =
-- ON` is enabled per-connection (see `db::connect`).

CREATE TABLE repositories (
    slug TEXT PRIMARY KEY, name TEXT NOT NULL, organization TEXT NOT NULL,
    description TEXT NOT NULL,
    visibility TEXT NOT NULL CHECK (visibility IN ('private','internal','public')),
    is_seeded INTEGER NOT NULL DEFAULT 0,
    size_label TEXT NOT NULL DEFAULT '0 B',
    locked_file_count INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT 'just now',
    created_at INTEGER NOT NULL
);

-- File content bytes live on the filesystem (see src/blob_store.rs), not in
-- this table — this is metadata (hash/size) only.
CREATE TABLE file_blobs (
    repo_slug TEXT NOT NULL REFERENCES repositories(slug) ON DELETE CASCADE,
    content_hash TEXT NOT NULL,       -- sha256 hex, 64 chars
    size_bytes INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (repo_slug, content_hash)
);

CREATE TABLE branches (
    repo_slug TEXT NOT NULL REFERENCES repositories(slug) ON DELETE CASCADE,
    name TEXT NOT NULL,
    head_commit_hash TEXT,            -- NULL = no commits yet
    is_default INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (repo_slug, name)
);

CREATE TABLE current_branch (
    repo_slug TEXT PRIMARY KEY REFERENCES repositories(slug) ON DELETE CASCADE,
    branch_name TEXT NOT NULL
);

CREATE TABLE commits (
    repo_slug TEXT NOT NULL REFERENCES repositories(slug) ON DELETE CASCADE,
    hash TEXT NOT NULL,               -- sha256 hex, derived from real content
    parent_hash TEXT,                 -- NULL = first commit. Single parent only
    message TEXT NOT NULL, description TEXT,
    author_email TEXT NOT NULL, author_name TEXT NOT NULL, author_initials TEXT NOT NULL,
    branch_name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (repo_slug, hash),
    FOREIGN KEY (repo_slug, parent_hash) REFERENCES commits(repo_slug, hash)
);
CREATE INDEX idx_commits_repo_created ON commits(repo_slug, created_at);

CREATE TABLE commit_files (
    repo_slug TEXT NOT NULL, commit_hash TEXT NOT NULL, path TEXT NOT NULL,
    change_type TEXT NOT NULL CHECK (change_type IN ('added','modified','deleted')),
    content_hash TEXT,                -- NULL for deleted
    size_bytes INTEGER, previous_content_hash TEXT,
    PRIMARY KEY (repo_slug, commit_hash, path),
    FOREIGN KEY (repo_slug, commit_hash) REFERENCES commits(repo_slug, hash) ON DELETE CASCADE,
    FOREIGN KEY (repo_slug, content_hash) REFERENCES file_blobs(repo_slug, content_hash)
);

-- Full tree snapshot per commit. Makes branch-aware reads O(1) (resolve
-- head_commit_hash, then a single indexed lookup here) instead of replaying
-- history.
CREATE TABLE tree_entries (
    repo_slug TEXT NOT NULL, commit_hash TEXT NOT NULL, path TEXT NOT NULL,
    content_hash TEXT NOT NULL, size_bytes INTEGER NOT NULL,
    file_kind TEXT NOT NULL CHECK (file_kind IN ('text','image','model3d','audio','binary')),
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (repo_slug, commit_hash, path),
    FOREIGN KEY (repo_slug, commit_hash) REFERENCES commits(repo_slug, hash) ON DELETE CASCADE,
    FOREIGN KEY (repo_slug, content_hash) REFERENCES file_blobs(repo_slug, content_hash)
);
CREATE INDEX idx_tree_entries_lookup ON tree_entries(repo_slug, commit_hash);

-- Uncommitted "working copy" content. One current value per path (mirrors an
-- actual working directory).
CREATE TABLE staged_content (
    repo_slug TEXT NOT NULL REFERENCES repositories(slug) ON DELETE CASCADE,
    path TEXT NOT NULL, content_hash TEXT NOT NULL, size_bytes INTEGER NOT NULL,
    uploaded_at INTEGER NOT NULL,
    PRIMARY KEY (repo_slug, path),
    FOREIGN KEY (repo_slug, content_hash) REFERENCES file_blobs(repo_slug, content_hash)
);

-- content_hash/size are locked in at stage time (git index-like behavior: a
-- re-upload after staging must not silently change what a later commit
-- captures).
CREATE TABLE pending_changes (
    repo_slug TEXT NOT NULL REFERENCES repositories(slug) ON DELETE CASCADE,
    path TEXT NOT NULL,
    change_type TEXT NOT NULL CHECK (change_type IN ('added','modified','deleted')),
    content_hash TEXT, size_bytes INTEGER, staged_at INTEGER NOT NULL,
    PRIMARY KEY (repo_slug, path),
    FOREIGN KEY (repo_slug, content_hash) REFERENCES file_blobs(repo_slug, content_hash)
);

-- Exclusive locks. Independent of commit history (Perforce-style, not
-- versioned).
CREATE TABLE file_locks (
    repo_slug TEXT NOT NULL REFERENCES repositories(slug) ON DELETE CASCADE,
    path TEXT NOT NULL, locked_by TEXT NOT NULL, locked_at INTEGER NOT NULL,
    PRIMARY KEY (repo_slug, path)
);
