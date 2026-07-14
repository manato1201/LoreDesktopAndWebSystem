"use client";

import { useCallback, useRef, useState } from "react";

const STEP = 5;

export function ImageDiffSlider({ path }: { path: string }) {
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

  return (
    <div className="flex flex-col gap-2">
      <div
        ref={containerRef}
        className="relative aspect-video touch-none select-none overflow-hidden rounded-comfortable bg-surface-elevated"
        onPointerMove={handlePointerMove}
      >
        <div className="absolute inset-0 flex items-center justify-center bg-[repeating-linear-gradient(135deg,var(--color-surface-elevated),var(--color-surface-elevated)_10px,var(--color-surface-interactive)_10px,var(--color-surface-interactive)_20px)]">
          <span className="rounded-pill bg-bg-base/70 px-3 py-1 text-xs font-semibold text-text-secondary">
            After
          </span>
        </div>

        <div
          className="absolute inset-0 flex items-center justify-center bg-[repeating-linear-gradient(45deg,var(--color-surface),var(--color-surface)_10px,var(--color-surface-interactive)_10px,var(--color-surface-interactive)_20px)]"
          style={{ clipPath: `inset(0 ${100 - position}% 0 0)` }}
        >
          <span className="rounded-pill bg-bg-base/70 px-3 py-1 text-xs font-semibold text-text-secondary">
            Before
          </span>
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
        Drag or use arrow keys to compare {path.split("/").pop()}
      </p>
    </div>
  );
}
