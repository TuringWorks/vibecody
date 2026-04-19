/**
 * Source-scan a11y regression test — US-019 (I-5 from 07-usability-improvements.md).
 *
 * WCAG 2.5.8 Target Size (Minimum) requires interactive targets to be
 * at least 24×24 CSS pixels; App.css locks in the VibeCody floor at
 * `min-height: 28px` for `.panel-btn` and `32px` for primary actions.
 *
 * This test scans App.css to ensure those rules exist and have not
 * regressed (e.g. been deleted, reduced to < 28px, or commented out).
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

function loadCss(): string {
  return readFileSync(resolve(__dirname, "../../App.css"), "utf8");
}

function extractRuleBodies(css: string, selector: string): string[] {
  // Escape regex meta-chars in selector (., :, [, ], etc.).
  const esc = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  // Match `selector { ... }` without allowing selector to be a prefix
  // of something else (e.g. `.panel-btn` must not match `.panel-btn-primary`).
  // Collect EVERY matching rule block, since a selector can appear in
  // multiple places (base styles + usability overrides).
  const re = new RegExp(`${esc}(?![A-Za-z0-9_-])\\s*\\{([^}]*)\\}`, "g");
  const bodies: string[] = [];
  let m: RegExpExecArray | null;
  while ((m = re.exec(css)) !== null) bodies.push(m[1]);
  return bodies;
}

describe("US-019 — min click target size (WCAG 2.5.8)", () => {
  it(".panel-btn has min-height >= 28px somewhere in App.css", () => {
    const css = loadCss();
    const bodies = extractRuleBodies(css, ".panel-btn");
    expect(bodies.length, ".panel-btn rule not found in App.css").toBeGreaterThan(0);
    const heights = bodies
      .map((b) => /min-height\s*:\s*(\d+)px/.exec(b))
      .filter((m): m is RegExpExecArray => m !== null)
      .map((m) => Number(m[1]));
    expect(heights.length, ".panel-btn has no min-height declaration anywhere").toBeGreaterThan(0);
    expect(Math.max(...heights)).toBeGreaterThanOrEqual(28);
  });

  it(".panel-btn.panel-btn--primary has min-height >= 32px", () => {
    const css = loadCss();
    const bodies = extractRuleBodies(css, ".panel-btn.panel-btn--primary");
    expect(bodies.length, ".panel-btn--primary rule not found in App.css").toBeGreaterThan(0);
    const m = /min-height\s*:\s*(\d+)px/.exec(bodies[0]);
    expect(m, "primary button rule has no min-height").not.toBeNull();
    expect(Number(m![1])).toBeGreaterThanOrEqual(32);
  });

  it("Usability block header is present (regression guard)", () => {
    const css = loadCss();
    expect(
      css,
      "Usability block comment removed — min-height rules likely moved or deleted",
    ).toContain("Minimum click target sizes");
  });
});
