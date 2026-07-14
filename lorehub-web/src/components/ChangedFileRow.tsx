import type { FileChange } from "@/lib/types";
import { ChangeTypeBadge } from "./ChangeTypeBadge";
import { FILE_KIND_ICON, inferFileKind } from "./icons";

export function ChangedFileRow({ change }: { change: FileChange }) {
  const Icon = FILE_KIND_ICON[inferFileKind(change.path)];

  return (
    <div className="flex items-center gap-3 rounded-standard px-3 py-2 text-sm">
      <Icon className="size-4 shrink-0 text-text-secondary" />
      <span className="min-w-0 flex-1 truncate text-text-primary">
        {change.path}
      </span>
      <span className="shrink-0 text-xs text-text-secondary">
        {change.sizeDeltaLabel}
      </span>
      <ChangeTypeBadge type={change.changeType} />
    </div>
  );
}
