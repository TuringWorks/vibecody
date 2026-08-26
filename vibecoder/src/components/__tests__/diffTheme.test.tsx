import { describe, it, expect } from "vitest";
import fs from "node:fs";
import path from "node:path";

/**
 * Every Monaco surface must follow the selected editor theme.
 *
 * Two ways this broke, both shipped:
 *   - App.tsx's git diff passed `theme={editorTheme}` but never *defined* it.
 *     The name is only a string; the main editor registered it in its own
 *     onMount, so opening a diff with no file open handed Monaco a name it had
 *     never seen and it fell back to `vs` — a white diff in a dark app.
 *   - DiffReviewPanel hardcoded `theme="vs-dark"`, so a user on a light theme
 *     got a dark review pane beside a light editor.
 *
 * A source-level check, because both failures are about wiring that renders
 * perfectly well while being wrong.
 */
const SRC = path.resolve(__dirname, "../..");

function read(rel: string): string {
  return fs.readFileSync(path.join(SRC, rel), "utf8");
}

/** Files that mount Monaco and must therefore participate in theming. */
const MONACO_SURFACES = ["App.tsx", "components/DiffReviewPanel.tsx", "components/DocumentTextEditor.tsx"];

describe("Monaco theming", () => {
  it("no surface pins a built-in Monaco theme", () => {
    for (const f of MONACO_SURFACES) {
      const src = read(f);
      const pinned = src.match(/theme=["'](vs|vs-dark|hc-black|hc-light)["']/g);
      expect(pinned, `${f} pins a built-in theme instead of following the editor`).toBeNull();
    }
  });

  it("every editor that names a theme also defines it before mounting", () => {
    for (const f of MONACO_SURFACES) {
      const src = read(f);
      const editors = (src.match(/<(?:Diff)?Editor\b/g) ?? []).length;
      if (editors === 0) continue;
      // Each editor needs the theme registered on the monaco instance — via its
      // own beforeMount, or an onMount that calls defineTheme.
      const registrations =
        (src.match(/beforeMount=\{/g) ?? []).length + (src.match(/defineTheme\(/g) ?? []).length;
      expect(
        registrations,
        `${f} mounts ${editors} Monaco editor(s) but never registers the theme`,
      ).toBeGreaterThan(0);
    }
  });

  it("the git diff view registers the theme itself", () => {
    // It can be opened with no file open, so it cannot rely on the main
    // editor's onMount having run.
    const app = read("App.tsx");
    const diffBlock = app.slice(app.indexOf("<DiffEditor"), app.indexOf("<DiffEditor") + 900);
    expect(diffBlock).toContain("beforeMount=");
  });
});
