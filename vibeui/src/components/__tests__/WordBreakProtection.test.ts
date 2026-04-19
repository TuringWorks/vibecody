/**
 * Source-scan regression test — US-022 (R-3 from 07-usability-improvements.md).
 *
 * Table cells and `<pre>` blocks that hold dynamic strings (URLs,
 * column values, log lines) must carry `wordBreak: "break-word"` so an
 * unbroken 200-char identifier doesn't push the column off-screen.
 *
 * The named target panels were called out in the audit; the test
 * enforces the fix stays in place:
 *   - CsvPanel: `<td>` cells with maxWidth + whiteSpace:"nowrap" must
 *     also carry `wordBreak: "break-word"`
 *   - ArenaPanel: `<pre>` block must carry `wordBreak: "break-word"`
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

function loadPanel(name: string): string {
  return readFileSync(resolve(__dirname, "..", name), "utf8");
}

describe("US-022 — word-break protection on dynamic-string cells (R-3)", () => {
  it("CsvPanel.tsx: data-cell <td> declares wordBreak:break-word", () => {
    const src = loadPanel("CsvPanel.tsx");
    // Find every <td ...> opening tag that also contains maxWidth (the
    // narrow-column protected cells). Each must carry wordBreak:"break-word".
    // [\s\S]*? allows attribute list to span newlines (td tags in CsvPanel
    // are formatted across 2 lines: opening attrs then style).
    const re = /<td\b[\s\S]*?\bmaxWidth\b[\s\S]*?>/g;
    const cells = src.match(re) ?? [];
    expect(cells.length, "CsvPanel.tsx has no <td maxWidth=...> cells — audit pattern moved?").toBeGreaterThan(0);
    for (const tag of cells) {
      expect(
        /\bwordBreak\s*:\s*["']break-word["']/.test(tag),
        `CsvPanel.tsx <td> with maxWidth missing wordBreak:break-word:\n  ${tag.slice(0, 200)}`,
      ).toBe(true);
    }
  });

  it("ArenaPanel.tsx: <pre> declares wordBreak:break-word", () => {
    const src = loadPanel("ArenaPanel.tsx");
    const re = /<pre\b[\s\S]*?>/g;
    const pres = src.match(re) ?? [];
    expect(pres.length, "ArenaPanel.tsx has no <pre> — audit pattern moved?").toBeGreaterThan(0);
    for (const tag of pres) {
      expect(
        /\bwordBreak\s*:\s*["']break-word["']/.test(tag),
        `ArenaPanel.tsx <pre> missing wordBreak:break-word:\n  ${tag.slice(0, 200)}`,
      ).toBe(true);
    }
  });
});
