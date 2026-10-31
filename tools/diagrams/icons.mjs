// Glyphs for diagram nodes. Each is drawn in a 32 x 32 box with strokes only,
// so the node's tone colour (set on the wrapping <g>) paints all of it.
//
// To add one: draw it in 32 x 32, use `stroke` styling only (the wrapper
// supplies fill="none", stroke="currentColor", round caps), and register it
// below under a short name.

const dot = (x, y) => `<circle cx="${x}" cy="${y}" r="1.1" fill="currentColor" stroke="none"/>`;

export const ICONS = {
  browser: `<rect x="3" y="6" width="26" height="20" rx="3"/><path d="M3 12h26"/>${dot(7, 9)}${dot(10.5, 9)}`,
  monitor: `<rect x="3" y="5" width="26" height="17" rx="2.5"/><path d="M12 28h8M16 22v6"/>`,
  user: `<circle cx="16" cy="11" r="5"/><path d="M5 28c1-6 5-9 11-9s10 3 11 9"/>`,
  server: `<rect x="4" y="5" width="24" height="9" rx="2.5"/><rect x="4" y="18" width="24" height="9" rx="2.5"/>${dot(9, 9.5)}${dot(9, 22.5)}`,
  database: `<ellipse cx="16" cy="8" rx="10" ry="4"/><path d="M6 8v16c0 2.2 4.5 4 10 4s10-1.8 10-4V8"/><path d="M6 16c0 2.2 4.5 4 10 4s10-1.8 10-4"/>`,
  cube: `<path d="M16 3l11 6v14l-11 6-11-6V9z"/><path d="M5 9l11 6 11-6M16 15v14"/>`,
  gear: `<circle cx="16" cy="16" r="5"/><path d="M16 3v4M16 25v4M3 16h4M25 16h4M7 7l3 3M22 22l3 3M25 7l-3 3M10 22l-3 3"/>`,
  shield: `<path d="M16 3l11 4v8c0 7-5 12-11 14C10 27 5 22 5 15V7z"/><path d="M11 16l4 4 7-8"/>`,
  doc: `<path d="M8 3h11l6 6v20H8z"/><path d="M19 3v6h6M12 16h9M12 21h9"/>`,
  lock: `<rect x="6" y="14" width="20" height="14" rx="3"/><path d="M10 14V10a6 6 0 0 1 12 0v4"/>`,
  hash: `<path d="M12 5L9 27M23 5l-3 22M5 12h22M4 21h22"/>`,
  branch: `<circle cx="9" cy="6" r="3"/><circle cx="9" cy="26" r="3"/><circle cx="23" cy="11" r="3"/><path d="M9 9v14M23 14c0 6-14 3-14 9"/>`,
  terminal: `<rect x="3" y="5" width="26" height="22" rx="3"/><path d="M9 13l5 4-5 4M17 22h7"/>`,
  swatches: `<rect x="4" y="4" width="10" height="10" rx="2"/><rect x="18" y="4" width="10" height="10" rx="2"/><rect x="4" y="18" width="10" height="10" rx="2"/><circle cx="23" cy="23" r="5"/>`,
  cloud: `<path d="M9 24a6 6 0 0 1-.6-12A8 8 0 0 1 24 12a6 6 0 0 1 0 12z"/>`,
  braces: `<path d="M12 5c-4 0-4 3-4 6s0 5-3 5c3 0 3 2 3 5s0 6 4 6M20 5c4 0 4 3 4 6s0 5 3 5c-3 0-3 2-3 5s0 6-4 6"/>`,
  workflow: `<circle cx="8" cy="16" r="4"/><circle cx="24" cy="8" r="4"/><circle cx="24" cy="24" r="4"/><path d="M12 16h4c4 0 4-8 8-8M16 16c4 0 4 8 8 8"/>`,
  folder: `<path d="M3 8a2 2 0 0 1 2-2h7l3 3h12a2 2 0 0 1 2 2v13a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>`,
  file: `<path d="M8 3h11l6 6v20H8z"/><path d="M19 3v6h6"/>`,
  tree: `<path d="M16 29V15M16 20l-6-5M16 16l7-6M16 11l-4-5"/><circle cx="10" cy="14" r="2.4"/><circle cx="23" cy="9" r="2.4"/><circle cx="12" cy="5" r="2.4"/>`,
  book: `<path d="M5 6c4-2 8-2 11 1 3-3 7-3 11-1v19c-4-2-8-2-11 1-3-3-7-3-11-1z"/><path d="M16 7v19"/>`,
  mail: `<rect x="3" y="7" width="26" height="18" rx="3"/><path d="M4 9l12 9 12-9"/>`,
  chart: `<path d="M4 4v24h24"/><path d="M9 21l5-6 4 3 7-9"/>`,
  key: `<circle cx="10" cy="16" r="5"/><path d="M15 16h13M24 16v5M28 16v3"/>`,
};

/** Inner SVG for `name`, or throws so a typo in a spec fails loudly. */
export function iconSvg(name) {
  const body = ICONS[name];
  if (!body) {
    throw new Error(`unknown icon "${name}" (known: ${Object.keys(ICONS).join(", ")})`);
  }
  return body;
}
