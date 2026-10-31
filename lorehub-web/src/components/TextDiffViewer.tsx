import { DIFF_LINE_SYMBOL } from "@/lib/change-symbols";
import type { DiffLine } from "@/lib/types";

const LINE_STYLE: Record<DiffLine["type"], string> = {
  context: "text-text-secondary",
  add: "bg-accent/10 text-text-primary",
  remove: "bg-negative/10 text-text-primary",
};

export function TextDiffViewer({ lines }: { lines: DiffLine[] }) {
  return (
    <pre className="overflow-x-auto rounded-comfortable bg-surface-elevated p-4 text-xs leading-relaxed">
      <code>
        {lines.map((line, index) => (
          <div key={index} className={`-mx-4 px-4 ${LINE_STYLE[line.type]}`}>
            <span className="select-none text-text-secondary/60">
              {DIFF_LINE_SYMBOL[line.type]}
            </span>{" "}
            {line.text}
          </div>
        ))}
      </code>
    </pre>
  );
}
