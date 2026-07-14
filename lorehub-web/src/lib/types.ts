export type Repository = {
  slug: string;
  name: string;
  organization: string;
  description: string;
  updatedAt: string;
  sizeLabel: string;
  lockedFileCount: number;
  visibility: "private" | "internal" | "public";
};

export type FileKind = "text" | "image" | "model3d" | "audio" | "binary";

export type DirectoryTreeNode = {
  kind: "directory";
  path: string;
  name: string;
  children: TreeNode[];
};

export type FileTreeNode = {
  kind: FileKind;
  path: string;
  name: string;
  sizeLabel: string;
  updatedAt: string;
  lockedBy: string | null;
};

export type TreeNode = DirectoryTreeNode | FileTreeNode;

export type FileChangeType = "added" | "modified" | "deleted";

export type FileChange = {
  path: string;
  changeType: FileChangeType;
  sizeDeltaLabel: string;
};

export type Commit = {
  hash: string;
  shortHash: string;
  message: string;
  description?: string;
  author: string;
  authorInitials: string;
  timestamp: string;
  changedFiles: FileChange[];
};

export type PRStatus = "open" | "merged" | "closed";

export type DiffLine = {
  type: "context" | "add" | "remove";
  text: string;
};

export type PRDiffFile =
  | {
      diffKind: "text";
      path: string;
      changeType: FileChangeType;
      lines: DiffLine[];
    }
  | {
      diffKind: "image" | "model3d";
      path: string;
      changeType: FileChangeType;
    };

export type PRComment = {
  id: string;
  author: string;
  authorInitials: string;
  timestamp: string;
  body: string;
};

export type PullRequest = {
  id: string;
  title: string;
  description: string;
  repoSlug: string;
  repoName: string;
  status: PRStatus;
  author: string;
  authorInitials: string;
  createdAt: string;
  updatedAt: string;
  changedFiles: PRDiffFile[];
  comments: PRComment[];
};

export type PermissionLevel = "read" | "write" | "lock";

export type AccessEntry = {
  principal: string;
  principalType: "user" | "team";
  permissions: PermissionLevel[];
};

export type MemberRole = "owner" | "admin" | "member";

export type OrgMember = {
  name: string;
  initials: string;
  email: string;
  role: MemberRole;
  joinedAt: string;
};

export type AuditLogEntry = {
  id: string;
  actor: string;
  action: string;
  target: string;
  timestamp: string;
};

export type StorageUsage = {
  usedLabel: string;
  totalLabel: string;
  usedPercent: number;
};
