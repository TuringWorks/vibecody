/**
 * Keyboard shortcuts — the single source of truth.
 *
 * This list existed in four hand-maintained copies (the welcome screen, this
 * app's README, the docs site, and the command palette's own help) which had
 * drifted apart: the READMEs advertised a `⌘P` palette that was never
 * implemented, documented a `⌥⌘I` DevTools binding that does not exist, named a
 * "Memory" tab that is not in `ALL_TABS`, and omitted seven working bindings.
 * Nothing tied any copy to a handler, so the drift was invisible.
 *
 * Every entry here corresponds to a handler in `App.tsx`, and
 * `shortcuts.test.ts` asserts that correspondence by reading the source, so a
 * binding cannot be added, removed, or renamed on one side alone.
 */

/**
 * Where a shortcut is live.
 *
 * `global` — a `window` keydown listener in `App.tsx`; works anywhere in the app.
 * `editor` — registered on the Monaco instance via `editor.addCommand` in
 *   `handleEditorDidMount`, so it fires only while the editor is mounted *and*
 *   focused. Monaco is not mounted at all on the welcome screen (it is the
 *   else-branch of "is a file open"), which is why these must not be listed
 *   there without qualification.
 */
export type ShortcutScope = "global" | "editor";

export interface Shortcut {
  /** Cmd on macOS, Ctrl elsewhere. */
  mod?: boolean;
  shift?: boolean;
  /** Option on macOS, Alt elsewhere. */
  alt?: boolean;
  /** The key as rendered to the user, e.g. "K", "1-9", "`". */
  key: string;
  description: string;
  scope: ShortcutScope;
}

export const SHORTCUTS: readonly Shortcut[] = [
  // ── Global (window keydown listeners in App.tsx) ──────────────────────────
  { mod: true, key: "K", description: "Command Palette", scope: "global" },
  { mod: true, key: "P", description: "Command Palette", scope: "global" },
  {
    mod: true,
    shift: true,
    key: "P",
    description: "Command Palette",
    scope: "global",
  },
  { mod: true, key: "J", description: "Toggle AI Panel", scope: "global" },
  { mod: true, key: "B", description: "Toggle Sidebar", scope: "global" },
  { mod: true, key: "`", description: "Toggle Terminal", scope: "global" },
  { mod: true, shift: true, key: "E", description: "Explorer", scope: "global" },
  {
    mod: true,
    shift: true,
    key: "G",
    description: "Source Control",
    scope: "global",
  },
  {
    mod: true,
    shift: true,
    key: "M",
    description: "Maximize Panels (Esc restores)",
    scope: "global",
  },
  { mod: true, key: "S", description: "Save File", scope: "global" },
  { mod: true, key: "O", description: "Open Folder", scope: "global" },
  // Not "Switch AI Tab": this indexes the first nine of ALL_TABS, and only the
  // first seven are in the AI group — 8 is project-hub and 9 is planning.
  { mod: true, key: "1-9", description: "Switch Panel Tab", scope: "global" },

  // ── Editor (Monaco commands; need a file open and the editor focused) ─────
  {
    mod: true,
    key: ".",
    description: "AI Edit (DiffComplete)",
    scope: "editor",
  },
  { alt: true, key: "\\", description: "AI Inline Completion", scope: "editor" },
] as const;

/** Render a shortcut for the current platform, e.g. "⌘⇧P" or "Ctrl+Shift+P". */
export function renderShortcut(s: Shortcut, isMac: boolean): string {
  const parts: string[] = [];
  if (s.mod) parts.push(isMac ? "⌘" : "Ctrl+");
  if (s.shift) parts.push(isMac ? "⇧" : "Shift+");
  if (s.alt) parts.push(isMac ? "⌥" : "Alt+");
  return parts.join("") + s.key;
}

export const globalShortcuts = (): Shortcut[] =>
  SHORTCUTS.filter((s) => s.scope === "global");

export const editorShortcuts = (): Shortcut[] =>
  SHORTCUTS.filter((s) => s.scope === "editor");
