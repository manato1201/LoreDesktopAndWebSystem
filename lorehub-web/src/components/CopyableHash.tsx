"use client";

import { shortHash } from "@/lib/format";
import { useToast } from "./Toast";

/**
 * A commit hash as its first 7 characters in a monospace face. Clicking
 * copies the full hash and confirms with a toast.
 */
export function CopyableHash({
  hash,
  className = "",
}: {
  hash: string;
  className?: string;
}) {
  const { showToast } = useToast();

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(hash);
      showToast("Copied");
    } catch {
      showToast("Could not copy. Select the hash and copy it manually.");
    }
  };

  return (
    <button
      type="button"
      onClick={copy}
      title={`Copy full hash: ${hash}`}
      aria-label={`Copy commit hash ${hash}`}
      className={`rounded-subtle bg-surface-interactive px-2 py-1 font-mono text-xs text-text-secondary transition-colors hover:text-text-primary ${className}`}
    >
      {shortHash(hash)}
    </button>
  );
}
