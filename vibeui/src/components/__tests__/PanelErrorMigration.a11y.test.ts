/**
 * Source-scan regression test — US-015 (A-2).
 *
 * Named-target list of panels that have been migrated from
 * `<div className="panel-error">…</div>` to the `<PanelError>`
 * primitive. If someone reverts to the plain-div pattern, this test
 * fails, reminding them the container must be `role="alert"` for
 * screen-reader announcement.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const TARGETS = [
  "AgentModesPanel.tsx",
  "AgentTeamsPanel.tsx",
];

function loadPanel(name: string): string {
  return readFileSync(resolve(__dirname, "..", name), "utf8");
}

describe("US-015 — migrated panels use PanelError (not raw panel-error div)", () => {
  for (const file of TARGETS) {
    it(`${file}: no raw <div className="panel-error">`, () => {
      const src = loadPanel(file);
      // Allow `className="panel-error"` inside the PanelError primitive
      // itself, but these panels should not contain the raw literal.
      expect(
        src.includes('className="panel-error"'),
        `${file} reverted to <div className="panel-error"> — use <PanelError> for role=alert semantics`,
      ).toBe(false);
    });

    it(`${file}: imports PanelError from shared`, () => {
      const src = loadPanel(file);
      expect(src).toMatch(/import\s*\{\s*PanelError\s*\}\s*from\s*["']\.\/shared\/PanelError["']/);
    });
  }
});
