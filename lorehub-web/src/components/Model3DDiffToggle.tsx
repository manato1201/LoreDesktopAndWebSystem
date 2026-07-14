"use client";

import { useState } from "react";
import { Model3DIcon } from "./icons";

const VARIANTS = ["Before", "After"] as const;

export function Model3DDiffToggle({ path }: { path: string }) {
  const [variant, setVariant] = useState<(typeof VARIANTS)[number]>("After");

  return (
    <div className="flex flex-col gap-2">
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

      <div className="flex aspect-video flex-col items-center justify-center gap-2 rounded-comfortable bg-[repeating-linear-gradient(90deg,var(--color-surface-elevated),var(--color-surface-elevated)_18px,var(--color-surface)_18px,var(--color-surface)_36px)]">
        <Model3DIcon className="size-10 text-text-secondary" />
        <p className="text-xs text-text-secondary">
          {variant} — Three.js viewport for {path.split("/").pop()} mounts here
        </p>
      </div>
    </div>
  );
}
