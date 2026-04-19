/**
 * Source-scan regression test — US-013 (C-1 from 07-usability-improvements.md).
 *
 * The design-system contract says UI colors must come from tokens
 * (`var(--…)`) so themes can remap them consistently. Panels that
 * hard-code hex values (e.g. `"#6b7280"`) break dark-mode parity and
 * resist theme overrides.
 *
 * Legitimate hex usage that's NOT a violation:
 *   - CSS fallback inside `var(--x, #abcdef)` — token is primary, hex
 *     only fires if the token is undefined (rare; kept for safety).
 *   - Color-picker / color-tools panels where hex IS the content.
 *
 * Scope is a named target list; migrate panels incrementally and add
 * them here once clean.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const TARGETS = [
  "AdminPanel.tsx",
  "AgilePanel.tsx",
  "BatchBuilderPanel.tsx",
];

function loadPanel(name: string): string {
  return readFileSync(resolve(__dirname, "..", name), "utf8");
}

/** Strip allowed hex usages so the remaining hex count is the violations. */
function stripAllowedHex(src: string): string {
  // 1. Remove `var(--token, #rrggbb)` fallbacks — hex is a defensive default.
  let cleaned = src.replace(/var\([^)]*#[0-9a-fA-F]{3,8}[^)]*\)/g, "");
  // 2. Remove hex in code comments (`// …#abcdef` and `/* …#abcdef */`).
  cleaned = cleaned.replace(/\/\/[^\n]*/g, "");
  cleaned = cleaned.replace(/\/\*[\s\S]*?\*\//g, "");
  return cleaned;
}

describe("US-013 — hex colors replaced with design tokens", () => {
  for (const file of TARGETS) {
    it(`${file}: contains no literal #rrggbb outside var() fallbacks`, () => {
      const src = stripAllowedHex(loadPanel(file));
      const matches = src.match(/#[0-9a-fA-F]{6}\b/g) ?? [];
      expect(
        matches.length,
        `Found hex literals in ${file}: ${matches.join(", ")}`,
      ).toBe(0);
    });
  }
});
