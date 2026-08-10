/**
 * Source-scan regression test — no phantom design tokens.
 *
 * `var(--foo)` where `--foo` is never defined does not warn, does not throw,
 * and does not show up in a type-check. The declaration is simply dropped, so
 * the element renders with the inherited or initial value. A background
 * silently becomes transparent; a border silently disappears.
 *
 * That is how the Sandbox panel's "Open Sandbox Folder" button broke: it used
 * `var(--accent)` (never defined anywhere — the real token is
 * `--accent-color`) for its background, and paired it with
 * `--btn-primary-fg`, which `themes.ts` computes as the best contrast *against
 * the accent fill*. On any theme with a bright accent that is `#000000`. Black
 * text, on a fill that never painted, over a dark panel.
 *
 * A `var(--foo, fallback)` use is not a violation — the fallback renders, and
 * the token is optional by construction. Only bare uses are checked.
 *
 * KNOWN_UNDEFINED is debt, not permission. It is an explicit inventory of
 * tokens that were already phantom when this test was written, kept here so
 * the list is visible in the repo instead of only in a terminal. Shrink it;
 * never grow it.
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { describe, expect, it } from "vitest";

const SRC = resolve(__dirname, "..", "..");
const DESIGN_SYSTEM = resolve(SRC, "..", "design-system");

/**
 * Tokens referenced without a fallback that no stylesheet defines, as of the
 * `--accent` fix. Each one renders as transparent / inherited / initial today.
 *
 * `--accent` is deliberately absent: it was the 23rd entry, and removing it is
 * what this test exists to keep true.
 */
const KNOWN_UNDEFINED = new Set([
  "--bg-default",
  "--bg-selected",
  "--bg-subtle",
  "--btn-primary-bg",
  "--error",
  "--error-fg",
  "--success",
  "--success-fg",
  "--warning",
  "--text-on-accent",
  "--text-warning-alt",
  "--accent-primary-10",
  "--icon-color",
  "--font-sans",
  "--spacing-xs",
  "--spacing-sm",
  "--spacing-md",
  "--spacing-lg",
  "--spacing-xl",
]);

const EXTENSIONS = [".ts", ".tsx", ".css"];

function walk(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    const path = join(dir, entry);
    if (entry === "node_modules" || entry === "dist") return [];
    if (statSync(path).isDirectory()) return walk(path);
    return EXTENSIONS.some((e) => path.endsWith(e)) ? [path] : [];
  });
}

const files = [...walk(SRC), ...walk(DESIGN_SYSTEM)];
const sources = new Map(files.map((f) => [f, readFileSync(f, "utf8")]));

/** Every token this codebase defines, in CSS or in the themes.ts var maps. */
const defined = new Set<string>();
for (const text of sources.values()) {
  for (const m of text.matchAll(/(--[a-zA-Z0-9-]+)\s*:/g)) defined.add(m[1]);
  for (const m of text.matchAll(/["'](--[a-zA-Z0-9-]+)["']\s*:/g)) defined.add(m[1]);
}

/** Bare `var(--x)` uses — no fallback, so an undefined token renders as nothing. */
function bareUses(text: string): string[] {
  return [...text.matchAll(/var\(\s*(--[a-zA-Z0-9-]+)\s*\)/g)].map((m) => m[1]);
}

describe("design tokens", () => {
  it("defines a large token set (guards against the scan finding nothing)", () => {
    // If the walk or the regex silently broke, `defined` would be tiny and
    // every assertion below would pass for the wrong reason.
    expect(defined.size).toBeGreaterThan(80);
    expect(defined.has("--accent-color")).toBe(true);
    expect(defined.has("--btn-primary-fg")).toBe(true);
  });

  it("never references `--accent` — the real token is `--accent-color`", () => {
    const offenders = [...sources.entries()]
      .filter(([file]) => !file.endsWith("DefinedTokensOnly.test.ts"))
      .filter(([, text]) => /var\(\s*--accent\s*[),]/.test(text))
      .map(([file]) => file.replace(SRC, "src"));

    expect(offenders, `use var(--accent-color); \`--accent\` is not defined`).toEqual([]);
  });

  it("introduces no new phantom tokens", () => {
    const offenders = [...sources.entries()]
      .filter(([file]) => !file.endsWith("DefinedTokensOnly.test.ts"))
      .flatMap(([file, text]) =>
        bareUses(text)
          .filter((token) => !defined.has(token) && !KNOWN_UNDEFINED.has(token))
          .map((token) => `${file.replace(SRC, "src")}: var(${token})`),
      );

    expect(
      [...new Set(offenders)],
      "these tokens are never defined, so the declaration is dropped and the " +
        "property falls back to inherited/initial — define the token or use an existing one",
    ).toEqual([]);
  });

  it("keeps KNOWN_UNDEFINED honest — entries that got defined must be removed", () => {
    const nowDefined = [...KNOWN_UNDEFINED].filter((t) => defined.has(t));
    expect(nowDefined, "these are defined now; drop them from KNOWN_UNDEFINED").toEqual([]);
  });

  it("keeps KNOWN_UNDEFINED honest — entries nobody references any more must be removed", () => {
    // Without this the list silently accretes tokens that were fixed long ago,
    // and stops being a usable inventory of what is actually broken.
    const stillUsed = new Set(
      [...sources.entries()]
        .filter(([file]) => !file.endsWith("DefinedTokensOnly.test.ts"))
        .flatMap(([, text]) => bareUses(text)),
    );
    const stale = [...KNOWN_UNDEFINED].filter((t) => !stillUsed.has(t));
    expect(stale, "no longer referenced; drop them from KNOWN_UNDEFINED").toEqual([]);
  });
});
