"use client";

import { useState } from "react";
import type { TreeNode } from "@/lib/types";
import {
  AudioIcon,
  ChevronRightIcon,
  FileTextIcon,
  FolderIcon,
  ImageIcon,
  LockIcon,
  Model3DIcon,
} from "./icons";

const FILE_ICON: Record<
  Exclude<TreeNode["kind"], "directory">,
  typeof FileTextIcon
> = {
  text: FileTextIcon,
  binary: FileTextIcon,
  image: ImageIcon,
  model3d: Model3DIcon,
  audio: AudioIcon,
};

function TreeRow({
  node,
  depth,
  selectedPath,
  onSelect,
  expanded,
  onToggle,
}: {
  node: TreeNode;
  depth: number;
  selectedPath: string | null;
  onSelect: (path: string) => void;
  expanded: Set<string>;
  onToggle: (path: string) => void;
}) {
  const indent = depth * 16 + 8;

  if (node.kind === "directory") {
    const isOpen = expanded.has(node.path);
    return (
      <div>
        <button
          type="button"
          onClick={() => onToggle(node.path)}
          aria-expanded={isOpen}
          style={{ paddingLeft: indent }}
          className="flex w-full items-center gap-1.5 rounded-subtle py-1.5 pr-2 text-left text-sm text-text-secondary transition-colors hover:bg-surface-interactive hover:text-text-primary"
        >
          <ChevronRightIcon
            className={`size-3.5 shrink-0 transition-transform ${isOpen ? "rotate-90" : ""}`}
          />
          <FolderIcon className="size-4 shrink-0" />
          <span className="truncate">{node.name}</span>
        </button>
        {isOpen &&
          node.children.map((child) => (
            <TreeRow
              key={child.path}
              node={child}
              depth={depth + 1}
              selectedPath={selectedPath}
              onSelect={onSelect}
              expanded={expanded}
              onToggle={onToggle}
            />
          ))}
      </div>
    );
  }

  const Icon = FILE_ICON[node.kind];
  const isSelected = node.path === selectedPath;

  return (
    <button
      type="button"
      onClick={() => onSelect(node.path)}
      aria-current={isSelected ? "true" : undefined}
      style={{ paddingLeft: indent + 20 }}
      className={`flex w-full items-center gap-1.5 rounded-subtle py-1.5 pr-2 text-left text-sm transition-colors ${
        isSelected
          ? "bg-surface-interactive font-bold text-text-primary"
          : "text-text-secondary hover:bg-surface-interactive hover:text-text-primary"
      }`}
    >
      <Icon className="size-4 shrink-0" />
      <span className="truncate">{node.name}</span>
      {node.lockedBy && (
        <LockIcon className="ml-auto size-3.5 shrink-0 text-warning" />
      )}
    </button>
  );
}

export function TreeView({
  nodes,
  selectedPath,
  onSelect,
}: {
  nodes: TreeNode[];
  selectedPath: string | null;
  onSelect: (path: string) => void;
}) {
  const [expanded, setExpanded] = useState<Set<string>>(
    () =>
      new Set(nodes.filter((n) => n.kind === "directory").map((n) => n.path)),
  );

  const onToggle = (path: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  };

  return (
    <nav aria-label="File tree" className="flex flex-col gap-0.5">
      {nodes.map((node) => (
        <TreeRow
          key={node.path}
          node={node}
          depth={0}
          selectedPath={selectedPath}
          onSelect={onSelect}
          expanded={expanded}
          onToggle={onToggle}
        />
      ))}
    </nav>
  );
}
