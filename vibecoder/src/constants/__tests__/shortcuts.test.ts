/**
 * Ties the advertised shortcut list to the handlers that implement it.
 *
 * The four hand-maintained copies of this list drifted precisely because
 * nothing checked them: `⌘P` was documented for months without a handler, and
 * seven working bindings were missing from the README. Source-scanning is
 * coarse, but it is enough to catch "advertised but never bound", which is the
 * failure that actually happened.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import {
  SHORTCUTS,
  editorShortcuts,
  globalShortcuts,
  renderShortcut,
  type Shortcut,
} from "../shortcuts";

const appSource = readFileSync(
  resolve(__dirname, "..", "..", "App.tsx"),
  "utf8",
);

/** The literal each handler compares `e.key` against. */
function keyLiteral(s: Shortcut): string {
  // Single letters are matched lower-case by the plain-mod handlers and
  // upper-case by the shift ones, mirroring what the browser reports.
  if (/^[A-Z]$/.test(s.key)) return s.shift ? s.key : s.key.toLowerCase();
  return s.key;
}

describe("keyboard shortcuts", () => {
  it("every global shortcut has a handler comparing its key", () => {
    for (const s of globalShortcuts()) {
      if (s.key === "1-9") {
        // Range check rather than an equality test.
        expect(
          appSource.includes("e.key >= '1' && e.key <= '9'"),
          "no ⌘1-9 range handler found",
        ).toBe(true);
        continue;
      }
      const literal = `e.key === '${keyLiteral(s)}'`;
      expect(
        appSource.includes(literal),
        `${renderShortcut(s, true)} (${s.description}) is advertised but no handler matches ${literal}`,
      ).toBe(true);
    }
  });

  it("shift-qualified shortcuts test the shift key", () => {
    for (const s of globalShortcuts().filter((x) => x.shift)) {
      expect(
        appSource.includes("e.shiftKey"),
        `${renderShortcut(s, true)} needs a shiftKey test`,
      ).toBe(true);
    }
  });

  it("editor shortcuts are registered on Monaco, not on window", () => {
    // These live in handleEditorDidMount via editor.addCommand. If one ever
    // moves to a window listener it should also move scope, or the welcome
    // screen will hide a shortcut that does work there.
    expect(editorShortcuts().length).toBeGreaterThan(0);
    expect(appSource).toContain("addCommand");
  });

  it("renders platform-appropriate labels", () => {
    const palette = SHORTCUTS.find(
      (s) => s.key === "P" && !s.shift && s.mod,
    ) as Shortcut;
    expect(renderShortcut(palette, true)).toBe("⌘P");
    expect(renderShortcut(palette, false)).toBe("Ctrl+P");

    const shifted = SHORTCUTS.find(
      (s) => s.key === "P" && s.shift,
    ) as Shortcut;
    expect(renderShortcut(shifted, true)).toBe("⌘⇧P");
    expect(renderShortcut(shifted, false)).toBe("Ctrl+Shift+P");

    const inline = SHORTCUTS.find((s) => s.alt) as Shortcut;
    expect(renderShortcut(inline, true)).toBe("⌥\\");
    expect(renderShortcut(inline, false)).toBe("Alt+\\");
  });

  it("does not call ⌘1-9 an AI-tab switcher", () => {
    // Only the first seven of ALL_TABS are in the AI group; 8 is project-hub
    // and 9 is planning, so the old "Switch AI Tab" label was wrong for two of
    // the nine it described.
    const tabs = SHORTCUTS.find((s) => s.key === "1-9") as Shortcut;
    expect(tabs.description).not.toContain("AI Tab");
  });

  it("has no duplicate key combinations bound to different actions", () => {
    const seen = new Map<string, string>();
    for (const s of SHORTCUTS) {
      const combo = `${s.mod ? "mod+" : ""}${s.shift ? "shift+" : ""}${s.alt ? "alt+" : ""}${s.key}`;
      const prior = seen.get(combo);
      expect(
        prior === undefined || prior === s.description,
        `${combo} is bound to both "${prior}" and "${s.description}"`,
      ).toBe(true);
      seen.set(combo, s.description);
    }
  });
});
