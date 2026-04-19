/**
 * Source-scan regression test — US-024 (R-5 from 07-usability-improvements.md).
 *
 * The audit found empty states used ad-hoc centering (`marginTop: 32`,
 * `marginTop: "24px"`, …). R-5 asks for one shared `.panel-empty-state`
 * class with flex centering. This test locks in both halves of the fix:
 *
 *   1. The class is defined in App.css with flex-centering rules.
 *   2. Panels called out in the audit adopt the class.
 *
 * Current adopters:
 *   - DocumentIngestPanel.tsx  (audit line 171)
 *   - HistoryPanel.tsx         (audit line 91)
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

describe("US-024 — standard panel-empty-state class (R-5)", () => {
  it("App.css defines .panel-empty-state with flex centering", () => {
    const css = loadCss();
    // Capture the body of the first .panel-empty-state rule.
    const m = css.match(/\.panel-empty-state\s*\{([^}]*)\}/);
    expect(m, "App.css is missing .panel-empty-state rule").not.toBeNull();
    const body = (m as RegExpMatchArray)[1];
    expect(/display\s*:\s*flex\b/.test(body), "panel-empty-state lacks display:flex").toBe(true);
    expect(
      /align-items\s*:\s*center\b/.test(body),
      "panel-empty-state lacks align-items:center",
    ).toBe(true);
    expect(
      /justify-content\s*:\s*center\b/.test(body),
      "panel-empty-state lacks justify-content:center",
    ).toBe(true);
  });

  const ADOPTERS = ["DocumentIngestPanel.tsx", "HistoryPanel.tsx"] as const;
  for (const file of ADOPTERS) {
    it(`${file} uses className="panel-empty-state" for its empty state`, () => {
      const src = loadPanel(file);
      expect(
        /className\s*=\s*["']panel-empty-state["']/.test(src),
        `${file} does not reference className="panel-empty-state"`,
      ).toBe(true);
    });
  }
});
