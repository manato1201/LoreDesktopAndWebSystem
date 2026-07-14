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
