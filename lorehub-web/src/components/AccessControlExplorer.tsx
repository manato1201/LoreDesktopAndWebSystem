"use client";

import { useState } from "react";
import { togglePermission as togglePermissionApi } from "@/lib/api";
import type { AccessEntry, PermissionLevel } from "@/lib/types";
import type { PureDirectoryNode } from "@/lib/tree-utils";
import { DirectoryTree } from "./DirectoryTree";
import { PermissionMatrix } from "./PermissionMatrix";

export function AccessControlExplorer({
  directories,
  initialEntries,
}: {
  directories: PureDirectoryNode[];
  initialEntries: Record<string, AccessEntry[]>;
}) {
  const [selectedPath, setSelectedPath] = useState(directories[0]?.path ?? "");
  const [entriesByPath, setEntriesByPath] = useState(initialEntries);

  const handleTogglePermission = async (
    principal: string,
    level: PermissionLevel,
  ) => {
    const updated = await togglePermissionApi(selectedPath, principal, level);
    if (updated) {
      setEntriesByPath((prev) => ({ ...prev, [selectedPath]: updated }));
    }
  };

  return (
    <div className="flex flex-col gap-4 lg:flex-row lg:items-start">
      <div className="w-full shrink-0 rounded-comfortable bg-surface p-2 lg:w-72">
        <DirectoryTree
          nodes={directories}
          selectedPath={selectedPath}
          onSelect={setSelectedPath}
        />
      </div>

      <div className="min-w-0 flex-1">
        <h2 className="mb-3 text-sm font-semibold text-text-secondary">
          Permissions on{" "}
          <code className="text-text-primary">{selectedPath}</code>
        </h2>
        <PermissionMatrix
          path={selectedPath}
          entries={entriesByPath[selectedPath] ?? []}
          onTogglePermission={handleTogglePermission}
        />
      </div>
    </div>
  );
}
