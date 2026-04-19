/**
 * Source-scan a11y regression test — US-008 (I-4 from 07-usability-improvements.md).
 *
 * WCAG 4.1.2: every `panel-tab-bar` container must carry `role="tablist"`
 * and every `panel-tab` button inside must carry `role="tab"` +
 * `aria-selected=`. Because migrating all 140+ panels in one story is
 * impractical, this test covers the representative batch migrated in
 * US-008. The list grows as future stories migrate more panels.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const FILES = [
  "A2aPanel.tsx",
  "AcpPanel.tsx",
  "AdminPanel.tsx",
  "AgentHostPanel.tsx",
  "AgentModesPanel.tsx",
  "AgentTeamPanel.tsx",
  "AgentTeamsPanel.tsx",
];

describe("US-008 — panel-tab-bar containers have ARIA tab roles", () => {
  for (const file of FILES) {
    it(`${file}: tablist role on panel-tab-bar containers`, () => {
      const path = resolve(__dirname, "..", file);
      const text = readFileSync(path, "utf8");

      // Find every `<div ... className="panel-tab-bar" ...>` tag start.
      // Uses a tempered greedy pattern ((?!<div)[\s\S])*? so the match
      // cannot span across other `<div` tags, and so `>` inside JSX
      // expressions (like `onClick={() => foo()}`) doesn't terminate
      // the match prematurely.
      const tabBarOpenings = [
        ...text.matchAll(
          /<div\b(?:(?!<div)[\s\S])*?className="[^"]*\bpanel-tab-bar\b[^"]*"(?:(?!<div)[\s\S])*?>/g,
        ),
      ];
      expect(tabBarOpenings.length, `expected panel-tab-bar in ${file}`).toBeGreaterThan(0);
      for (const match of tabBarOpenings) {
        expect(
          match[0],
          `panel-tab-bar without role="tablist" in ${file}: ${match[0]}`,
        ).toMatch(/role="tablist"/);
      }
    });

    it(`${file}: role=tab + aria-selected on panel-tab buttons`, () => {
      const path = resolve(__dirname, "..", file);
      const text = readFileSync(path, "utf8");

      // Match `<button ... className={`panel-tab${...}`}` (template literal)
      // and `<button ... className="panel-tab ..."` (plain string). Require
      // role="tab" and aria-selected= somewhere within the opening tag.
      // Tempered greedy ((?!<button)[\s\S])*? prevents a match from
      // spanning multiple buttons.
      const buttonOpenings = [
        ...text.matchAll(
          /<button\b(?:(?!<button)[\s\S])*?className=(?:\{`[^`]*\bpanel-tab(?!-)[^`]*`\}|"[^"]*\bpanel-tab(?!-)[^"]*")(?:(?!<button)[\s\S])*?>/g,
        ),
      ];
      expect(buttonOpenings.length, `expected panel-tab button in ${file}`).toBeGreaterThan(0);
      for (const match of buttonOpenings) {
        expect(
          match[0],
          `panel-tab button missing role="tab" in ${file}: ${match[0]}`,
        ).toMatch(/role="tab"/);
        expect(
          match[0],
          `panel-tab button missing aria-selected= in ${file}: ${match[0]}`,
        ).toMatch(/aria-selected=/);
      }
    });
  }
});
