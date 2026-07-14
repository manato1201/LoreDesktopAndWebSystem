"use client";

import { useEffect, useState } from "react";
import { imageUrl } from "@/lib/api";
import type { FileKind } from "@/lib/types";
import { AudioIcon } from "./icons";
import { ModelViewer } from "./ModelViewer";

const PREVIEW_ICON: Partial<Record<FileKind, typeof AudioIcon>> = {
  audio: AudioIcon,
};

const PREVIEW_LABEL: Partial<Record<FileKind, string>> = {
  audio: "Waveform preview",
  binary: "No preview available for this file type",
};

/**
 * Simulates the chunk-based streaming viewer from ARCHITECTURE.md §4.3: a
 * short progress phase stands in for the backend streaming chunks to the
 * client before the preview is shown. `model3d` renders a real Three.js
 * canvas (see ModelViewer) and `image` renders a real `<img>` from
 * lorehub-api; other kinds fall back to a static placeholder. The parent
 * must pass `key={path}` so a new file remounts this component (and
 * therefore resets `progress`) instead of reusing state across files.
 */
export function StreamingPreview({
  kind,
  repoSlug,
  path,
}: {
  kind: FileKind;
  repoSlug: string;
  path: string;
}) {
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    let frame: number;
    const start = performance.now();
    const durationMs = 700;

    const step = (now: number) => {
      const pct = Math.min(100, Math.round(((now - start) / durationMs) * 100));
      setProgress(pct);
      if (pct < 100) {
        frame = requestAnimationFrame(step);
      }
    };

    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  }, []);

  const Icon = PREVIEW_ICON[kind];
  const isLoaded = progress >= 100;

  if (isLoaded && kind === "model3d") {
    return <ModelViewer className="aspect-video" />;
  }

  if (isLoaded && kind === "image") {
    return (
      // eslint-disable-next-line @next/next/no-img-element -- dynamic cross-origin SVG from lorehub-api, not a static asset next/image can optimize
      <img
        src={imageUrl(repoSlug, path)}
        alt=""
        className="aspect-video w-full rounded-comfortable object-cover"
      />
    );
  }

  return (
    <div className="flex aspect-video flex-col items-center justify-center gap-3 rounded-comfortable bg-surface-elevated">
      {isLoaded ? (
        <>
          {Icon && <Icon className="size-10 text-text-secondary" />}
          <p className="max-w-xs text-center text-xs text-text-secondary">
            {PREVIEW_LABEL[kind]}
          </p>
        </>
      ) : (
        <div className="flex w-48 flex-col items-center gap-2">
          <p className="text-xs text-text-secondary">Streaming chunks…</p>
          <div
            role="progressbar"
            aria-valuenow={progress}
            aria-valuemin={0}
            aria-valuemax={100}
            className="h-1.5 w-full overflow-hidden rounded-pill bg-surface"
          >
            <div
              className="h-full rounded-pill bg-accent"
              style={{ width: `${progress}%` }}
            />
          </div>
        </div>
      )}
    </div>
  );
}
