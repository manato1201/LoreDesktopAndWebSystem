import type { DirectoryTreeNode, TreeNode } from "./types";

export type PureDirectoryNode = {
  kind: "directory";
  path: string;
  name: string;
  children: PureDirectoryNode[];
};

export function findNode(
  nodes: TreeNode[],
  path: string,
): TreeNode | undefined {
  for (const node of nodes) {
    if (node.path === path) return node;
    if (node.kind === "directory") {
      const found = findNode(node.children, path);
      if (found) return found;
    }
  }
  return undefined;
}

export function directoriesOnly(nodes: TreeNode[]): PureDirectoryNode[] {
  return nodes
    .filter((node): node is DirectoryTreeNode => node.kind === "directory")
    .map((node) => ({ ...node, children: directoriesOnly(node.children) }));
}

export function withLockOverrides(
  nodes: TreeNode[],
  overrides: Record<string, string | null>,
): TreeNode[] {
  return nodes.map((node) => {
    if (node.kind === "directory") {
      return { ...node, children: withLockOverrides(node.children, overrides) };
    }
    if (node.path in overrides) {
      return { ...node, lockedBy: overrides[node.path] };
    }
    return node;
  });
}
