import type { FileKind, TreeNode } from "./types";

/** Hashes are shown as their first 7 characters everywhere (the full value is what gets copied). */
export const SHORT_HASH_LENGTH = 7;

export function shortHash(hash: string): string {
  return hash.slice(0, SHORT_HASH_LENGTH);
}

/**
 * Exact, locale-independent time for tooltips and screen readers, e.g.
 * "2026-10-25 09:41 UTC". UTC on purpose: the same commit reads the same for
 * every viewer, and the server is the source of truth for the instant.
 */
export function formatAbsoluteTime(unixSeconds: number): string {
  const iso = new Date(unixSeconds * 1000).toISOString();
  return `${iso.slice(0, 10)} ${iso.slice(11, 16)} UTC`;
}

const UNITS = ["B", "KB", "MB", "GB", "TB"] as const;

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  // Round first, then check the unit: 1_048_575 B is 1023.999 KB, which
  // rounds to "1024.0 KB" — it should read "1.0 MB".
  let rounded = Math.round(value * 10) / 10;
  if (rounded >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
    rounded = Math.round(value * 10) / 10;
  }
  return `${rounded.toFixed(1)} ${UNITS[unit]}`;
}

export type StorageBucket = "audio" | "model3d" | "image" | "other";

export const STORAGE_BUCKET_LABEL: Record<StorageBucket, string> = {
  audio: "Audio",
  model3d: "3D",
  image: "Textures & images",
  other: "Other",
};

const BUCKET_OF_KIND: Record<FileKind, StorageBucket> = {
  audio: "audio",
  model3d: "model3d",
  image: "image",
  text: "other",
  binary: "other",
};

export type StorageBreakdown = {
  totalBytes: number;
  /** Fixed display order; buckets with no bytes are kept (value 0) so the legend stays stable. */
  buckets: { bucket: StorageBucket; bytes: number }[];
};

/**
 * Sums the file sizes of a tree by kind. The total is the sum of the same
 * numbers the buckets are made of, so the stacked bar can never disagree with
 * the figure printed next to it.
 */
export function summarizeStorage(tree: TreeNode[]): StorageBreakdown {
  const sums: Record<StorageBucket, number> = {
    audio: 0,
    model3d: 0,
    image: 0,
    other: 0,
  };

  const walk = (nodes: TreeNode[]) => {
    for (const node of nodes) {
      if (node.kind === "directory") {
        walk(node.children);
      } else {
        sums[BUCKET_OF_KIND[node.kind]] += node.sizeBytes;
      }
    }
  };
  walk(tree);

  const order: StorageBucket[] = ["audio", "model3d", "image", "other"];
  const buckets = order.map((bucket) => ({ bucket, bytes: sums[bucket] }));
  return {
    totalBytes: buckets.reduce((acc, b) => acc + b.bytes, 0),
    buckets,
  };
}
