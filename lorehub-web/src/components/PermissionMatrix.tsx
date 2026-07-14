import type { AccessEntry, PermissionLevel } from "@/lib/types";

const LEVELS: PermissionLevel[] = ["read", "write", "lock"];
const LEVEL_LABEL: Record<PermissionLevel, string> = {
  read: "Read",
  write: "Write",
  lock: "Lock",
};

export function PermissionMatrix({
  path,
  entries,
  onTogglePermission,
}: {
  path: string;
  entries: AccessEntry[];
  onTogglePermission: (principal: string, level: PermissionLevel) => void;
}) {
  if (entries.length === 0) {
    return (
      <p className="rounded-comfortable bg-surface p-4 text-sm text-text-secondary">
        No overrides on <code className="text-text-primary">{path}</code> —
        inherits permissions from its parent directory.
      </p>
    );
  }

  return (
    <div className="overflow-hidden rounded-comfortable bg-surface">
      <table className="w-full border-collapse text-sm">
        <thead>
          <tr className="border-b border-border/40 text-left text-xs text-text-secondary">
            <th className="px-4 py-3 font-semibold">Principal</th>
            {LEVELS.map((level) => (
              <th key={level} className="px-4 py-3 text-center font-semibold">
                {LEVEL_LABEL[level]}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {entries.map((entry) => (
            <tr
              key={entry.principal}
              className="border-b border-border/40 last:border-0"
            >
              <td className="px-4 py-3">
                <span className="font-bold text-text-primary">
                  {entry.principal}
                </span>
                <span className="ml-2 text-xs uppercase tracking-wide text-text-secondary">
                  {entry.principalType}
                </span>
              </td>
              {LEVELS.map((level) => {
                const granted = entry.permissions.includes(level);
                return (
                  <td key={level} className="px-4 py-3 text-center">
                    <button
                      type="button"
                      onClick={() => onTogglePermission(entry.principal, level)}
                      aria-pressed={granted}
                      aria-label={`${LEVEL_LABEL[level]} for ${entry.principal} on ${path}`}
                      className={`size-6 rounded-standard transition-colors ${
                        granted
                          ? "bg-accent"
                          : "bg-surface-interactive hover:bg-surface-elevated"
                      }`}
                    />
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
