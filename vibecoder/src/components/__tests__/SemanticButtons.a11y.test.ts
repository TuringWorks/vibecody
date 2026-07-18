/**
 * Source-scan a11y regression test — US-011 (A-1 from 07-usability-improvements.md).
 *
 * WCAG 4.1.2 (Name, Role, Value): interactive elements must expose a
 * correct semantic role. A `<div onClick={...}>` that *behaves* like a
 * button is not keyboard-reachable by default, not announced as a
 * button by AT, and has no built-in `Enter`/`Space` activation.
 *
 * Scope here is narrow — migrate panels one at a time rather than a
 * big-bang rewrite, because many div-onClick patterns are legitimate
 * (overlay backdrop dismissal, event-propagation blockers). Each
 * migration gets an entry in TARGETS. When all listed targets are
 * clean, expand the list.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

interface Target {
  file: string;
  /** Substring that identified the old div-onClick. Used in the error msg. */
  hint: string;
}

const TARGETS: Target[] = [
  // AIChat tool-call cards — clickable expand/collapse on the header.
  { file: "AIChat.tsx", hint: "tool-card-header" },
];

function loadPanel(name: string): string {
  return readFileSync(resolve(__dirname, "..", name), "utf8");
}

describe("US-011 — interactive click targets use <button>, not <div>", () => {
  for (const { file, hint } of TARGETS) {
    it(`${file}: no <div …onClick=…> for "${hint}"`, () => {
      const src = loadPanel(file);
      // Match only within a single opening div tag: `<div …${hint}…onClick=…>`
      // where `…` cannot contain `>` (so we don't escape into nested JSX).
      const re = new RegExp(
        `<div\\b[^>]*\\b${hint}\\b[^>]*onClick=|<div\\b[^>]*onClick=[^>]*\\b${hint}\\b`,
      );
      expect(
        re.test(src),
        `${file} still has <div …${hint}… onClick=…>; migrate to <button>.`,
      ).toBe(false);
    });
  }
});
