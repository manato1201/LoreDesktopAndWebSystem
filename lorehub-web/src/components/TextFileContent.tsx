"use client";

import { useEffect, useState } from "react";
import { getFileContent } from "@/lib/api";

/**
 * Keyed by `path` in the parent so a new file remounts this component
 * instead of reusing state across files (see StreamingPreview for the
 * same pattern).
 */
export function TextFileContent({
  repoSlug,
  path,
}: {
  repoSlug: string;
  path: string;
}) {
  const [content, setContent] = useState<string | null>(null);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getFileContent(repoSlug, path).then((result) => {
      if (!cancelled) {
        setContent(result);
        setLoaded(true);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [repoSlug, path]);

  return (
    <pre className="overflow-x-auto rounded-comfortable bg-surface-elevated p-4 text-xs leading-relaxed text-text-secondary">
      <code>
        {!loaded ? "Loading…" : (content ?? "Preview not available.")}
      </code>
    </pre>
  );
}
