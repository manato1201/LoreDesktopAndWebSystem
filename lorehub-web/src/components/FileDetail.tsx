import type { FileKind, FileTreeNode } from "@/lib/types";
import { LockIcon, UnlockIcon } from "./icons";
import { StreamingPreview } from "./StreamingPreview";
import { TextFileContent } from "./TextFileContent";

const KIND_LABEL: Record<FileKind, string> = {
  text: "Text",
  image: "Image",
  model3d: "3D Model",
  audio: "Audio",
  binary: "Binary",
};

export function FileDetail({
  repoSlug,
  node,
  onToggleLock,
}: {
  repoSlug: string;
  node: FileTreeNode;
  onToggleLock: () => void;
}) {
  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <h2 className="break-all text-lg font-bold text-text-primary">
            {node.name}
          </h2>
          <p className="text-xs text-text-secondary">
            {KIND_LABEL[node.kind]} · {node.sizeLabel} · Updated{" "}
            {node.updatedAt}
          </p>
        </div>
        <button
          type="button"
          onClick={onToggleLock}
          className={`flex shrink-0 items-center gap-2 rounded-pill px-4 py-2 text-xs font-bold uppercase tracking-wide transition-colors ${
            node.lockedBy
              ? "bg-warning/15 text-warning hover:bg-warning/25"
              : "bg-surface-interactive text-text-primary hover:bg-surface-elevated"
          }`}
        >
          {node.lockedBy ? (
            <UnlockIcon className="size-4" />
          ) : (
            <LockIcon className="size-4" />
          )}
          {node.lockedBy ? "Unlock" : "Lock"}
        </button>
      </div>

      {node.lockedBy && (
        <p className="flex items-center gap-2 text-xs text-warning">
          <LockIcon className="size-3.5" />
          Locked by {node.lockedBy}
        </p>
      )}

      {node.kind === "text" ? (
        <TextFileContent key={node.path} repoSlug={repoSlug} path={node.path} />
      ) : (
        <StreamingPreview
          key={node.path}
          kind={node.kind}
          repoSlug={repoSlug}
          path={node.path}
        />
      )}
    </div>
  );
}
