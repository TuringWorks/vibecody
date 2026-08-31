/**
 * Source-scan a11y regression test — US-007 (I-2 from 07-usability-improvements.md).
 *
 * WCAG 1.1.1 / 4.1.2: every `<button>` whose only visible child is a glyph
 * icon (✕ × X ↺ ↻ ⟳ ←  → ⚙ ⋮) or a lucide-react icon component must carry an
 * `aria-label`. We enforce this on the specific buttons surfaced by the audit
 * to prevent regression; the set can grow over time.
 *
 * Each entry names a file and the button's `onClick` handler. Targets used to
 * be pinned by line number, which made every unrelated edit above a target
 * report a false a11y regression — inserting one error banner moved three of
 * them and failed the suite on a button that had never lost its label. The
 * handler is what identifies the button, so that is what we anchor on.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

type Target = { file: string; handler: string; desc: string };

const TARGETS: Target[] = [
  { file: "AdminPanel.tsx", handler: "onClick={() => setError(null)}", desc: "close error message" },
  { file: "AgilePanel.tsx", handler: "onClick={() => removeEpic(epic.id)}", desc: "delete epic" },
  { file: "AppBuilderPanel.tsx", handler: "onClick={() => setErrorMsg(\"\")}", desc: "close error message" },
  { file: "ArchitectureSpecPanel.tsx", handler: "onClick={() => setReport(\"\")}", desc: "close report" },
  { file: "ArchitectureSpecPanel.tsx", handler: "onClick={() => setEditingCell(null)}", desc: "close cell editing" },
  { file: "BrowserPanel.tsx", handler: "onClick={refresh}", desc: "refresh browser" },
  { file: "CodeMetricsPanel.tsx", handler: "onClick={() => setError(null)}", desc: "close error message" },
  { file: "ColorPalettePanel.tsx", handler: "onClick={e => { e.stopPropagation(); onRemove(); }", desc: "delete color token" },
  { file: "ColorPalettePanel.tsx", handler: "onClick={e => { e.stopPropagation(); removePalette(p.id); }", desc: "delete palette" },
  { file: "ColorPalettePanel.tsx", handler: "onClick={() => setShowExport(false)}", desc: "close export panel" },
  { file: "CompanyApprovalsPanel.tsx", handler: "onClick={() => setCmdResult(null)}", desc: "clear command result" },
  { file: "CompanyDashboardPanel.tsx", handler: "onClick={() => setActionMsg(null)}", desc: "clear action message" },
  { file: "CompanyGoalsPanel.tsx", handler: "onClick={() => setCmdResult(null)}", desc: "clear command result" },
  { file: "CompanyHeartbeatPanel.tsx", handler: "onClick={() => setTriggerResult(null)}", desc: "clear trigger result" },
  { file: "CompanyOrgChartPanel.tsx", handler: "onClick={() => setActionMsg(null)}", desc: "clear action message" },
  { file: "CompanySecretsPanel.tsx", handler: "onClick={() => setCmdResult(null)}", desc: "clear command result" },
  { file: "DataGenPanel.tsx", handler: "onClick={() => removeField(f.id)}", desc: "delete field" },
  { file: "EditPredictionPanel.tsx", handler: "onClick={() => setError(null)}", desc: "close error message" },
  { file: "HealthScorePanel.tsx", handler: "onClick={() => setError(\"\")}", desc: "close error message" },
  { file: "ReviewProtocolPanel.tsx", handler: "onClick={() => setError(\"\")}", desc: "close error message" },
  { file: "SettingsPanel.tsx", handler: "onClick={onClose}", desc: "close settings panel" },
];

/**
 * Return the opening `<button ...>` tag containing `handler`.
 *
 * Walks back to the `<button` that opens the tag and forward to the `>` that
 * closes it, so a tag spread over several lines is returned whole.
 */
function buttonTagContaining(text: string, handler: string): string {
  const lines = text.split("\n");
  const hit = lines.findIndex((l) => l.includes(handler));
  if (hit === -1) {
    throw new Error(`no button with handler ${handler}`);
  }
  let start = hit;
  while (start > 0 && !lines[start].includes("<button")) {
    start--;
  }
  if (!lines[start].includes("<button")) {
    throw new Error(`no <button opening tag above handler ${handler}`);
  }
  for (let end = start; end < lines.length && end < start + 40; end++) {
    if (lines[end].match(/[^=]>[^>]*$/)) {
      return lines.slice(start, end + 1).join(" ");
    }
  }
  throw new Error(`could not find closing '>' for <button with ${handler}`);
}

describe("US-007 — icon-only buttons have aria-label", () => {
  for (const t of TARGETS) {
    it(`${t.file} (${t.desc})`, () => {
      const text = readFileSync(resolve(__dirname, "..", t.file), "utf8");
      const tag = buttonTagContaining(text, t.handler);
      expect(tag, `expected aria-label on the ${t.desc} button in ${t.file}`).toMatch(
        /aria-label\s*=/,
      );
    });
  }
});
