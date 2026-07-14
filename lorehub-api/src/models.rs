use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Private,
    Internal,
    Public,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    pub slug: String,
    pub name: String,
    pub organization: String,
    pub description: String,
    pub updated_at: String,
    pub size_label: String,
    pub locked_file_count: u32,
    pub visibility: Visibility,
}

/// Mirrors `TreeNode` in lorehub-web/src/lib/types.ts. Every file-kind
/// variant shares the same fields — Rust's tagged-enum serde needs a
/// literal discriminant per kind, unlike the TS union's single `FileKind`
/// field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TreeNode {
    #[serde(rename = "directory")]
    Directory {
        path: String,
        name: String,
        children: Vec<TreeNode>,
    },
    #[serde(rename = "text")]
    Text {
        path: String,
        name: String,
        size_label: String,
        updated_at: String,
        locked_by: Option<String>,
    },
    #[serde(rename = "image")]
    Image {
        path: String,
        name: String,
        size_label: String,
        updated_at: String,
        locked_by: Option<String>,
    },
    #[serde(rename = "model3d")]
    Model3d {
        path: String,
        name: String,
        size_label: String,
        updated_at: String,
        locked_by: Option<String>,
    },
    #[serde(rename = "audio")]
    Audio {
        path: String,
        name: String,
        size_label: String,
        updated_at: String,
        locked_by: Option<String>,
    },
    #[serde(rename = "binary")]
    Binary {
        path: String,
        name: String,
        size_label: String,
        updated_at: String,
        locked_by: Option<String>,
    },
}

impl TreeNode {
    pub fn path(&self) -> &str {
        match self {
            TreeNode::Directory { path, .. }
            | TreeNode::Text { path, .. }
            | TreeNode::Image { path, .. }
            | TreeNode::Model3d { path, .. }
            | TreeNode::Audio { path, .. }
            | TreeNode::Binary { path, .. } => path,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FileChangeType {
    Added,
    Modified,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub change_type: FileChangeType,
    pub size_delta_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub author: String,
    pub author_initials: String,
    pub timestamp: String,
    pub changed_files: Vec<FileChange>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PrStatus {
    Open,
    Merged,
    Closed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiffLineType {
    Context,
    Add,
    Remove,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    #[serde(rename = "type")]
    pub kind: DiffLineType,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "diffKind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PrDiffFile {
    #[serde(rename = "text")]
    Text {
        path: String,
        change_type: FileChangeType,
        lines: Vec<DiffLine>,
    },
    #[serde(rename = "image")]
    Image {
        path: String,
        change_type: FileChangeType,
    },
    #[serde(rename = "model3d")]
    Model3d {
        path: String,
        change_type: FileChangeType,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrComment {
    pub id: String,
    pub author: String,
    pub author_initials: String,
    pub timestamp: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub repo_slug: String,
    pub repo_name: String,
    pub status: PrStatus,
    pub author: String,
    pub author_initials: String,
    pub created_at: String,
    pub updated_at: String,
    pub changed_files: Vec<PrDiffFile>,
    pub comments: Vec<PrComment>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum PermissionLevel {
    Read,
    Write,
    Lock,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PrincipalType {
    User,
    Team,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessEntry {
    pub principal: String,
    pub principal_type: PrincipalType,
    pub permissions: Vec<PermissionLevel>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MemberRole {
    Owner,
    Admin,
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgMember {
    pub name: String,
    pub initials: String,
    pub email: String,
    pub role: MemberRole,
    pub joined_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogEntry {
    pub id: String,
    pub actor: String,
    pub action: String,
    pub target: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageUsage {
    pub used_label: String,
    pub total_label: String,
    pub used_percent: u8,
}
