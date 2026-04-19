/**
 * Source-scan regression test — US-023 (R-4 from 07-usability-improvements.md).
 *
 * Fixed-width sidebars that live inside a flex row collapse below their
 * intended width when the parent narrows unless they carry `flexShrink: 0`.
 * The audit called out CanvasPanel (palette + properties) and DatabasePanel
 * (saved connections + schema tree). This test locks in the fix so a future
 * refactor can't silently drop the protection.
 *
 * Scan shape: every inline-style block that declares `borderRight` OR
 * `borderLeft` together with a numeric pixel `width` is treated as a
 * sidebar candidate. Each candidate must also declare `flexShrink: 0`.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const TARGETS = ["CanvasPanel.tsx", "DatabasePanel.tsx"] as const;

function loadPanel(name: string): string {
  return readFileSync(resolve(__dirname, "..", name), "utf8");
}

/** Collect every `style={{ ... }}` body that contains `borderRight` or
 *  `borderLeft` AND a numeric `width: N` (not a percentage / var). */
function extractSidebarStyleBodies(src: string): string[] {
  const bodies: string[] = [];
  // Tempered-greedy body capture. `[\s\S]*?` keeps it non-greedy across
  // newlines; we stop at the first `}}` which closes the inline style.
  const re = /style=\{\{([\s\S]*?)\}\}/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(src)) !== null) {
    const body = m[1];
    const hasBorder = /\bborder(Right|Left)\s*:/.test(body);
    const hasPxWidth = /\bwidth\s*:\s*\d{2,4}\b/.test(body);
    if (hasBorder && hasPxWidth) bodies.push(body);
  }
  return bodies;
}

describe("US-023 — fixed-width sidebars declare flexShrink:0 (R-4)", () => {
  for (const file of TARGETS) {
    it(`${file}: every sidebar (border + pixel width) carries flexShrink:0`, () => {
      const src = loadPanel(file);
      const sidebars = extractSidebarStyleBodies(src);
      expect(
        sidebars.length,
        `${file} has no sidebar-shaped style blocks — audit pattern moved?`,
      ).toBeGreaterThan(0);
      for (const body of sidebars) {
        expect(
          /\bflexShrink\s*:\s*0\b/.test(body),
          `${file} sidebar missing flexShrink:0. Style body:\n  ${body.replace(/\s+/g, " ").trim().slice(0, 240)}`,
        ).toBe(true);
      }
    });
  }
});
