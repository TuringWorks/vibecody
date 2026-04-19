/**
 * Source-scan a11y regression test — US-017 (A-3 from 07-usability-improvements.md).
 *
 * Every `<label>` element in the named target panels must carry an
 * `htmlFor` attribute, and the referenced id must appear on an input,
 * select, or textarea in the same file. Missing label association is a
 * WCAG 1.3.1 / 3.3.2 failure — AT can't tell which label belongs to
 * which control.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const TARGETS = [
  "CloudAutofixPanel.tsx",
  "DocumentIngestPanel.tsx",
  "EnvPanel.tsx",
];

function loadPanel(name: string): string {
  return readFileSync(resolve(__dirname, "..", name), "utf8");
}

function extractIds(src: string): Set<string> {
  const ids = new Set<string>();
  const re = /\bid\s*=\s*["']([^"']+)["']/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(src)) !== null) ids.add(m[1]);
  return ids;
}

describe("US-017 — form labels are associated with their inputs", () => {
  for (const file of TARGETS) {
    it(`${file}: every <label> has htmlFor`, () => {
      const src = loadPanel(file);
      // Match opening <label ...> tag, allowing attributes to span lines.
      const re = /<label\b[\s\S]*?>/g;
      const labels = src.match(re) ?? [];
      expect(labels.length, `${file} has no <label> elements`).toBeGreaterThan(0);
      for (const tag of labels) {
        expect(
          /\bhtmlFor\s*=/.test(tag),
          `<label> in ${file} lacks htmlFor — controls must be associated for WCAG 1.3.1:\n  ${tag.slice(0, 160)}`,
        ).toBe(true);
      }
    });

    it(`${file}: every label htmlFor references an existing id`, () => {
      const src = loadPanel(file);
      const ids = extractIds(src);
      const re = /<label\b[^>]*?\bhtmlFor\s*=\s*["']([^"']+)["']/g;
      let m: RegExpExecArray | null;
      while ((m = re.exec(src)) !== null) {
        const target = m[1];
        expect(
          ids.has(target),
          `${file}: <label htmlFor="${target}"> has no matching id="${target}"`,
        ).toBe(true);
      }
    });
  }
});
