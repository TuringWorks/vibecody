/**
 * Source-scan regression test — every Monaco instance escapes the editor clip.
 *
 * Monaco renders hovers, the suggest widget and the context menu as overflow
 * widgets inside its own DOM. `.editor-container` is `overflow: hidden`
 * (App.css), so any widget wider than the editor pane is clipped rather than
 * repositioned — a diagnostic hover comes out sliced mid-word:
 *
 *     Cannot find module 'child_process'. Did you me…
 *
 * The text has already wrapped to the hover's own width by then, which is what
 * makes it deceptive: it reads as a message that ends there, not one that was
 * cut, and the half naming the fix is the half that is gone.
 *
 * Nothing catches this automatically. It type-checks, it renders, and jsdom has
 * no layout — so a test that mounts an editor cannot see it either. Only
 * opening a file with a long diagnostic near the right edge shows it, which is
 * why the guard is a source scan: it asserts the *option is passed*, which is
 * the thing a new editor is likely to forget.
 *
 * It does not assert the widgets are visible. `position: fixed` still resolves
 * against an ancestor that establishes a containing block (`transform`,
 * `filter`, `backdrop-filter`, `perspective`, `will-change`), so adding one of
 * those to an editor ancestor would reintroduce the clip with this test still
 * green. See `lib/monacoOptions.ts`.
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { describe, expect, it } from "vitest";

const SRC = resolve(__dirname, "..", "..");

/** Every `.tsx` under `src/`, tests excluded. */
function sourceFiles(dir: string, out: string[] = []): string[] {
  for (const entry of readdirSync(dir)) {
    if (entry === "node_modules" || entry === "__tests__") continue;
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) sourceFiles(full, out);
    else if (entry.endsWith(".tsx")) out.push(full);
  }
  return out;
}

/**
 * Strip comments so the scan reads code, not prose about code.
 *
 * `main.tsx` opens with a comment explaining why `monaco-setup` must run
 * "before anything renders <Editor>", and names `@monaco-editor/react` in the
 * same sentence — so every cheaper filter matched it, and the guard reported a
 * file that mounts nothing.
 */
function code(text: string): string {
  return text.replace(/\/\*[\s\S]*?\*\//g, "").replace(/^\s*\/\/.*$/gm, "");
}

/** Files that mount a Monaco editor. */
function monacoMounts(): { file: string; text: string }[] {
  return sourceFiles(SRC)
    .map((file) => ({ file, text: readFileSync(file, "utf8") }))
    .filter(({ text }) => /<(Editor|DiffEditor)\b/.test(code(text)));
}

describe("Monaco overflow widgets", () => {
  it("finds the editors it is meant to be guarding", () => {
    // A scan that silently matches nothing is a passing test that checks
    // nothing — the failure mode this whole file exists to avoid.
    expect(monacoMounts().length).toBeGreaterThanOrEqual(3);
  });

  it("passes the overflow options to every Monaco instance", () => {
    const missing = monacoMounts()
      .filter(({ text }) => !text.includes("MONACO_OVERFLOW_OPTIONS"))
      .map(({ file }) => file.slice(SRC.length + 1));

    expect(
      missing,
      `These files mount Monaco without spreading MONACO_OVERFLOW_OPTIONS into `
        + `\`options\`, so their hovers will be clipped by .editor-container:\n`
        + missing.map((f) => `  - ${f}`).join("\n"),
    ).toEqual([]);
  });

  it("keeps the option itself set", () => {
    const opts = readFileSync(join(SRC, "lib", "monacoOptions.ts"), "utf8");
    expect(opts).toMatch(/fixedOverflowWidgets:\s*true/);
  });
});
