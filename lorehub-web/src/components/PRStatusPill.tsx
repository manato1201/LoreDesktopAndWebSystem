import type { PRStatus } from "@/lib/types";

const STYLES: Record<PRStatus, string> = {
  open: "bg-accent/15 text-accent",
  merged: "bg-announcement/15 text-announcement",
  closed: "bg-negative/15 text-negative",
};

const LABEL: Record<PRStatus, string> = {
  open: "Open",
  merged: "Merged",
  closed: "Closed",
};

export function PRStatusPill({ status }: { status: PRStatus }) {
  return (
    <span
      className={`shrink-0 rounded-pill px-2.5 py-1 text-[11px] font-bold uppercase tracking-wide ${STYLES[status]}`}
    >
      {LABEL[status]}
    </span>
  );
}
