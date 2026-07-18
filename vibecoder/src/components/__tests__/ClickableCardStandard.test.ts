/**
 * Source-scan regression test — US-025 (I-7 from 07-usability-improvements.md).
 *
 * Clickable cards in different panels used to implement hover in three
 * different ways: JS `onMouseEnter` mutating inline style, inline
 * `transition` with no state, or nothing at all. The audit prescribed
 * a single shared `.panel-card--clickable` class with `:hover` and
 * `:active` rules. This test locks the convention in:
 *
 *   1. App.css defines .panel-card--clickable + :hover + :active.
 *   2. Panels called out in the audit adopt the class.
 *
 * Current adopters:
 *   - SpecPanel.tsx       (audit I-7 line 197)
 *   - HistoryPanel.tsx    (audit I-7 line 199)
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const REPO = resolve(__dirname, "..", "..");

function loadCss(): string {
  return readFileSync(resolve(REPO, "App.css"), "utf8");
}
function loadPanel(name: string): string {
  return readFileSync(resolve(__dirname, "..", name), "utf8");
}

describe("US-025 — standard panel-card--clickable class (I-7)", () => {
  it("App.css defines .panel-card--clickable plus :hover and :active", () => {
    const css = loadCss();
    expect(
      /\.panel-card--clickable\s*\{[^}]*cursor\s*:\s*pointer/.test(css),
      "App.css .panel-card--clickable missing cursor:pointer",
    ).toBe(true);
    expect(
      /\.panel-card--clickable:hover\s*\{[^}]*background\s*:/.test(css),
      "App.css .panel-card--clickable:hover missing background rule",
    ).toBe(true);
    expect(
      /\.panel-card--clickable:active\s*\{[^}]*background\s*:/.test(css),
      "App.css .panel-card--clickable:active missing background rule",
    ).toBe(true);
  });

  const ADOPTERS = ["SpecPanel.tsx", "HistoryPanel.tsx"] as const;
  for (const file of ADOPTERS) {
    it(`${file} references className "panel-card--clickable"`, () => {
      const src = loadPanel(file);
      expect(
        /panel-card--clickable/.test(src),
        `${file} does not reference panel-card--clickable`,
      ).toBe(true);
    });
  }

  it("SpecPanel.tsx no longer uses inline onMouseEnter-to-mutate-background hack", () => {
    const src = loadPanel("SpecPanel.tsx");
    // The specific pre-fix pattern: onMouseEnter callback that assigns to
    // currentTarget.style.background. The lock-in fails if it creeps back.
    expect(
      /onMouseEnter\s*=\s*\{[^}]*currentTarget\.style\.background/.test(src),
      "SpecPanel.tsx still mutates background via onMouseEnter — migrate to panel-card--clickable",
    ).toBe(false);
  });
});
