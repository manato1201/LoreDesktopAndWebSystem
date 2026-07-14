"use client";

import { useCallback, useRef, useState } from "react";
import { imageBeforeUrl, imageUrl } from "@/lib/api";
import type { FileChangeType } from "@/lib/types";

const STEP = 5;

export function ImageDiffSlider({
  repoSlug,
  path,
  changeType,
}: {
  repoSlug: string;
  path: string;
  changeType: FileChangeType;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const draggingRef = useRef(false);
  const [position, setPosition] = useState(50);

  const updateFromClientX = useCallback((clientX: number) => {
    const el = containerRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const pct = ((clientX - rect.left) / rect.width) * 100;
    setPosition(Math.min(100, Math.max(0, Math.round(pct))));
  }, []);

  const handlePointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    draggingRef.current = true;
    event.currentTarget.setPointerCapture(event.pointerId);
    updateFromClientX(event.clientX);
  };

  const handlePointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!draggingRef.current) return;
    updateFromClientX(event.clientX);
  };

  const handlePointerUp = (event: React.PointerEvent<HTMLDivElement>) => {
    draggingRef.current = false;
    event.currentTarget.releasePointerCapture(event.pointerId);
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "ArrowLeft") {
      setPosition((p) => Math.max(0, p - STEP));
    } else if (event.key === "ArrowRight") {
      setPosition((p) => Math.min(100, p + STEP));
    } else if (event.key === "Home") {
      setPosition(0);
    } else if (event.key === "End") {
      setPosition(100);
    } else {
      return;
    }
    event.preventDefault();
  };

  const fileName = path.split("/").pop();
  const after = imageUrl(repoSlug, path);

  // An added (or deleted) file has nothing to compare against, so show a
  // single image instead of a before/after slider.
  if (changeType !== "modified") {
    return (
      <div className="flex flex-col gap-2">
        {/* eslint-disable-next-line @next/next/no-img-element -- dynamic cross-origin SVG from lorehub-api */}
        <img
          src={after}
          alt=""
          className="aspect-video w-full rounded-comfortable object-cover"
        />
        <p className="text-center text-xs text-text-secondary">
          {changeType === "added" ? "Added" : "Removed"} · {fileName}
        </p>
      </div>
    );
  }

  const before = imageBeforeUrl(repoSlug, path);

  return (
    <div className="flex flex-col gap-2">
      <div
        ref={containerRef}
        className="relative aspect-video touch-none select-none overflow-hidden rounded-comfortable bg-surface-elevated"
        onPointerMove={handlePointerMove}
      >
        {/* eslint-disable-next-line @next/next/no-img-element -- dynamic cross-origin SVG from lorehub-api */}
        <img
          src={after}
          alt="After"
          className="pointer-events-none absolute inset-0 size-full object-cover"
        />

        <div
          className="pointer-events-none absolute inset-0"
          style={{ clipPath: `inset(0 ${100 - position}% 0 0)` }}
        >
          {/* eslint-disable-next-line @next/next/no-img-element -- dynamic cross-origin SVG from lorehub-api */}
          <img src={before} alt="Before" className="size-full object-cover" />
        </div>

        <div className="pointer-events-none absolute left-3 top-3 rounded-pill bg-bg-base/70 px-3 py-1 text-xs font-semibold text-text-secondary">
          Before
        </div>
        <div className="pointer-events-none absolute right-3 top-3 rounded-pill bg-bg-base/70 px-3 py-1 text-xs font-semibold text-text-secondary">
          After
        </div>

        <div
          role="slider"
          tabIndex={0}
          aria-label="Before/after comparison position"
          aria-valuenow={position}
          aria-valuemin={0}
          aria-valuemax={100}
          onPointerDown={handlePointerDown}
          onPointerUp={handlePointerUp}
          onKeyDown={handleKeyDown}
          className="absolute inset-y-0 flex w-6 -translate-x-1/2 cursor-ew-resize items-center justify-center outline-none"
          style={{ left: `${position}%` }}
        >
          <div className="h-full w-0.5 bg-text-primary" />
          <div className="absolute flex size-8 items-center justify-center rounded-full bg-text-primary text-bg-base shadow-heavy">
            ⇔
          </div>
        </div>
      </div>
      <p className="text-center text-xs text-text-secondary">
        Drag or use arrow keys to compare {fileName}
      </p>
    </div>
  );
}
