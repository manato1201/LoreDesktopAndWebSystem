"use client";

import { useState } from "react";
import { ModelViewer } from "./ModelViewer";

const VARIANTS = ["Before", "After"] as const;

export function Model3DDiffToggle({ path }: { path: string }) {
  const [variant, setVariant] = useState<(typeof VARIANTS)[number]>("After");

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between gap-2">
        <div
          role="tablist"
          aria-label="3D model comparison"
          className="flex w-fit gap-1 rounded-pill bg-surface-interactive p-1"
        >
          {VARIANTS.map((v) => (
            <button
              key={v}
              type="button"
              role="tab"
              aria-selected={variant === v}
              onClick={() => setVariant(v)}
              className={`rounded-pill px-3 py-1 text-xs font-bold transition-colors ${
                variant === v
                  ? "bg-accent text-bg-base"
                  : "text-text-secondary hover:text-text-primary"
              }`}
            >
              {v}
            </button>
          ))}
        </div>
        <p className="text-xs text-text-secondary">
          Drag to orbit · scroll to zoom
        </p>
      </div>

      <ModelViewer
        key={variant}
        variant={variant === "Before" ? "before" : "after"}
        className="aspect-video"
      />
      <p className="text-xs text-text-secondary">
        {variant} — {path.split("/").pop()}
      </p>
    </div>
  );
}
