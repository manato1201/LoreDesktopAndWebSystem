"use client";

import { useMemo, useState } from "react";
import { mockFileContents } from "@/lib/mock-tree";
import type { TreeNode } from "@/lib/types";
import { findNode, withLockOverrides } from "@/lib/tree-utils";
import { FileDetail } from "./FileDetail";
import { PlaceholderScreen } from "./PlaceholderScreen";
import { RepositoryIcon } from "./icons";
import { TreeView } from "./TreeView";

export function RepositoryExplorer({ tree }: { tree: TreeNode[] }) {
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [lockOverrides, setLockOverrides] = useState<
    Record<string, string | null>
  >({});

  const effectiveTree = useMemo(
    () => withLockOverrides(tree, lockOverrides),
    [tree, lockOverrides],
  );

  const selectedNode = selectedPath
    ? findNode(effectiveTree, selectedPath)
    : undefined;
  const selectedFile =
    selectedNode && selectedNode.kind !== "directory" ? selectedNode : null;

  const handleToggleLock = () => {
    if (!selectedFile) return;
    setLockOverrides((prev) => ({
      ...prev,
      [selectedFile.path]: selectedFile.lockedBy ? null : "You",
    }));
  };

  return (
    <div className="flex flex-col gap-4 lg:flex-row lg:items-start">
      <div
        className={`w-full shrink-0 rounded-comfortable bg-surface p-2 lg:block lg:w-72 ${
          selectedFile ? "hidden" : "block"
        }`}
      >
        <TreeView
          nodes={effectiveTree}
          selectedPath={selectedPath}
          onSelect={setSelectedPath}
        />
      </div>

      <div
        className={`min-w-0 flex-1 rounded-comfortable bg-surface p-5 lg:block ${
          selectedFile ? "block" : "hidden"
        }`}
      >
        {selectedFile ? (
          <>
            <button
              type="button"
              onClick={() => setSelectedPath(null)}
              className="mb-4 text-xs text-text-secondary hover:text-text-primary lg:hidden"
            >
              ← Back to file tree
            </button>
            <FileDetail
              node={selectedFile}
              content={mockFileContents[selectedFile.path]}
              onToggleLock={handleToggleLock}
            />
          </>
        ) : (
          <PlaceholderScreen
            icon={RepositoryIcon}
            title="Select a file"
            description="Choose a file from the tree to preview its contents, metadata, and lock status."
          />
        )}
      </div>
    </div>
  );
}
