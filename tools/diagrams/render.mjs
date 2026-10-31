// A tiny diagram renderer: a plain JS object in, an SVG string out.
//
// Why not Mermaid for everything? Mermaid decides the layout for you, which is
// fast but hard to make readable for a system with groups, numbered steps and
// two kinds of arrow. Here the author places every node, so the picture says
// what the author means. The cost is that coordinates are hand-written; the
// validator below catches the usual mistakes (overlaps, nodes off the canvas,
// edges to nodes that do not exist).
//
// Vocabulary (see specs/ for real examples):
//   groups : labelled frames around related nodes          { id, label, x, y, w, h, tone, dashed }
//   nodes  : tile (icon + name + caption), box, diamond, pill
//   edges  : arrows between nodes, optionally bent via points, labelled,
//            dashed, and numbered with a step badge
//   labels : free text
//
// Coordinates are the node's centre. Edges leave a node from the side that
// faces the next point, so route a bent edge through `via` points that share
// an x or a y with the node centre to get clean right angles.

import { iconSvg } from "./icons.mjs";

const TILE = 76;
const FONT_SANS = `Inter, 'Segoe UI', 'Yu Gothic UI', 'Yu Gothic', Meiryo, 'Hiragino Sans', sans-serif`;
const FONT_MONO = `'JetBrains Mono', Consolas, 'SFMono-Regular', Menlo, monospace`;

/** Tone name -> { line, fill, edge } colours. */
const LIGHT = {
  bg: "#f7f8f5",
  surface: "#ffffff",
  border: "#cdd3ca",
  ink: "#14171a",
  ink2: "#5a615c",
  tones: {
    green: { line: "#14a34e", fill: "#eef7f1", edge: "#8fcea8" },
    blue: { line: "#2c6fd1", fill: "#eef3fc", edge: "#9bbcec" },
    orange: { line: "#c2691a", fill: "#fbf3ea", edge: "#e3b27f" },
    gray: { line: "#5f6661", fill: "#f1f3ef", edge: "#c5cbc1" },
    red: { line: "#c23b4a", fill: "#fcefef", edge: "#e3a1a9" },
  },
};

/**
 * The same roles, but pointing at the CSS variables the HTML documentation
 * defines for its light and dark themes, so an inline diagram follows the
 * reader's theme. (Fallbacks keep it readable if a variable is missing.)
 */
const PAGE = {
  bg: "var(--bg-surface, #ffffff)",
  surface: "var(--bg-interactive, #eef0ec)",
  border: "var(--border, #cdd3ca)",
  ink: "var(--text-primary, #14171a)",
  ink2: "var(--text-secondary, #5a615c)",
  tones: {
    green: tone("--accent", "#14a34e"),
    blue: tone("--announcement", "#2c6fd1"),
    orange: tone("--warning", "#c2691a"),
    gray: {
      line: "var(--text-secondary, #5f6661)",
      fill: "var(--bg-interactive, #f1f3ef)",
      edge: "var(--border, #c5cbc1)",
    },
    red: tone("--negative", "#c23b4a"),
  },
};

function tone(variable, fallback) {
  return {
    line: `var(${variable}, ${fallback})`,
    fill: `color-mix(in srgb, var(${variable}, ${fallback}) 9%, var(--bg-surface, #fff))`,
    edge: `color-mix(in srgb, var(${variable}, ${fallback}) 45%, var(--bg-surface, #fff))`,
  };
}

const TONE_NAMES = Object.keys(LIGHT.tones);

// ---------------------------------------------------------------------------
// small helpers

const num = (v) => String(Math.round(v * 10) / 10);

const esc = (s) =>
  String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

/** Rough text width: CJK glyphs are one em wide, Latin about 0.56 (0.6 mono). */
export function textWidth(text, size, mono = false) {
  let w = 0;
  for (const ch of String(text)) {
    const code = ch.codePointAt(0);
    if (code >= 0x2e80) w += size;
    else w += size * (mono ? 0.6 : 0.56);
  }
  return w;
}

function assertTone(name, where) {
  if (name !== undefined && !TONE_NAMES.includes(name)) {
    throw new Error(`${where}: unknown tone "${name}" (known: ${TONE_NAMES.join(", ")})`);
  }
}

// ---------------------------------------------------------------------------
// node geometry

