import type { FileChangeType } from "@/lib/types";

const STYLES: Record<FileChangeType, string> = {
  added: "bg-accent/15 text-accent",
  modified: "bg-warning/15 text-warning",
  deleted: "bg-negative/15 text-negative",
};

const LABEL: Record<FileChangeType, string> = {
  added: "Added",
  modified: "Modified",
  deleted: "Deleted",
};

/**
 * Colour is never the only signal: every change type also carries a symbol
 * (added +, deleted −, modified ~), so the three stay distinguishable
 * without colour.
 */
export const CHANGE_SYMBOL: Record<FileChangeType, string> = {
  added: "+",
  modified: "~",
  deleted: "−",
};

export function ChangeTypeBadge({ type }: { type: FileChangeType }) {
  return (
    <span
      className={`inline-flex shrink-0 items-center gap-1 rounded-subtle px-1.5 py-0.5 text-[10.5px] font-semibold uppercase tracking-wide ${STYLES[type]}`}
    >
      <span aria-hidden="true" className="font-mono text-xs leading-none">
        {CHANGE_SYMBOL[type]}
      </span>
      {LABEL[type]}
    </span>
  );
}
