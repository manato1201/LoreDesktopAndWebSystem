"use client";

import { useState } from "react";
import { toggleLock } from "@/lib/api";
import type { TreeNode } from "@/lib/types";
import { findNode } from "@/lib/tree-utils";
import { FileDetail } from "./FileDetail";
import { PlaceholderScreen } from "./PlaceholderScreen";
import { ReadmePreview } from "./ReadmePreview";
import { RepositoryIcon } from "./icons";
import { TreeView } from "./TreeView";

export function RepositoryExplorer({
  repoSlug,
  tree: initialTree,
}: {
  repoSlug: string;
  tree: TreeNode[];
}) {
  const [tree, setTree] = useState(initialTree);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);

  const selectedNode = selectedPath ? findNode(tree, selectedPath) : undefined;
  const selectedFile =
    selectedNode && selectedNode.kind !== "directory" ? selectedNode : null;

  const readmeNode = tree.find(
    (node) =>
      node.kind !== "directory" && node.name.toLowerCase() === "readme.md",
  );

  const handleToggleLock = async () => {
    if (!selectedFile) return;
    const updated = await toggleLock(
      repoSlug,
      selectedFile.path,
      !selectedFile.lockedBy,
    );
    if (updated) setTree(updated);
  };

  return (
    <div className="flex flex-col gap-4 lg:flex-row lg:items-start">
      <div
        className={`w-full shrink-0 rounded-comfortable bg-surface p-2 lg:block lg:w-72 ${
          selectedFile ? "hidden" : "block"
        }`}
      >
        {tree.length > 0 ? (
          <TreeView
            nodes={tree}
            selectedPath={selectedPath}
            onSelect={setSelectedPath}
          />
        ) : (
          <p className="p-3 text-xs text-text-secondary">
            No files yet. Push to this repository to see its file tree here.
          </p>
        )}
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
              repoSlug={repoSlug}
              node={selectedFile}
              onToggleLock={handleToggleLock}
            />
          </>
        ) : readmeNode ? (
          <ReadmePreview repoSlug={repoSlug} path={readmeNode.path} />
        ) : tree.length === 0 ? (
          <PlaceholderScreen
            icon={RepositoryIcon}
            title="This repository is empty"
            description="Push files to this repository to see its file tree and README here."
          />
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
