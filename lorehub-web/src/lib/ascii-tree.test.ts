import { describe, expect, it } from "vitest";
import {
  ASCII_TREE,
  buildAsciiTree,
  strokeChar,
  TREE_COLS,
  TREE_ROWS,
} from "./ascii-tree";

describe("buildAsciiTree", () => {
  const rows = buildAsciiTree();

  it("draws the same tree every time (fixed seed)", () => {
    expect(buildAsciiTree()).toEqual(rows);
    expect(ASCII_TREE).toBe(rows.join("\n"));
  });

  it("fits its grid", () => {
    expect(rows).toHaveLength(TREE_ROWS);
    for (const row of rows) {
      expect(row.length).toBeLessThanOrEqual(TREE_COLS);
    }
  });

  it("is plain printable ASCII, so any font and any terminal can show it", () => {
    for (const row of rows) {
      expect(row).toMatch(/^[\x20-\x7e]*$/);
    }
  });

  it("has a canopy, a trunk and rings, from top to bottom", () => {
    const filledRows = rows.map((r) => r.trim().length > 0);
    // Something is drawn in the upper half (the canopy) ...
    expect(filledRows.slice(0, TREE_ROWS / 2).some(Boolean)).toBe(true);
    // ... the last row is the bottom of the rings ...
    expect(rows[TREE_ROWS - 1]).toContain("'-");
    // ... and the centre of the rings is there.
    expect(rows.join("\n")).toContain("( () )");
  });

  it("never touches the top edge (the canopy is not clipped)", () => {
    expect(rows[0].trim()).toBe("");
  });
});

describe("strokeChar", () => {
  it("picks the stroke that matches the heading", () => {
    expect(strokeChar(0, -1)).toBe("|");
    expect(strokeChar(2, 0)).toBe("-");
    expect(strokeChar(2, -1)).toBe("/"); // right and up
    expect(strokeChar(-2, -1)).toBe("\\"); // left and up
    expect(strokeChar(2, 1)).toBe("\\"); // right and down
  });
});
