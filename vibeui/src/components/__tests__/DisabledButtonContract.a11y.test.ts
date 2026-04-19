/**
 * Source-scan a11y regression test — US-010 (I-3 from 07-usability-improvements.md).
 *
 * The usability audit flagged inconsistent disabled-button styling:
 * some panels set inline `opacity: 0.5` / `cursor: not-allowed`, others
 * rely on implicit browser defaults, and a few accidentally override
 * the disabled visual by hard-coding `opacity: 1` in inline styles.
 *
 * The contract is a single global rule in App.css:
 *
 *   button:disabled, .panel-btn:disabled {
 *     opacity: <dim>;
 *     cursor: not-allowed;
 *     pointer-events: none;
 *   }
 *
 * `pointer-events: none` is the non-obvious invariant — without it,
 * `:hover` / custom click handlers fire on disabled buttons on some
 * browsers. This test locks the contract so future edits can't silently
 * weaken it.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const CSS_PATH = resolve(__dirname, "..", "..", "App.css");
const css = () => readFileSync(CSS_PATH, "utf8");

function findDisabledRuleBody(text: string): string | null {
  // Grab any rule whose selector list contains `:disabled` and mentions
  // either `button` or `.panel-btn`.
  const re =
    /(button:disabled[^{]*|\.panel-btn:disabled[^{]*|button:disabled\s*,\s*\.panel-btn:disabled[^{]*)\{([^}]*)\}/;
  const m = text.match(re);
  return m ? m[2] : null;
}

describe("US-010 — standardized disabled button CSS", () => {
  it("App.css defines a global button:disabled / .panel-btn:disabled rule", () => {
    const body = findDisabledRuleBody(css());
    expect(body, "expected button:disabled rule in App.css").not.toBeNull();
  });

  it("disabled rule sets opacity to a dimmed value (< 1)", () => {
    const body = findDisabledRuleBody(css())!;
    const m = body.match(/opacity\s*:\s*([0-9.]+)/);
    expect(m, "expected opacity declaration in disabled rule").not.toBeNull();
    expect(parseFloat(m![1])).toBeLessThan(1);
  });

  it("disabled rule forbids interaction via cursor + pointer-events", () => {
    const body = findDisabledRuleBody(css())!;
    expect(body).toMatch(/cursor\s*:\s*not-allowed/);
    expect(body).toMatch(/pointer-events\s*:\s*none/);
  });
});
