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
