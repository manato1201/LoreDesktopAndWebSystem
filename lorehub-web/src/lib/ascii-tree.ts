/**
 * A tree drawn in text: branches fork like a repository's branches, and the
 * trunk stands on a cross-section whose rings stand for history
 * (ASCII_MOTIF_MAP_2026-10-27.md: Lore's motif is "tree, rings, forking").
 *
 * This is the "grow" idea reduced to its still image: a recursive branch
 * function run once, with a fixed seed, so the picture is identical on every
 * render (and for `prefers-reduced-motion`, which is the only mode there is
 * for now — animation can be layered on later by replaying the same steps).
 */

export const TREE_COLS = 51;
export const TREE_ROWS = 30;

/** Fixed: the same tree every time, on the server and in the browser. */
const SEED = 20261027;

/** Small deterministic random generator (mulberry32). */
function createRandom(seed: number): () => number {
  let state = seed | 0;
  return () => {
    state = (state + 0x6d2b79f5) | 0;
    let t = Math.imul(state ^ (state >>> 15), 1 | state);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** The tree trunk's cross-section: end grain with growth rings. */
const RINGS = [
  "      .-''''''''-.      ",
  "    .'  .-''''-.  '.    ",
  "   /  .'  .--.  '.  \\   ",
  "  |  |   ( () )   |  |  ",
  "   \\  '.  '--'  .'  /   ",
  "    '.  '-....-'  .'    ",
  "      '-........-'      ",
];

const LEAVES = [".", ":", "*", "o", "@"];

type Grid = string[][];

function blank(): Grid {
  return Array.from({ length: TREE_ROWS }, () =>
    Array.from({ length: TREE_COLS }, () => " "),
  );
}

function plot(grid: Grid, x: number, y: number, ch: string, overwrite = true) {
  const col = Math.round(x);
  const row = Math.round(y);
  if (col < 0 || col >= TREE_COLS || row < 0 || row >= TREE_ROWS) return;
  if (!overwrite && grid[row][col] !== " ") return;
  grid[row][col] = ch;
}

/**
 * The character that best draws a stroke heading (dx, dy) on a terminal grid.
 * `dy` is "up is negative", as on screen.
 */
export function strokeChar(dx: number, dy: number): string {
  const ax = Math.abs(dx);
  const ay = Math.abs(dy);
  if (ax < ay * 0.35) return "|";
  if (ay < ax * 0.35) return "-";
  // Moving right and up (dx > 0, dy < 0) reads as "/"; right and down as "\".
  return dx * dy < 0 ? "/" : "\\";
}

/**
 * Draws one branch from (x, y) leaning `angle` radians off vertical, then
 * forks into two shorter, thinner branches until `depth` runs out, where it
 * ends in a puff of leaves.
 */
function grow(
  grid: Grid,
  random: () => number,
  x: number,
  y: number,
  angle: number,
  length: number,
  depth: number,
) {
  // A cell is about twice as tall as it is wide, so horizontal travel is
  // doubled to keep the angle looking right.
  const dx = Math.sin(angle) * 2;
  const dy = -Math.cos(angle);
  const steps = Math.max(2, Math.round(length));

  let cx = x;
  let cy = y;
  for (let i = 0; i < steps; i++) {
    plot(grid, cx, cy, strokeChar(dx, dy));
    cx += dx;
    cy += dy;
  }

  if (depth === 0) {
    puff(grid, random, cx, cy);
    return;
  }

  const spread = 0.5 + random() * 0.25;
  grow(grid, random, cx, cy, angle - spread, length * 0.8, depth - 1);
  grow(grid, random, cx, cy, angle + spread, length * 0.8, depth - 1);
  // Every other fork also sends a short shoot straight on, so the crown fills in.
  if (depth % 2 === 1) {
    grow(
      grid,
      random,
      cx,
      cy,
      angle + (random() - 0.5) * 0.2,
      length * 0.6,
      depth - 1,
    );
  }
}

/** A cluster of leaves around (x, y); never overwrites a branch. */
function puff(grid: Grid, random: () => number, x: number, y: number) {
  for (let i = 0; i < 9; i++) {
    const ox = (random() - 0.5) * 8;
    const oy = (random() - 0.5) * 4;
    const ch = LEAVES[Math.floor(random() * LEAVES.length)];
    plot(grid, x + ox, y + oy, ch, false);
  }
}

export function buildAsciiTree(): string[] {
  const grid = blank();
  const random = createRandom(SEED);

  const ringsTop = TREE_ROWS - RINGS.length;
  const trunkBottom = ringsTop - 1;
  const centre = Math.floor(TREE_COLS / 2);

  // Rings first, then the trunk standing on them.
  RINGS.forEach((line, i) => {
    const left = centre - Math.floor(line.length / 2);
    [...line].forEach((ch, j) => {
      if (ch !== " ") plot(grid, left + j, ringsTop + i, ch);
    });
  });

  const trunkHeight = 6;
  for (let i = 0; i < trunkHeight; i++) {
    plot(grid, centre - 1, trunkBottom - i, "|");
    plot(grid, centre + 1, trunkBottom - i, "|");
  }

  grow(grid, random, centre, trunkBottom - trunkHeight, 0, 3, 4);

  return grid.map((row) => row.join("").replace(/\s+$/, ""));
}

/** Built once, at module load. */
export const ASCII_TREE: string = buildAsciiTree().join("\n");
