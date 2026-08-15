#!/usr/bin/env node
/**
 * Guard: the welcome screen's heading must never be sliced off the top.
 *
 * The screen stacks a heading, a subtitle, two buttons, two shortcut grids and
 * a feature list inside `height: 100%`. On a short window that overflows, and
 * `justify-content: center` pushes the excess out of BOTH ends — the half above
 * the top edge is not reachable by scrolling, because a scroll container's
 * origin is its top. The heading renders cut in half and there is nothing the
 * user can do about it.
 *
 * This has been fixed at least twice and come back, which is what a guard is
 * for. It cannot be caught by `tsc` or by jsdom: it only exists once a real
 * engine has laid the box out, so this measures in a real one.
 *
 * Usage: node scripts/check-welcome-layout.mjs   (run from vibecoder/)
 *
 * Not in the CI matrix: it needs a browser binary
 * (`npx playwright install chromium`) that CI does not currently download. Run
 * it locally when touching `.welcome-screen`, and via `make lint-welcome`.
 */
import { chromium } from "playwright";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const css = readFileSync(resolve(import.meta.dirname, "../src/App.css"), "utf8");

/** The welcome screen as `App.tsx` renders it, at its real content volume. */
function html() {
  const rows = (n) =>
    Array.from({ length: n }, (_, i) => `<div style="font-size:13px">Shortcut ${i} — description</div>`).join("");
  return `<!doctype html><html><head><style>
    html,body,#root{height:100%;margin:0}
    /* Stands in for the editor area, which clips its overflow. */
    .editor-area{height:100%;overflow:hidden}
    ${css}
  </style></head><body><div id="root"><div class="editor-area">
    <div class="welcome-screen">
      <h2>Welcome to VibeCoder</h2>
      <p>AI-Powered Code Editor built with Rust + Tauri</p>
      <div class="welcome-actions"><button>Open Folder</button><button>Take a Tour</button></div>
      <div class="features">
        <h3>Keyboard Shortcuts</h3>
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:8px 24px">${rows(14)}</div>
        <h3>With a file open, in the editor</h3>
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:8px 24px">${rows(10)}</div>
        <h3>Features</h3>
        <ul><li>one</li><li>two</li><li>three</li><li>four</li></ul>
      </div>
    </div>
  </div></div></body></html>`;
}

// 280px is not a hypothetical: the reported screenshot was a window this short.
const HEIGHTS = [1000, 600, 400, 280];

const browser = await chromium.launch();
const failures = [];
let sawOverflow = false;

for (const height of HEIGHTS) {
  const page = await browser.newPage({ viewport: { width: 1200, height } });
  await page.setContent(html());
  const m = await page.evaluate(() => {
    const box = document.querySelector(".welcome-screen");
    const heading = document.querySelector(".welcome-screen h2");
    const b = box.getBoundingClientRect();
    const h = heading.getBoundingClientRect();
    return {
      overflows: box.scrollHeight > box.clientHeight,
      // Negative means the heading begins above the container's top edge.
      headingTop: Math.round(h.top - b.top),
      headingHeight: Math.round(h.height),
    };
  });
  await page.close();

  if (m.overflows) sawOverflow = true;
  if (m.headingTop < 0) {
    failures.push(
      `  ${height}px window: heading starts ${-m.headingTop}px above the top edge ` +
        `(heading is ${m.headingHeight}px tall) — sliced, and unreachable by scrolling`,
    );
  } else {
    console.log(`  ${String(height).padStart(4)}px window: heading fully visible (top +${m.headingTop}px)`);
  }
}
await browser.close();

// Without this the check could pass by never having produced the condition it
// guards — a green run that measured nothing.
if (!sawOverflow) {
  console.error(
    "✖ the fixture never overflowed at any tested height, so nothing was actually guarded.\n" +
      "  Either the content shrank or the harness is wrong; adjust HEIGHTS or the fixture.",
  );
  process.exit(1);
}

if (failures.length > 0) {
  console.error("✖ The welcome heading is clipped off the top:\n" + failures.join("\n"));
  console.error(
    "\n  `.welcome-screen` centres with auto margins on its end children so that\n" +
      "  overflow can only spill downward. Restoring `justify-content: center`\n" +
      "  brings this back.",
  );
  process.exit(1);
}

console.log(`✓ heading intact at ${HEIGHTS.length} window heights, including ones that overflow.`);
