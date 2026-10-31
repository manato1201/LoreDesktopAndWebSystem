// Run with: node --test tools/diagrams
import { test } from "node:test";
import assert from "node:assert/strict";
import { renderDiagram, textWidth, validateSpec } from "./render.mjs";
import { injectIntoHtml, loadSpecs } from "./build.mjs";
import { ICONS } from "./icons.mjs";

const tiny = () => ({
  name: "tiny",
  title: "t",
  desc: "d",
  width: 600,
  height: 300,
  nodes: [
    { id: "a", x: 120, y: 120, icon: "user", label: "A", tone: "blue" },
    { id: "b", x: 420, y: 120, icon: "server", label: "B", tone: "green" },
  ],
  edges: [{ from: "a", to: "b", label: "go", step: 1 }],
});

test("every real spec validates and renders in both modes without junk values", async () => {
  const specs = await loadSpecs();
  assert.ok(specs.length >= 8, "expected the full diagram set");
  for (const spec of specs) {
    for (const mode of ["standalone", "page"]) {
      const svg = renderDiagram(spec, { mode });
      assert.match(svg, /^<svg [^>]*viewBox="0 0 \d+ \d+"/, `${spec.name}: starts with an <svg>`);
      assert.ok(svg.trimEnd().endsWith("</svg>"), `${spec.name}: ends with </svg>`);
      assert.ok(!svg.includes("NaN"), `${spec.name}: has NaN`);
      assert.ok(!svg.includes("undefined"), `${spec.name}: has undefined`);
    }
  }
});

test("rendering is deterministic (so --check can compare files)", () => {
  assert.equal(renderDiagram(tiny()), renderDiagram(tiny()));
});

test("standalone has its own background; page mode follows the host page", () => {
  assert.match(renderDiagram(tiny(), { mode: "standalone" }), /class="dg-bg"/);
  const page = renderDiagram(tiny(), { mode: "page" });
  assert.ok(!page.includes('class="dg-bg"'));
  assert.match(page, /var\(--bg-surface/);
});

test("text in the diagram is escaped", () => {
  const spec = tiny();
  spec.nodes[0].label = "a < b & c";
  assert.match(renderDiagram(spec), /a &lt; b &amp; c/);
});

test("an edge to a node that does not exist is an error", () => {
  const spec = tiny();
  spec.edges.push({ from: "a", to: "nope" });
  assert.throws(() => validateSpec(spec), /no node "nope"/);
});

test("two nodes drawn on top of each other are an error", () => {
  const spec = tiny();
  spec.nodes[1].x = 130;
  assert.throws(() => validateSpec(spec), /overlap/);
});

test("a node off the canvas is an error", () => {
  const spec = tiny();
  spec.nodes[0].x = 10;
  assert.throws(() => validateSpec(spec), /outside/);
});

test("a typo in an icon or tone fails loudly instead of drawing nothing", () => {
  const badIcon = tiny();
  badIcon.nodes[0].icon = "serverr";
  assert.throws(() => renderDiagram(badIcon), /unknown icon/);

  const badTone = tiny();
  badTone.nodes[0].tone = "purple";
  assert.throws(() => renderDiagram(badTone), /unknown tone/);
});

test("duplicate node ids are an error", () => {
  const spec = tiny();
  spec.nodes[1].id = "a";
  assert.throws(() => validateSpec(spec), /duplicate/);
});

test("CJK text is measured wider than Latin text of the same length", () => {
  assert.ok(textWidth("ああああ", 12) > textWidth("aaaa", 12));
});

test("every registered icon is drawable", () => {
  for (const [name, body] of Object.entries(ICONS)) {
    assert.ok(body.includes("<"), `${name} has markup`);
  }
});

test("injectIntoHtml replaces only the marked block", () => {
  const html = "A<!-- diagram:x -->old<!-- /diagram:x -->B";
  assert.equal(
    injectIntoHtml(html, "x", "<svg/>"),
    "A<!-- diagram:x -->\n<svg/>\n<!-- /diagram:x -->B",
  );
  assert.equal(injectIntoHtml(html, "missing", "<svg/>"), null);
});
