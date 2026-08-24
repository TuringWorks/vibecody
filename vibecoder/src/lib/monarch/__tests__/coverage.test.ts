/**
 * The invariant that was silently broken: **every language id the editor can
 * assign must be one Monaco knows.**
 *
 * `detectLanguage` returned ids like `matlab`, `cobol`, `sas` and `cmake` that
 * were never registered. Monaco rejects an unknown id, so those files got no
 * highlighting *and* no IntelliSense — the LSP providers are keyed on the same
 * id. Nothing failed; the file just rendered grey.
 *
 * Monaco's real id list is read from the shipped `basic-languages` contribution
 * files rather than hardcoded, so a Monaco upgrade that adds or renames a
 * language is caught here instead of at runtime.
 */

import { describe, it, expect } from "vitest";
import { readFileSync, readdirSync, existsSync } from "node:fs";
import { join } from "node:path";
import { MONARCH_LANGUAGES } from "../languages";

const packageRoot = join(__dirname, "..", "..", "..", "..");

/**
 * Where Monaco keeps one directory per language. It moved in 0.56 —
 * `basic-languages/<id>/<id>.contribution.js` became
 * `languages/definitions/<id>/register.js` — so both layouts are read.
 */
const MONACO_LANGUAGE_LAYOUTS = [
  { dir: ["languages", "definitions"], file: (id: string) => `register.js` },
  { dir: ["basic-languages"], file: (id: string) => `${id}.contribution.js` },
] as const;

/** Language ids Monaco ships, read from its own contribution files. */
function monacoLanguageIds(): Set<string> {
  const monacoRoot = join(
    packageRoot,
    "node_modules",
    "monaco-editor",
    "esm",
    "vs",
  );
  const ids = new Set<string>([
    // The worker-backed languages live outside the per-language directories.
    "typescript", "javascript", "json", "css", "scss", "less", "html",
    "handlebars", "razor", "plaintext",
  ]);
  let found = 0;
  for (const layout of MONACO_LANGUAGE_LAYOUTS) {
    const base = join(monacoRoot, ...layout.dir);
    if (!existsSync(base)) continue;
    for (const dir of readdirSync(base)) {
      const contribution = join(base, dir, layout.file(dir));
      if (!existsSync(contribution)) continue;
      const source = readFileSync(contribution, "utf8");
      for (const match of source.matchAll(/id:\s*["']([^"']+)["']/g)) {
        ids.add(match[1]);
        found += 1;
      }
    }
  }
  // Reading zero definitions means Monaco moved its files again, not that it
  // ships no languages. Returning the ten hardcoded ids would let this test
  // report "10 is not greater than 50" and send the next reader hunting
  // through `languages.ts` for a bug that is not there.
  if (found === 0) {
    throw new Error(
      `No Monaco language definitions found under ${MONACO_LANGUAGE_LAYOUTS.map(
        (l) => join(...l.dir),
      ).join(" or ")} — Monaco changed its layout again; update MONACO_LANGUAGE_LAYOUTS.`,
    );
  }
  return ids;
}

/** Every value `detectLanguage` can return, read from its lookup table. */
function detectLanguageIds(): Set<string> {
  const source = readFileSync(
    join(packageRoot, "src", "utils", "fileUtils.tsx"),
    "utf8",
  );
  const table = source
    .split("const languageMap: Record<string, string> = {")[1]
    .split("\n    };")[0];
  const ids = new Set<string>();
  for (const match of table.matchAll(/:\s*'([^']+)'/g)) ids.add(match[1]);
  return ids;
}

describe("language id coverage", () => {
  const monaco = monacoLanguageIds();
  const ours = new Set(MONARCH_LANGUAGES.map((spec) => spec.id));

  it("reads a plausible number of ids from Monaco", () => {
    // Guards the parsing above: if the contribution format changes and this
    // returns nothing, every assertion below would pass vacuously.
    expect(monaco.size).toBeGreaterThan(50);
    expect(monaco.has("cpp")).toBe(true);
    expect(monaco.has("python")).toBe(true);
  });

  it("every language detectLanguage can assign is registered somewhere", () => {
    const orphans = [...detectLanguageIds()]
      .filter((id) => !monaco.has(id) && !ours.has(id))
      .sort();
    expect(
      orphans,
      "these ids render as grey text with no IntelliSense — add a spec to " +
        "lib/monarch/languages.ts or map the extension to a language Monaco has",
    ).toEqual([]);
  });

  it("does not shadow a language Monaco already ships", () => {
    // Monaco's own grammars are better maintained than ours; if it gains one of
    // these, ours should be deleted rather than left to rot.
    const shadowed = [...ours].filter((id) => monaco.has(id)).sort();
    expect(shadowed).toEqual([]);
  });

  it("supplies a grammar for every language server we route to", () => {
    // The other direction: a language with a server but no grammar is exactly
    // the "IntelliSense works, file is grey" state we set out to fix.
    const lspSource = readFileSync(
      join(packageRoot, "src", "lib", "lsp.ts"),
      "utf8",
    );
    const extensionTable = lspSource
      .split("const EXTENSION_LANGUAGE: Record<string, string> = {")[1]
      .split("\n};")[0];
    const detect = detectLanguageIds();

    // Every extension routed to a server should also be an extension whose
    // Monaco language is known. We check via detectLanguage's own table.
    const fileUtils = readFileSync(
      join(packageRoot, "src", "utils", "fileUtils.tsx"),
      "utf8",
    );
    const detectTable = fileUtils
      .split("const languageMap: Record<string, string> = {")[1]
      .split("\n    };")[0];
    const detectMap = new Map<string, string>();
    for (const match of detectTable.matchAll(/'([^']+)':\s*'([^']+)'/g)) {
      detectMap.set(match[1], match[2]);
    }

    const unhighlighted: string[] = [];
    for (const match of extensionTable.matchAll(/^\s*([A-Za-z0-9_]+):\s*"([^"]+)",/gm)) {
      const [, extension, lspLanguage] = match;
      const monacoLanguage = detectMap.get(extension);
      if (monacoLanguage === undefined) {
        unhighlighted.push(`.${extension} (→ ${lspLanguage}): no detectLanguage entry`);
      } else if (!monaco.has(monacoLanguage) && !ours.has(monacoLanguage)) {
        unhighlighted.push(`.${extension} (→ ${lspLanguage}): "${monacoLanguage}" unregistered`);
      }
    }
    expect(unhighlighted, "server-backed files that would render grey").toEqual([]);
    expect(detect.size).toBeGreaterThan(40);
  });

  it("registers the languages people asked about with their own grammar", () => {
    // Zig/Nim/Crystal/D/V used to borrow C++/Python/Ruby highlighting, which
    // mis-coloured the keywords each language actually has.
    for (const id of ["zig", "nim", "crystal", "d", "v", "vala", "odin", "gleam"]) {
      expect(ours.has(id), `${id} has no grammar`).toBe(true);
    }
  });
});
