/**
 * Guards the monaco-editor pin against a silent dependency bump.
 *
 * A blanket dependency update once moved monaco 0.55.1 → 0.56.0. Three things
 * broke and nothing noticed:
 *
 *   1. 0.56 changed its package `exports` map from an identity mapping
 *      (`"./*": "./*"`) to one that rebases every subpath under `esm/vs`
 *      (`"./*": "./esm/vs/*.js"`). Under that, `monaco-editor/esm/vs/x`
 *      resolves to `esm/vs/esm/vs/x.js` — so every `esm/vs/**` import,
 *      including the five language workers in `monaco-setup.ts`, stopped
 *      resolving even though the files were still on disk. `vite build`
 *      failed; the editor would have shipped stuck on "Loading...".
 *      (On 0.56 the correct form drops the prefix: `monaco-editor/editor/…`.)
 *   2. `patches/monaco-editor+0.55.1.patch` no longer applied, silently
 *      dropping a crash guard in the diff editor's view-zone computation.
 *   3. `tsc --noEmit` and the whole vitest suite stayed green, because tsc
 *      resolves through the filesystem and no test imports `monaco-setup.ts`.
 *
 * These assertions are cheap and run with every suite. CI also builds the
 * bundle now (`vibecoder-checks` → `build`), which catches (1) directly.
 */

import { describe, it, expect } from "vitest";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

/** Repo-relative to this file: `src/__tests__` → `vibecoder/`. */
const packageRoot = join(__dirname, "..", "..");

const readJson = (path: string): Record<string, unknown> =>
  JSON.parse(readFileSync(path, "utf8")) as Record<string, unknown>;

const installedMonacoVersion = (): string => {
  const pkg = readJson(
    join(packageRoot, "node_modules", "monaco-editor", "package.json"),
  );
  return String(pkg.version);
};

describe("monaco-editor pin", () => {
  it("maps subpaths through `exports` unchanged, so `esm/vs/**` resolves", () => {
    // The exact mechanism that broke the bundle. `monaco-setup.ts` imports the
    // language workers by their on-disk path, which only works while the
    // exports map is the identity. A rebasing map makes them unresolvable
    // however present the files are.
    const pkg = readJson(
      join(packageRoot, "node_modules", "monaco-editor", "package.json"),
    );
    const wildcard = (pkg.exports as Record<string, unknown> | undefined)?.["./*"];
    expect(
      wildcard,
      `monaco-editor ${installedMonacoVersion()} maps "./*" to ${JSON.stringify(wildcard)} ` +
        'instead of "./*". Every `monaco-editor/esm/vs/**` import in src/ must be ' +
        "rewritten to the exports-relative form (drop the `esm/vs/` prefix) before " +
        "this version can be used — and verified with `npx vite build`, not just tsc.",
    ).toBe("./*");
  });

  it("is the version its patch was written for", () => {
    // patch-package matches on the exact version in the filename. A bump
    // leaves the patch silently unapplied — `npm install` warns and moves on.
    const patches = readdirSync(join(packageRoot, "patches")).filter((name) =>
      name.startsWith("monaco-editor+"),
    );
    expect(patches, "expected exactly one monaco-editor patch").toHaveLength(1);

    const patchedVersion = patches[0].replace(/^monaco-editor\+/, "").replace(/\.patch$/, "");
    expect(
      installedMonacoVersion(),
      `patches/${patches[0]} does not apply to the installed monaco-editor. ` +
        "Re-create it against the new version (and confirm the diff-editor " +
        "crash guard it carries is still needed) before bumping.",
    ).toBe(patchedVersion);
  });

  it("pins the 0.55.x line in package.json", () => {
    const pkg = readJson(join(packageRoot, "package.json"));
    const declared = (pkg.devDependencies as Record<string, string>)[
      "monaco-editor"
    ];
    // `^0.55.1` cannot reach 0.56 (caret on a 0.x minor is bounded), so the
    // range itself is the pin. Moving it is a deliberate upgrade with the
    // checklist in this file's header, not a routine `npm audit fix`.
    expect(declared).toBe("^0.55.1");
  });
});
