import type { FileChangeType } from "@/lib/types";

const STYLES: Record<FileChangeType, string> = {
  added: "bg-accent/15 text-accent",
  modified: "bg-announcement/15 text-announcement",
  deleted: "bg-negative/15 text-negative",
};

const LABEL: Record<FileChangeType, string> = {
  added: "Added",
  modified: "Modified",
  deleted: "Deleted",
};

export function ChangeTypeBadge({ type }: { type: FileChangeType }) {
  return (
    <span
      className={`shrink-0 rounded-subtle px-1.5 py-0.5 text-[10.5px] font-semibold uppercase tracking-wide ${STYLES[type]}`}
    >
      {LABEL[type]}
    </span>
  );
}