/** Fills in width/height and the inside-test for a node. */
function shapeOf(node) {
  const kind = node.kind ?? "tile";
  const lines = String(node.label ?? "").split("\n");
  const longest = Math.max(...lines.map((l) => textWidth(l, 14)));
  let w;
  let h;
  if (kind === "tile") {
    w = TILE;
    h = TILE;
  } else if (kind === "diamond") {
    w = Math.max(150, longest + 70);
    h = 30 + lines.length * 20 + 36;
  } else if (kind === "pill") {
    w = Math.max(110, longest + 48);
    h = 40;
  } else if (kind === "box") {
    w = Math.max(120, longest + 36);
    h = 18 + lines.length * 20 + (node.caption ? 18 : 0);
  } else {
    throw new Error(`node ${node.id}: unknown kind "${kind}"`);
  }
  if (node.w) w = node.w;
  if (node.h) h = node.h;

  const hw = w / 2;
  const hh = h / 2;
  const inside =
    kind === "diamond"
      ? (px, py) => Math.abs(px) / hw + Math.abs(py) / hh <= 1
      : (px, py) => Math.abs(px) <= hw && Math.abs(py) <= hh;
  return { kind, lines, w, h, hw, hh, inside };
}

/**
 * The point where a ray from (ox, oy) — relative to the node centre — towards
 * (dx, dy) leaves the node, pushed out by `gap`. Found by bisection so every
 * shape uses the same code.
 */
function exitPoint(shape, ox, oy, dx, dy, gap) {
  const len = Math.hypot(dx, dy) || 1;
  const ux = dx / len;
  const uy = dy / len;
  let lo = 0;
  let hi = 2000;
  for (let i = 0; i < 40; i++) {
    const mid = (lo + hi) / 2;
    if (shape.inside(ox + ux * mid, oy + uy * mid)) lo = mid;
    else hi = mid;
  }
  const t = lo + gap;
  return [ox + ux * t, oy + uy * t];
}

/** Bounding box of a node including the text drawn under a tile. */
function footprint(node, shape) {
  const x = node.x;
  const y = node.y;
  if (shape.kind !== "tile") {
    return { x0: x - shape.hw, y0: y - shape.hh, x1: x + shape.hw, y1: y + shape.hh + (node.caption ? 0 : 0) };
  }
  const nameW = textWidth(node.label ?? "", 15);
  const capW = node.caption ? textWidth(node.caption, 11.5, true) : 0;
  const half = Math.max(TILE / 2, nameW / 2, capW / 2);
  return { x0: x - half, y0: y - TILE / 2, x1: x + half, y1: y + TILE / 2 + (node.caption ? 40 : 24) };
}

// ---------------------------------------------------------------------------
// validation

export function validateSpec(spec) {
  const problems = [];
  const ids = new Set();
  const boxes = [];

  for (const g of spec.groups ?? []) assertTone(g.tone, `group ${g.id ?? g.label}`);

  for (const node of spec.nodes ?? []) {
    if (ids.has(node.id)) problems.push(`duplicate node id "${node.id}"`);
    ids.add(node.id);
    assertTone(node.tone, `node ${node.id}`);
    const shape = shapeOf(node);
    const box = footprint(node, shape);
    boxes.push({ id: node.id, ...box });
    if (box.x0 < 0 || box.y0 < 0 || box.x1 > spec.width || box.y1 > spec.height) {
      problems.push(`node "${node.id}" is partly outside the ${spec.width}x${spec.height} canvas`);
    }
  }

  for (let i = 0; i < boxes.length; i++) {
    for (let j = i + 1; j < boxes.length; j++) {
      const a = boxes[i];
      const b = boxes[j];
      const overlap = a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1;
      if (overlap) problems.push(`nodes "${a.id}" and "${b.id}" overlap`);
    }
  }

  for (const e of spec.edges ?? []) {
    for (const end of [e.from, e.to]) {
      if (!ids.has(end)) problems.push(`edge ${e.from} -> ${e.to}: no node "${end}"`);
    }
    assertTone(e.tone, `edge ${e.from} -> ${e.to}`);
  }

  if (problems.length > 0) {
    throw new Error(`diagram "${spec.name}" has problems:\n  - ${problems.join("\n  - ")}`);
  }
}

// ---------------------------------------------------------------------------
// drawing

