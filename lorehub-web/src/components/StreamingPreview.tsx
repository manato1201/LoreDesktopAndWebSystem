"use client";

import { useEffect, useState } from "react";
import type { FileKind } from "@/lib/types";
import { AudioIcon, ImageIcon, Model3DIcon } from "./icons";

const PREVIEW_ICON: Partial<Record<FileKind, typeof ImageIcon>> = {
  image: ImageIcon,
  model3d: Model3DIcon,
  audio: AudioIcon,
};

const PREVIEW_LABEL: Partial<Record<FileKind, string>> = {
  image: "Image preview",
  model3d: "3D viewport — Three.js renderer mounts here",
  audio: "Waveform preview",
  binary: "No preview available for this file type",
};

/**
 * Simulates the chunk-based streaming viewer from ARCHITECTURE.md §4.3:
 * a short progress phase stands in for the backend streaming chunks to the
 * client before the (mocked) preview is shown.
 */
/**
 * The parent must pass `key={path}` so a new file remounts this component
 * (and therefore resets `progress`) instead of reusing state across files.
 */
export function StreamingPreview({ kind }: { kind: FileKind }) {
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
