import { describe, expect, it } from "vitest";
import { CHANGE_SYMBOL, DIFF_LINE_SYMBOL } from "./change-symbols";

describe("change symbols", () => {
  it("gives every change type its own symbol, so colour is never the only signal", () => {
    const symbols = Object.values(CHANGE_SYMBOL);
    expect(new Set(symbols).size).toBe(symbols.length);
  });

  it("uses a real minus sign for deletions, not an ASCII hyphen", () => {
    expect(CHANGE_SYMBOL.deleted).toBe("−");
    expect(CHANGE_SYMBOL.deleted).not.toBe("-");
  });

  it("keeps the line-level diff symbols in step with the file-level ones", () => {
    expect(DIFF_LINE_SYMBOL.add).toBe(CHANGE_SYMBOL.added);
    expect(DIFF_LINE_SYMBOL.remove).toBe(CHANGE_SYMBOL.deleted);
  });
});
