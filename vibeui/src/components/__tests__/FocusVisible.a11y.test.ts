/**
 * Source-scan a11y regression test — US-009 (I-1 from 07-usability-improvements.md).
 *
 * WCAG 2.4.7: every interactive element must expose a visible focus
 * indicator when reached via keyboard. VibeUI's `.panel-input:focus`
 * previously set `outline: none` with no replacement indicator, and
 * `.panel-btn` / `.panel-tab` had no focus style at all.
 *
 * This test asserts that App.css defines `:focus-visible` rules for the
 * global interactive classes and that they apply a non-empty outline.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const CSS_PATH = resolve(__dirname, "..", "..", "App.css");
const text = () => readFileSync(CSS_PATH, "utf8");

function extractRuleBody(css: string, selectorSubstr: string): string | null {
  // Find `<anything containing selectorSubstr>:focus-visible { ... }` and
  // return the body between braces.
  const re = new RegExp(
    `([^}]*\\b${selectorSubstr}[^}]*:focus-visible[^{]*)\\{([^}]*)\\}`,
  );
  const m = css.match(re);
  return m ? m[2] : null;
}

describe("US-009 — global interactive elements have :focus-visible outlines", () => {
  for (const sel of ["panel-btn", "panel-input", "panel-select", "panel-tab"]) {
    it(`.${sel}:focus-visible defines a visible outline`, () => {
      const body = extractRuleBody(text(), sel);
      expect(body, `expected .${sel}:focus-visible rule in App.css`).not.toBeNull();
      // Outline must be set and non-none. Width >= 2px per WCAG guidance.
      expect(body!, `expected outline on .${sel}:focus-visible`).toMatch(
        /outline\s*:\s*(?!none)/,
      );
      expect(body!).toMatch(/outline-offset\s*:/);
    });
  }

  it(".panel-input:focus no longer sets `outline: none` without a replacement", () => {
    // The old rule was `.panel-input:focus { border-color: ...; outline: none; }`
    // which removed the focus ring entirely. The new contract is: either
    // keep the outline, or pair `outline: none` with a `:focus-visible`
    // rule that supplies one. The `:focus-visible` rule check above is
    // the guarantee; here we just assert the `:focus` rule alone never
    // eats the outline without a peer `:focus-visible` to replace it.
    const css = text();
    // Locate the `.panel-input:focus {` rule.
    const focusMatch = css.match(/\.panel-input:focus\s*\{([^}]*)\}/);
    if (focusMatch && /outline\s*:\s*none/.test(focusMatch[1])) {
      // If it sets outline:none, a :focus-visible peer must exist.
      expect(css).toMatch(/\.panel-input[^{]*:focus-visible[^{]*\{[^}]*outline/);
    }
  });
});