function roundedPath(points, radius) {
  if (points.length < 2) return "";
  let d = `M ${num(points[0][0])} ${num(points[0][1])}`;
  for (let i = 1; i < points.length - 1; i++) {
    const [px, py] = points[i - 1];
    const [cx, cy] = points[i];
    const [nx, ny] = points[i + 1];
    const inLen = Math.hypot(cx - px, cy - py);
    const outLen = Math.hypot(nx - cx, ny - cy);
    const r = Math.min(radius, inLen / 2, outLen / 2);
    const ax = cx + ((px - cx) / inLen) * r;
    const ay = cy + ((py - cy) / inLen) * r;
    const bx = cx + ((nx - cx) / outLen) * r;
    const by = cy + ((ny - cy) / outLen) * r;
    d += ` L ${num(ax)} ${num(ay)} Q ${num(cx)} ${num(cy)} ${num(bx)} ${num(by)}`;
  }
  const last = points[points.length - 1];
  d += ` L ${num(last[0])} ${num(last[1])}`;
  return d;
}


/** Width of a step badge: a circle for one character, a pill for longer text. */
function badgeWidth(text) {
  return Math.max(22, 10 + String(text).length * 8);
}

/** A step badge whose left edge is at `left`, vertically centred on `cy`. */
function badge(left, cy, text, tone) {
  const w = badgeWidth(text);
  return [
    `<rect class="bd-${tone}" x="${num(left)}" y="${num(cy - 11)}" width="${num(w)}" height="22" rx="11"/>`,
    `<text class="dg-badge-t" x="${num(left + w / 2)}" y="${num(cy + 4.2)}" text-anchor="middle" font-family="${FONT_SANS}" font-size="12" font-weight="700">${esc(text)}</text>`,
  ].join("");
}

function css(palette) {
  const lines = [
    `.dg-bg{fill:${palette.bg}}`,
    `.dg-chip{fill:${palette.bg}}`,
    `.dg-tile{fill:${palette.surface};stroke:${palette.border};stroke-width:1.5}`,
    `.dg-name{fill:${palette.ink}}`,
    `.dg-cap{fill:${palette.ink2}}`,
    `.dg-text{fill:${palette.ink}}`,
    `.dg-text2{fill:${palette.ink2}}`,
    `.dg-elabel{paint-order:stroke;stroke:${palette.bg};stroke-width:5px;stroke-linejoin:round}`,
    `.dg-edge{fill:none;stroke-width:2;stroke-linecap:round;stroke-linejoin:round}`,
    `.dg-dash{stroke-dasharray:7 6}`,
    `.dg-badge-t{fill:#ffffff}`,
  ];
  for (const [name, t] of Object.entries(palette.tones)) {
    lines.push(
      `.c-${name}{color:${t.line}}`,
      `.tx-${name}{fill:${t.line}}`,
      `.bx-${name}{fill:${t.fill};stroke:${t.edge};stroke-width:1.5}`,
      `.gr-${name}{fill:${t.fill};stroke:${t.edge};stroke-width:1.5}`,
      `.ed-${name}{stroke:${t.line}}`,
      `.mk-${name}{fill:${t.line}}`,
      `.bd-${name}{fill:${t.line}}`,
    );
  }
  return lines.join("");
}

function drawGroup(g) {
  const t = g.tone ?? "gray";
  const dash = g.dashed ? ` stroke-dasharray="7 6"` : "";
  const labelW = textWidth(g.label, 12, true) + g.label.length * 0.9 + 20;
  return [
    `<rect class="gr-${t}" x="${num(g.x)}" y="${num(g.y)}" width="${num(g.w)}" height="${num(g.h)}" rx="16"${dash}/>`,
    `<rect class="dg-chip" x="${num(g.x + 14)}" y="${num(g.y - 11)}" width="${num(labelW)}" height="22"/>`,
    `<text class="tx-${t}" x="${num(g.x + 24)}" y="${num(g.y + 4)}" font-family="${FONT_MONO}" font-size="12" font-weight="700" letter-spacing="1">${esc(g.label)}</text>`,
  ].join("");
}

