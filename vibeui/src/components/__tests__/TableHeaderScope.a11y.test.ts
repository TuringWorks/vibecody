/**
 * Source-scan a11y regression test — US-018 (A-5 from 07-usability-improvements.md).
 *
 * `<th>` elements must carry `scope="col"` (or `scope="row"` for row
 * headers) so assistive tech can associate data cells with their
 * headers. Missing `scope` is a WCAG 1.3.1 failure on any table with
 * two or more header columns.
 *
 * Named-target list to keep scope bounded; additional panels can be
 * migrated and added here one commit at a time.
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

describe("US-018 — <th> elements declare scope= for AT table navigation", () => {
  for (const file of TARGETS) {
    it(`${file}: every <th> has scope="col" or scope="row"`, () => {
      const src = loadPanel(file);
      // Opening <th ...> tag; attrs may span lines (rare but allow it).
      const re = /<th\b[\s\S]*?>/g;
      const ths = src.match(re) ?? [];
      expect(ths.length, `${file} has no <th> elements`).toBeGreaterThan(0);
      for (const tag of ths) {
        expect(
          /\bscope\s*=\s*["'](?:col|row|colgroup|rowgroup)["']/.test(tag),
          `<th> in ${file} lacks scope=:\n  ${tag.slice(0, 160)}`,
        ).toBe(true);
      }
    });
  }
});
