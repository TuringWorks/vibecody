/**
 * Source-scan regression test — US-014 (C-2 and C-3 from
 * 07-usability-improvements.md).
 *
 * Modal backdrops and shadowed floating surfaces should draw from the
 * `--overlay-bg` / `--elevation-1..3` tokens defined in
 * `design-system/tokens.css`. Inline `rgba(0, 0, 0, 0.5)` backdrops
 * and bespoke `boxShadow: "0 4px 12px rgba(0, 0, 0, 0.35)"` drops
 * bypass the theming pipeline: dark-theme shadows look wrong in
 * light-theme and vice versa.
 *
 * Allowed:
 *   - `var(--elevation-3, 0 8px 32px rgba(0,0,0,0.4))` — token primary
 *     with a defensive fallback.
 *
 * Forbidden in TARGETS:
 *   - Inline `rgba(0, 0, 0, 0.4…0.6)` as a `background` value.
 *   - Inline multi-arg `boxShadow: "0 Npx Mpx rgba(0,0,0,…)"`.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const TARGETS = [
  { path: ["shared", "ToggleSwitch.tsx"], hint: "ToggleSwitch" },
  { path: ["AgilePanel.tsx"], hint: "AgilePanel backdrop" },
];

function loadPanel(segments: string[]): string {
  return readFileSync(resolve(__dirname, "..", ...segments), "utf8");
}

/** Strip `var(--x, …rgba…)` fallbacks so only bare literals remain. */
function stripVarFallbacks(src: string): string {
  // Walk the string, find `var(`, then consume balanced parens.
  let out = "";
  let i = 0;
  while (i < src.length) {
    if (src.startsWith("var(", i)) {
      let depth = 1;
      i += 4;
      while (i < src.length && depth > 0) {
        if (src[i] === "(") depth++;
        else if (src[i] === ")") depth--;
        i++;
      }
      continue;
    }
    out += src[i];
    i++;
  }
  // Also strip code comments so rgba/boxShadow docs don't count.
  out = out.replace(/\/\/[^\n]*/g, "");
  out = out.replace(/\/\*[\s\S]*?\*\//g, "");
  return out;
}

describe("US-014 — overlay + elevation tokens replace inline rgba/boxShadow", () => {
  for (const { path, hint } of TARGETS) {
    it(`${hint}: no inline rgba(0,0,0,0.3…0.6) backgrounds`, () => {
      const src = stripVarFallbacks(loadPanel(path));
      const matches = src.match(/rgba\(\s*0\s*,\s*0\s*,\s*0\s*,\s*0\.[3-6][0-9]?\s*\)/g) ?? [];
      expect(
        matches.length,
        `Inline backdrop rgbas in ${hint}: ${matches.join(", ")}`,
      ).toBe(0);
    });

    it(`${hint}: no inline "0 Npx Mpx rgba(…)" boxShadow literals`, () => {
      const src = stripVarFallbacks(loadPanel(path));
      const matches =
        src.match(/boxShadow\s*:\s*["'`]\s*0\s+\d+px\s+\d+px[^"'`]*rgba[^"'`]*["'`]/g) ?? [];
      expect(
        matches.length,
        `Inline boxShadow literals in ${hint}: ${matches.join(" | ")}`,
      ).toBe(0);
    });
  }
});
