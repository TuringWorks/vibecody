/**
 * Source-scan responsive-layout regression test — US-020
 * (R-1 from 07-usability-improvements.md).
 *
 * Every `<table>` in the named target panels must sit inside a wrapper
 * element with `overflowX: "auto"` (or the `.panel-table-wrapper`
 * helper class). Otherwise the table overflows its container at narrow
 * viewports and horizontal content is unreachable.
 *
 * Heuristic: split the source on `<table\b`. For every table, the
 * preceding ~220 chars must contain an `overflowX: "auto"` signal.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const TARGETS = [
  "AgentModesPanel.tsx",
  "AgilePanel.tsx",
  "BatchBuilderPanel.tsx",
];

function loadPanel(name: string): string {
  return readFileSync(resolve(__dirname, "..", name), "utf8");
}

const OVERFLOW_SIGNAL =
  /overflowX\s*:\s*["']auto["']|overflow-x\s*:\s*auto|panel-table-wrapper/;

describe("US-020 — tables live inside a horizontal-overflow wrapper", () => {
  for (const file of TARGETS) {
    it(`${file}: every <table> is wrapped in overflowX:"auto"`, () => {
      const src = loadPanel(file);
      const parts = src.split(/<table\b/);
      const tableCount = parts.length - 1;
      expect(tableCount, `${file} has no <table> elements`).toBeGreaterThan(0);
      for (let i = 1; i < parts.length; i++) {
        const preceding = parts[i - 1].slice(-220);
        expect(
          OVERFLOW_SIGNAL.test(preceding),
          `<table> #${i} in ${file} not wrapped in overflowX:"auto". Preceding context:\n${preceding}`,
        ).toBe(true);
      }
    });
  }
});
