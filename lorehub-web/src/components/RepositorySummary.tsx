import {
  formatBytes,
  STORAGE_BUCKET_LABEL,
  summarizeStorage,
  type StorageBucket,
} from "@/lib/format";
import type { TreeNode } from "@/lib/types";

const BUCKET_COLOR: Record<StorageBucket, string> = {
  audio: "var(--color-accent)",
  model3d: "var(--color-announcement)",
  image: "var(--color-warning)",
  other: "var(--color-border-light)",
};

export function RepositorySummary({
  tree,
  commitCount,
  branchCount,
  pullRequestCount,
}: {
  tree: TreeNode[];
  commitCount: number;
  branchCount: number;
  pullRequestCount: number;
}) {
  const storage = summarizeStorage(tree);
  const nonEmpty = storage.buckets.filter((b) => b.bytes > 0);

  return (
    <aside
      aria-label="Repository summary"
      className="flex flex-col gap-5 rounded-comfortable bg-surface p-5"
    >
      <dl className="grid grid-cols-3 gap-2 text-center xl:grid-cols-1 xl:text-left">
        <Stat label="Commits" value={commitCount} />
        <Stat label="Branches" value={branchCount} />
        <Stat label="Pull requests" value={pullRequestCount} />
      </dl>

      <div className="flex flex-col gap-2">
        <div className="flex items-baseline justify-between">
          <h2 className="text-xs font-semibold text-text-secondary">Size</h2>
          <p className="font-mono text-sm font-bold text-text-primary">
            {formatBytes(storage.totalBytes)}
          </p>
        </div>

        {storage.totalBytes > 0 ? (
          <>
            <div
              role="img"
              aria-label={`Size by type: ${nonEmpty
                .map(
                  (b) =>
                    `${STORAGE_BUCKET_LABEL[b.bucket]} ${formatBytes(b.bytes)}`,
                )
                .join(", ")}`}
              className="flex h-2 w-full overflow-hidden rounded-pill bg-surface-interactive"
            >
              {nonEmpty.map((b) => (
                <div
                  key={b.bucket}
                  style={{
                    width: `${(b.bytes / storage.totalBytes) * 100}%`,
                    backgroundColor: BUCKET_COLOR[b.bucket],
                  }}
                />
              ))}
            </div>
            <ul className="flex flex-col gap-1 text-xs">
              {nonEmpty.map((b) => (
                <li key={b.bucket} className="flex items-center gap-2">
                  <span
                    aria-hidden="true"
                    className="size-2 shrink-0 rounded-full"
                    style={{ backgroundColor: BUCKET_COLOR[b.bucket] }}
                  />
                  <span className="flex-1 text-text-secondary">
                    {STORAGE_BUCKET_LABEL[b.bucket]}
                  </span>
                  <span className="font-mono text-text-primary">
                    {formatBytes(b.bytes)}
                  </span>
                </li>
              ))}
            </ul>
          </>
        ) : (
          <p className="text-xs text-text-secondary">No files yet.</p>
        )}
      </div>
    </aside>
  );
}

function Stat({ label, value }: { label: string; value: number }) {
  return (
    <div className="flex flex-col gap-0.5 xl:flex-row xl:items-baseline xl:justify-between">
      <dt className="text-xs text-text-secondary">{label}</dt>
      <dd className="font-mono text-sm font-bold text-text-primary">
        {value.toLocaleString("en-US")}
      </dd>
    </div>
  );
}
