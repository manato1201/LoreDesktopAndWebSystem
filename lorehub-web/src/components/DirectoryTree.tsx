"use client";

import { useState } from "react";
import type { PureDirectoryNode } from "@/lib/tree-utils";
import { ChevronRightIcon, FolderIcon } from "./icons";

function DirRow({
  node,
  depth,
  selectedPath,
  onSelect,
  expanded,
  onToggle,
}: {
  node: PureDirectoryNode;
  depth: number;
  selectedPath: string;
  onSelect: (path: string) => void;
  expanded: Set<string>;
  onToggle: (path: string) => void;
}) {
  const isOpen = expanded.has(node.path);
  const isSelected = node.path === selectedPath;

  return (
    <div>
      <button
        type="button"
        onClick={() => {
          onSelect(node.path);
          onToggle(node.path);
        }}
        aria-current={isSelected ? "true" : undefined}
        aria-expanded={isOpen}
        style={{ paddingLeft: depth * 16 + 8 }}
        className={`flex w-full items-center gap-1.5 rounded-subtle py-1.5 pr-2 text-left text-sm transition-colors ${
          isSelected
            ? "bg-surface-interactive font-bold text-text-primary"
            : "text-text-secondary hover:bg-surface-interactive hover:text-text-primary"
        }`}
      >
        <ChevronRightIcon
          className={`size-3.5 shrink-0 transition-transform ${isOpen ? "rotate-90" : ""}`}
        />
        <FolderIcon className="size-4 shrink-0" />
        <span className="truncate">{node.name}</span>
      </button>
      {isOpen &&
        node.children.map((child) => (
          <DirRow
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

export function DirectoryTree({
  nodes,
  selectedPath,
  onSelect,
}: {
  nodes: PureDirectoryNode[];
  selectedPath: string;
  onSelect: (path: string) => void;
}) {
  const [expanded, setExpanded] = useState<Set<string>>(
    () => new Set(nodes.map((n) => n.path)),
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
    <nav aria-label="Directory tree" className="flex flex-col gap-0.5">
      {nodes.map((node) => (
        <DirRow
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
