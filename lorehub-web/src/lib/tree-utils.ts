import type { TreeNode } from "./types";

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
