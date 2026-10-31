import { ASCII_TREE } from "@/lib/ascii-tree";

/**
 * Decorative: shown only where there is nothing else to look at (an empty
 * repository, a missing page). A still picture, so it needs no
 * `prefers-reduced-motion` handling. Screen readers get one sentence instead
 * of fifty rows of punctuation.
 */
export function AsciiTree({ className = "" }: { className?: string }) {
  return (
    <div
      role="img"
      aria-label="A tree drawn in text. Its forking branches stand for repository branches, and the rings at its base for history."
      className={className}
    >
      <pre
        aria-hidden="true"
        className="m-0 select-none overflow-hidden font-mono text-[9px] leading-[1.1] text-accent"
      >
        {ASCII_TREE}
      </pre>
    </div>
  );
}
