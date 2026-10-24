"use client";

import { useState, type ReactNode } from "react";
import { toggleLock } from "@/lib/api";
import type { TreeNode } from "@/lib/types";
import { findNode } from "@/lib/tree-utils";
import { Breadcrumb } from "./Breadcrumb";
import { EmptyRepositoryGuide } from "./EmptyRepositoryGuide";
import { FileDetail } from "./FileDetail";
import { PlaceholderScreen } from "./PlaceholderScreen";
import { ReadmePreview } from "./ReadmePreview";
import { RepositoryIcon } from "./icons";
import { TreeView } from "./TreeView";

/**
 * Left: file tree (stays in view while a long page scrolls). Centre: the
 * selected file, the README, or a guide when the repository is empty. Right:
 * the summary passed in as `summary` (shown below the content on narrower
 * screens).
 */
export function RepositoryExplorer({
  repoSlug,
  tree: initialTree,
  summary,
}: {
  repoSlug: string;
  tree: TreeNode[];
  summary: ReactNode;
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
    <div className="grid gap-4 lg:grid-cols-[18rem_minmax(0,1fr)] xl:grid-cols-[18rem_minmax(0,1fr)_16rem]">
      <div
        className={`rounded-comfortable bg-surface p-2 lg:sticky lg:top-4 lg:block lg:max-h-[calc(100vh-2rem)] lg:self-start lg:overflow-y-auto ${
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
          <p className="p-3 text-xs text-text-secondary">No files yet.</p>
        )}
      </div>

      <div
        className={`min-w-0 rounded-comfortable bg-surface p-5 lg:block ${
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
            <Breadcrumb
              items={selectedFile.path
                .split("/")
                .map((segment) => ({ label: segment }))}
            />
            <FileDetail
              repoSlug={repoSlug}
              node={selectedFile}
              onToggleLock={handleToggleLock}
            />
          </>
        ) : readmeNode ? (
          <ReadmePreview repoSlug={repoSlug} path={readmeNode.path} />
        ) : tree.length === 0 ? (
          <EmptyRepositoryGuide />
        ) : (
          <PlaceholderScreen
            icon={RepositoryIcon}
            title="Select a file"
            description="Choose a file from the tree to preview its contents, metadata, and lock status."
          />
        )}
      </div>

      <div
        className={`lg:col-span-2 xl:col-span-1 ${
          selectedFile ? "hidden lg:block" : "block"
        }`}
      >
        {summary}
      </div>
    </div>
  );
}
