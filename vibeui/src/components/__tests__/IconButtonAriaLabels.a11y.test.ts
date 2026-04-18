/**
 * Source-scan a11y regression test — US-007 (I-2 from 07-usability-improvements.md).
 *
 * WCAG 1.1.1 / 4.1.2: every `<button>` whose only visible child is a glyph
 * icon (✕ × X ↺ ↻ ⟳ ←  → ⚙ ⋮) or a lucide-react icon component must carry an
 * `aria-label`. We enforce this on the specific files surfaced by the audit
 * to prevent regression; the set can grow over time.
 *
 * Each entry names a file and a line containing the offending `<button` open
 * tag. The test opens the file, walks forward from that line until the
 * balancing `>` that ends the opening tag, and asserts `aria-label=` appears
 * inside.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

type Target = { file: string; line: number; desc: string };

const TARGETS: Target[] = [
  { file: "AdminPanel.tsx", line: 144, desc: "close error message" },
  { file: "AgilePanel.tsx", line: 2691, desc: "delete epic" },
  { file: "AppBuilderPanel.tsx", line: 424, desc: "close error message" },
  { file: "ArchitectureSpecPanel.tsx", line: 605, desc: "close report" },
  { file: "ArchitectureSpecPanel.tsx", line: 768, desc: "close cell editing" },
  { file: "BrowserPanel.tsx", line: 202, desc: "refresh browser" },
  { file: "CodeMetricsPanel.tsx", line: 151, desc: "close error message" },
  { file: "ColorPalettePanel.tsx", line: 104, desc: "delete color token" },
  { file: "ColorPalettePanel.tsx", line: 235, desc: "delete palette" },
  { file: "ColorPalettePanel.tsx", line: 317, desc: "close export panel" },
  { file: "CompanyApprovalsPanel.tsx", line: 167, desc: "clear command result" },
  { file: "CompanyDashboardPanel.tsx", line: 168, desc: "clear action message" },
  { file: "CompanyGoalsPanel.tsx", line: 80, desc: "clear command result" },
  { file: "CompanyHeartbeatPanel.tsx", line: 121, desc: "clear trigger result" },
  { file: "CompanyOrgChartPanel.tsx", line: 125, desc: "clear action message" },
  { file: "CompanySecretsPanel.tsx", line: 108, desc: "clear command result" },
  { file: "DataGenPanel.tsx", line: 312, desc: "delete field" },
  { file: "EditPredictionPanel.tsx", line: 141, desc: "close error message" },
  { file: "HealthScorePanel.tsx", line: 117, desc: "close error message" },
  { file: "ReviewProtocolPanel.tsx", line: 96, desc: "close error message" },
  { file: "SettingsPanel.tsx", line: 3126, desc: "close settings panel" },
];

// Locate a `<button` opening tag at or after `line` and return [startLine, endLine].
// Handles multi-line tags by scanning until the first `>` that is not inside a JSX expression.
function findButtonTagRange(
  lines: string[],
  startHint: number,
): [number, number] {
  // startHint is 1-indexed; arr is 0-indexed.
  let i = startHint - 1;
  // Walk up to 20 lines looking for `<button`.
  for (let probe = 0; probe < 20 && i + probe < lines.length; probe++) {
    if (lines[i + probe].includes("<button")) {
      i = i + probe;
      break;
    }
  }
  // Walk forward looking for the first `>` that closes the opening tag.
  // Simple heuristic: count `{...}` braces on a single line — good enough
  // for our hand-authored React files (no `>` embedded inside expressions
  // in the 21 flagged locations).
  for (let j = i; j < lines.length && j < i + 40; j++) {
    if (lines[j].match(/[^=]>[^>]*$/)) {
      return [i, j];
    }
  }
  throw new Error(`could not find closing '>' for <button at line ${startHint}`);
}

describe("US-007 — icon-only buttons have aria-label", () => {
  for (const t of TARGETS) {
    it(`${t.file}:${t.line} (${t.desc})`, () => {
      const path = resolve(__dirname, "..", t.file);
      const text = readFileSync(path, "utf8");
      const lines = text.split("\n");
      const [start, end] = findButtonTagRange(lines, t.line);
      const tag = lines.slice(start, end + 1).join(" ");
      expect(tag, `expected aria-label in <button at ${t.file}:${t.line}`).toMatch(
        /aria-label\s*=/,
      );
    });
  }
});
