/**
 * Source-scan responsive-overlay regression test — US-021
 * (R-6 from 07-usability-improvements.md).
 *
 * Command palettes, notification panels, and dropdowns are positioned
 * overlays whose contents can expand beyond the viewport. Each must
 * declare a viewport-aware `max-height` so a long list doesn't push
 * below the bottom of the screen. The house convention is
 * `min(Npx, Mvh)` — e.g. `min(400px, 60vh)`.
 *
 * This test enumerates each overlay and asserts the style block it
 * owns declares `maxHeight` (inline) or `max-height` (CSS) with a
 * viewport-relative fallback.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

interface Target {
  file: string;
  css?: string;
  /** A substring that must appear near the max-height declaration
   *  (used to anchor the assertion on the right block). */
  anchor: string;
}

const TARGETS: Target[] = [
  { file: "CommandPalette.tsx", anchor: "command-palette-list" },
  { file: "NotificationCenter.css", css: "NotificationCenter.css", anchor: ".notification-center__panel" },
  { file: "AutomationsPanel.tsx", anchor: "ResolutionBadge" },
];

function loadFile(name: string): string {
  return readFileSync(resolve(__dirname, "..", name), "utf8");
}

const VIEWPORT_MAX_HEIGHT =
  /max[-]?[Hh]eight\s*[:=]?\s*["']?\s*(?:min\s*\(|[^"';\n]*?vh\b)/;

describe("US-021 — overlays declare viewport-aware max-height (R-6)", () => {
  for (const t of TARGETS) {
    it(`${t.file} overlay near "${t.anchor}" carries viewport-aware max-height`, () => {
      const src = loadFile(t.file);
      const idx = src.indexOf(t.anchor);
      expect(idx, `anchor "${t.anchor}" not found in ${t.file}`).toBeGreaterThan(-1);
      // Scan the 1200 chars following the anchor — long enough to span
      // the element's style block without straying into unrelated rules.
      const slice = src.slice(idx, idx + 1200);
      expect(
        VIEWPORT_MAX_HEIGHT.test(slice),
        `${t.file} overlay at "${t.anchor}" lacks a viewport-aware max-height. Slice:\n${slice.slice(0, 400)}`,
      ).toBe(true);
    });
  }
});
