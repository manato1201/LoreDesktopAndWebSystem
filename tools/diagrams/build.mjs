// Builds every diagram from its spec.
//
//   node tools/diagrams/build.mjs           write the files
//   node tools/diagrams/build.mjs --check   exit 1 if any file is out of date
//
// Outputs
//   docs/diagrams/<name>.svg          standalone, light palette (README, Markdown)
//   docs/lore-ecosystem-docs.html     page-themed copy injected between
//                                     <!-- diagram:<name> --> and <!-- /diagram:<name> -->
//
// Edit a spec in tools/diagrams/specs/, run this, commit the results.

import { readdirSync, readFileSync, writeFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { renderDiagram } from "./render.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..", "..");
const SPEC_DIR = join(HERE, "specs");
const SVG_DIR = join(ROOT, "docs", "diagrams");
const HTML_PATH = join(ROOT, "docs", "lore-ecosystem-docs.html");

export async function loadSpecs() {
  const files = readdirSync(SPEC_DIR)
    .filter((f) => f.endsWith(".mjs"))
    .sort();
  const specs = [];
  for (const file of files) {
    const mod = await import(pathToFileURL(join(SPEC_DIR, file)).href);
    const spec = mod.default;
    if (spec.name + ".mjs" !== file) {
      throw new Error(`${file}: spec.name is "${spec.name}", expected the file name`);
    }
    specs.push(spec);
  }
  return specs;
}

/** Replaces the block between the markers for `name`, or returns null if the page has none. */
export function injectIntoHtml(html, name, svg) {
  const open = `<!-- diagram:${name} -->`;
  const close = `<!-- /diagram:${name} -->`;
  const start = html.indexOf(open);
  const end = html.indexOf(close);
  if (start === -1 || end === -1 || end < start) return null;
  return html.slice(0, start + open.length) + "\n" + svg + "\n" + html.slice(end);
}

async function main() {
  const check = process.argv.includes("--check");
  const specs = await loadSpecs();
  let stale = 0;

  const write = (path, content) => {
    // Git on Windows may check files out with CRLF; the content is the same.
    const current = existsSync(path) ? readFileSync(path, "utf8").replaceAll("\r\n", "\n") : null;
    if (current === content) return;
    if (check) {
      console.error(`stale: ${path}`);
      stale += 1;
    } else {
      writeFileSync(path, content);
      console.log(`wrote: ${path}`);
    }
  };

  if (!existsSync(SVG_DIR)) mkdirSync(SVG_DIR, { recursive: true });

  let html = readFileSync(HTML_PATH, "utf8").replaceAll("\r\n", "\n");
  const usedInHtml = [];

  for (const spec of specs) {
    write(join(SVG_DIR, `${spec.name}.svg`), renderDiagram(spec, { mode: "standalone" }) + "\n");
    const next = injectIntoHtml(html, spec.name, renderDiagram(spec, { mode: "page" }));
    if (next !== null) {
      html = next;
      usedInHtml.push(spec.name);
    }
  }
  write(HTML_PATH, html);

  console.log(`${specs.length} diagrams; ${usedInHtml.length} embedded in the HTML page`);
  if (check && stale > 0) {
    console.error("Run `node tools/diagrams/build.mjs` and commit the result.");
    process.exit(1);
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await main();
