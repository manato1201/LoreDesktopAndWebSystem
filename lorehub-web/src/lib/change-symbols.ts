import type { DiffLine, FileChangeType } from "./types";

/**
 * Colour is never the only signal for a change (IMPROVEMENT_DESIGN_2026-10.md
 * §3): every kind also carries a symbol, so added / modified / deleted stay
 * distinguishable with colour removed. Both the file-level badge and the
 * line-level diff viewer read from here so the two cannot drift apart.
 *
 * The minus is U+2212 (a real minus sign), not the ASCII hyphen.
 */
export const CHANGE_SYMBOL: Record<FileChangeType, string> = {
  added: "+",
  modified: "~",
  deleted: "−",
};

export const DIFF_LINE_SYMBOL: Record<DiffLine["type"], string> = {
  context: " ",
  add: CHANGE_SYMBOL.added,
  remove: CHANGE_SYMBOL.deleted,
};
