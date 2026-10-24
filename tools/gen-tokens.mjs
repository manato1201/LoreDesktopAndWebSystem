// Generates every platform's design tokens from design-tokens/tokens.json.
//
//   node tools/gen-tokens.mjs           write the generated files
//   node tools/gen-tokens.mjs --check   exit 1 if any generated file is stale
//
// Generated files must never be edited by hand: change tokens.json and rerun.

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const TOKENS_PATH = join(ROOT, "design-tokens", "tokens.json");

const CSS_OUT = join(ROOT, "lorehub-web", "src", "app", "tokens.generated.css");
const QML_OUTS = [
  join(ROOT, "loreforge-client", "qml", "Theme.qml"),
  join(ROOT, "loreforge-server-admin", "qml", "Theme.qml"),
];

const HEADER =
  "Generated from design-tokens/tokens.json by tools/gen-tokens.mjs. Do not edit by hand.";

export function renderCss(tokens) {
  const lines = [];
  for (const c of tokens.colors) lines.push(`  ${c.css}: ${c.value};`);
  lines.push("");
  for (const r of tokens.radii) lines.push(`  ${r.css}: ${r.px}px;`);
  lines.push("");
  for (const s of tokens.spacing) lines.push(`  ${s.css}: ${s.px}px;`);
  lines.push("");
  for (const s of tokens.shadows) lines.push(`  ${s.css}: ${s.value};`);
  lines.push("");
  lines.push(`  ${tokens.fontFamily.css}: ${tokens.fontFamily.value};`);

  return `/* ${HEADER} */\n:root {\n${lines.join("\n")}\n}\n`;
}

export function renderQml(tokens) {
  const color = (c) => `    readonly property color ${c.qml}: "${c.value}"`;
  const int = (t) => `    readonly property int ${t.qml}: ${t.px}`;

  const blocks = [
    tokens.colors.slice(0, 4).map(color),
    tokens.colors.slice(4, 6).map(color),
    tokens.colors.slice(6, 8).map(color),
    tokens.colors.slice(8, 10).map(color),
    tokens.colors.slice(10).map(color),
    tokens.radii.filter((r) => r.qml).map(int),
    tokens.spacing.filter((s) => s.qml).map(int),
    tokens.fontSizes.map(int),
  ];

  return `pragma Singleton
import QtQuick

// ${HEADER}
// Roles are documented in DESIGN.md section 10.
QtObject {
${blocks.map((b) => b.join("\n")).join("\n\n")}
}
`;
}

function outputs(tokens) {
  return [
    [CSS_OUT, renderCss(tokens)],
    ...QML_OUTS.map((path) => [path, renderQml(tokens)]),
  ];
}

function main() {
  const tokens = JSON.parse(readFileSync(TOKENS_PATH, "utf8"));
  const check = process.argv.includes("--check");
  let stale = 0;

  for (const [path, content] of outputs(tokens)) {
    // Git on Windows may check files out with CRLF; the content is the same.
    const current = existsSync(path)
      ? readFileSync(path, "utf8").replaceAll("\r\n", "\n")
      : null;
    if (current === content) continue;
    if (check) {
      console.error(`stale: ${path}`);
      stale += 1;
    } else {
      writeFileSync(path, content);
      console.log(`wrote: ${path}`);
    }
  }

  if (check && stale > 0) {
    console.error("Run `node tools/gen-tokens.mjs` and commit the result.");
    process.exit(1);
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
