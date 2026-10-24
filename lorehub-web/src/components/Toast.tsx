"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";

/** How long a toast stays on screen (IMPROVEMENT_DESIGN_2026-10.md §3). */
export const TOAST_DURATION_MS = 2000;

type ToastContextValue = {
  showToast: (message: string) => void;
};

const ToastContext = createContext<ToastContextValue | null>(null);

export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext);
  if (!ctx) {
    throw new Error("useToast must be used inside <ToastProvider>");
  }
  return ctx;
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const [message, setMessage] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const showToast = useCallback((next: string) => {
    if (timer.current) clearTimeout(timer.current);
    setMessage(next);
    timer.current = setTimeout(() => setMessage(null), TOAST_DURATION_MS);
  }, []);

  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
    },
    [],
  );

  return (
    <ToastContext.Provider value={{ showToast }}>
      {children}
      {/* The live region stays mounted so screen readers announce text changes. */}
      <div
        role="status"
        aria-live="polite"
        // QUALITY_STANDARDS.md §1: z-index in steps of 10. A toast sits above modals (1000).
        style={{ zIndex: 1100 }}
        className="pointer-events-none fixed bottom-6 left-1/2 -translate-x-1/2"
      >
        {message && (
          <p className="rounded-pill bg-surface-elevated px-4 py-2 text-sm font-semibold text-text-primary shadow-[var(--shadow-heavy)]">
            {message}
          </p>
        )}
      </div>
    </ToastContext.Provider>
  );
}