function drawNode(node, shape) {
  const t = node.tone ?? "gray";
  const { x, y } = node;
  if (shape.kind === "tile") {
    const tx = x - TILE / 2;
    const ty = y - TILE / 2;
    const out = [
      `<rect class="dg-tile" x="${num(tx)}" y="${num(ty)}" width="${TILE}" height="${TILE}" rx="14"/>`,
    ];
    if (node.icon) {
      out.push(
        `<g class="c-${t}" transform="translate(${num(x - 16)} ${num(y - 16)})" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${iconSvg(node.icon)}</g>`,
      );
    }
    out.push(
      `<text class="dg-name" x="${num(x)}" y="${num(y + TILE / 2 + 20)}" text-anchor="middle" font-family="${FONT_SANS}" font-size="15" font-weight="600">${esc(node.label ?? "")}</text>`,
    );
    if (node.caption) {
      out.push(
        `<text class="dg-cap" x="${num(x)}" y="${num(y + TILE / 2 + 37)}" text-anchor="middle" font-family="${FONT_MONO}" font-size="11.5">${esc(node.caption)}</text>`,
      );
    }
    return out.join("");
  }

  const x0 = x - shape.hw;
  const y0 = y - shape.hh;
  let outline;
  if (shape.kind === "diamond") {
    outline = `<polygon class="bx-${t}" points="${num(x)},${num(y0)} ${num(x + shape.hw)},${num(y)} ${num(x)},${num(y + shape.hh)} ${num(x - shape.hw)},${num(y)}"/>`;
  } else {
    const rx = shape.kind === "pill" ? shape.hh : 10;
    outline = `<rect class="bx-${t}" x="${num(x0)}" y="${num(y0)}" width="${num(shape.w)}" height="${num(shape.h)}" rx="${num(rx)}"/>`;
  }

  const lineH = 20;
  const textBlock = shape.lines.length * lineH + (node.caption ? 18 : 0);
  const firstY = y - textBlock / 2 + 15;
  const out = [outline];
  if (node.badge !== undefined) {
    // A step number on the shape's top-left corner, like the numbered edges.
    const left = shape.kind === "diamond" ? x - shape.hw + 8 : x0 - 9;
    const cy = shape.kind === "diamond" ? y - 4 : y0 + 2;
    out.push(badge(left, cy, node.badge, t));
  }
  shape.lines.forEach((line, i) => {
    out.push(
      `<text class="dg-name" x="${num(x)}" y="${num(firstY + i * lineH)}" text-anchor="middle" font-family="${FONT_SANS}" font-size="14" font-weight="600">${esc(line)}</text>`,
    );
  });
  if (node.caption) {
    out.push(
      `<text class="dg-cap" x="${num(x)}" y="${num(firstY + shape.lines.length * lineH + 2)}" text-anchor="middle" font-family="${FONT_MONO}" font-size="11">${esc(node.caption)}</text>`,
    );
  }
  return out.join("");
}

function edgeGeometry(edge, nodes, shapes) {
  const from = nodes.get(edge.from);
  const to = nodes.get(edge.to);
  const via = edge.via ?? [];
  const fo = edge.fromOffset ?? [0, 0];
  const to_ = edge.toOffset ?? [0, 0];

  const fromRef = [from.x + fo[0], from.y + fo[1]];
  const toRef = [to.x + to_[0], to.y + to_[1]];
  const afterFrom = via[0] ?? toRef;
  const beforeTo = via[via.length - 1] ?? fromRef;

  const [sx, sy] = exitPoint(shapes.get(from.id), fo[0], fo[1], afterFrom[0] - fromRef[0], afterFrom[1] - fromRef[1], 3);
  const [ex, ey] = exitPoint(shapes.get(to.id), to_[0], to_[1], beforeTo[0] - toRef[0], beforeTo[1] - toRef[1], 6);

  return [[from.x + sx, from.y + sy], ...via, [to.x + ex, to.y + ey]];
}

function pointOnPolyline(points, segIndex, t) {
  const [ax, ay] = points[segIndex];
  const [bx, by] = points[segIndex + 1];
  return [ax + (bx - ax) * t, ay + (by - ay) * t];
}

function longestSegment(points) {
  let best = 0;
  let bestLen = -1;
  for (let i = 0; i < points.length - 1; i++) {
    const len = Math.hypot(points[i + 1][0] - points[i][0], points[i + 1][1] - points[i][1]);
    if (len > bestLen) {
      bestLen = len;
      best = i;
    }
  }
  return best;
}

