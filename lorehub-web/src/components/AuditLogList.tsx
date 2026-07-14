import type { AuditLogEntry } from "@/lib/types";

export function AuditLogList({ entries }: { entries: AuditLogEntry[] }) {
  return (
    <ul className="flex flex-col divide-y divide-border/40 rounded-comfortable bg-surface px-4">
      {entries.map((entry) => (
        <li key={entry.id} className="flex items-center gap-2 py-3 text-sm">
          <span className="font-bold text-text-primary">{entry.actor}</span>
          <span className="text-text-secondary">{entry.action}</span>
          <code className="truncate text-text-primary">{entry.target}</code>
          <span className="ml-auto shrink-0 text-xs text-text-secondary">
            {entry.timestamp}
          </span>
        </li>
      ))}
    </ul>
  );
}
