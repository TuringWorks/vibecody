/**
 * Source-scan a11y regression test — US-016 (A-6 from 07-usability-improvements.md).
 *
 * Modal dialog overlays must carry `role="dialog"` + `aria-modal="true"`
 * so AT knows it's a modal (focus trap expected, rest of page is
 * inert for AT purposes). AgilePanel's sprint-details and card-editor
 * overlays were using `role="button"` on both the backdrop AND the
 * modal pane — incorrect for both (backdrops are presentational; the
 * modal pane is a dialog).
 *
 * The test scans named panel files for modal pane JSX that sits at
 * `position: "fixed", inset: 0, zIndex: …` and asserts it's marked
 * as a dialog, not a button.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const TARGETS = [
  "AgilePanel.tsx",
];

function loadPanel(name: string): string {
  return readFileSync(resolve(__dirname, "..", name), "utf8");
}

describe("US-016 — modal overlays carry role=dialog + aria-modal", () => {
  for (const file of TARGETS) {
    it(`${file}: every "position: fixed, inset: 0, zIndex" overlay declares role=dialog or is a presentational backdrop`, () => {
      const src = loadPanel(file);
      // Find each opening tag containing `position: "fixed"` + `inset: 0`.
      // Tempered-greedy stops at the nearest `>` that closes the tag.
      const re =
        /<(?:div|section|aside)\b[^>]*position\s*:\s*["']fixed["'][^>]*inset\s*:\s*0[^>]*>/g;
      const overlays = src.match(re) ?? [];
      expect(overlays.length).toBeGreaterThan(0);
      for (const tag of overlays) {
        const isDialog = /\brole\s*=\s*["']dialog["']/.test(tag)
          && /\baria-modal\s*=/.test(tag);
        const isBackdrop = /\baria-hidden\s*=\s*\{?\s*["']?true["']?/.test(tag)
          || /\brole\s*=\s*["']presentation["']/.test(tag);
        expect(
          isDialog || isBackdrop,
          `Overlay in ${file} lacks role="dialog" + aria-modal (or aria-hidden for backdrops):\n  ${tag}`,
        ).toBe(true);
      }
    });
  }
});