function drawEdge(edge, points, prefix) {
  const t = edge.tone ?? "blue";
  const dash = edge.dashed ? " dg-dash" : "";
  const out = [
    `<path class="dg-edge ed-${t}${dash}" d="${roundedPath(points, 10)}" marker-end="url(#${prefix}-arrow-${t})"/>`,
  ];
  if (edge.bidirectional) {
    out[0] = out[0].replace(`marker-end`, `marker-start="url(#${prefix}-arrow-${t})" marker-end`);
  }

  if (edge.label || edge.step !== undefined) {
    const seg = edge.labelSeg ?? longestSegment(points);
    const [px, py] = pointOnPolyline(points, seg, edge.labelAt ?? 0.5);
    const lx = px + (edge.labelDx ?? 0);
    const ly = py + (edge.labelDy ?? 0);
    const badgeSpace = edge.step !== undefined ? badgeWidth(edge.step) + 4 : 0;
    const textW = edge.label ? textWidth(edge.label, 12, true) : 0;
    const total = badgeSpace + textW;
    let cursor = lx - total / 2;
    if (edge.step !== undefined) {
      out.push(badge(cursor, ly, edge.step, t));
      cursor += badgeSpace;
    }
    if (edge.label) {
      out.push(
        `<text class="dg-elabel tx-${t}" x="${num(cursor)}" y="${num(ly + 4)}" font-family="${FONT_MONO}" font-size="12">${esc(edge.label)}</text>`,
      );
    }
  }
  return out.join("");
}

function drawLabel(l) {
  const size = l.size ?? 12;
  const mono = l.mono ?? true;
  const cls = l.ink ? "dg-text" : l.tone ? `tx-${l.tone}` : "dg-text2";
  const lines = String(l.text).split("\n");
  return lines
    .map(
      (line, i) =>
        `<text class="${cls}" x="${num(l.x)}" y="${num(l.y + i * (size + 5))}" text-anchor="${l.anchor ?? "middle"}" font-family="${mono ? FONT_MONO : FONT_SANS}" font-size="${size}"${l.weight ? ` font-weight="${l.weight}"` : ""}>${esc(line)}</text>`,
    )
    .join("");
}

/**
 * Renders `spec` to an SVG string.
 *   mode "standalone": fixed light palette and a background, for .svg files
 *   mode "page":       follows the host page's CSS variables, no background
 */
export function renderDiagram(spec, { mode = "standalone" } = {}) {
  validateSpec(spec);
  const palette = mode === "page" ? PAGE : LIGHT;

  const nodes = new Map(spec.nodes.map((n) => [n.id, n]));
  const shapes = new Map(spec.nodes.map((n) => [n.id, shapeOf(n)]));

  const markers = TONE_NAMES.map(
    (name) =>
      `<marker id="dg-${spec.name}-arrow-${name}" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="9" markerHeight="9" orient="auto-start-reverse"><path class="mk-${name}" d="M1 1 L9 5 L1 9 z"/></marker>`,
  ).join("");

  const sizeAttrs =
    mode === "page"
      ? `class="dg" style="width:100%;height:auto;max-width:${spec.width}px"`
      : `width="${spec.width}" height="${spec.height}"`;

  const parts = [
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${spec.width} ${spec.height}" ${sizeAttrs} role="img" aria-labelledby="dg-t-${spec.name} dg-d-${spec.name}">`,
    `<title id="dg-t-${spec.name}">${esc(spec.title)}</title>`,
    `<desc id="dg-d-${spec.name}">${esc(spec.desc)}</desc>`,
    `<defs><style>${css(palette)}</style>${markers}</defs>`,
  ];
  if (mode === "standalone") {
    parts.push(`<rect class="dg-bg" width="${spec.width}" height="${spec.height}" rx="18"/>`);
  }
  for (const g of spec.groups ?? []) parts.push(drawGroup(g));
  for (const e of spec.edges ?? []) parts.push(drawEdge(e, edgeGeometry(e, nodes, shapes), `dg-${spec.name}`));
  for (const n of spec.nodes) parts.push(drawNode(n, shapes.get(n.id)));
  for (const l of spec.labels ?? []) parts.push(drawLabel(l));
  parts.push(`</svg>`);
  return parts.join("\n");
}
