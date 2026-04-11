/**
 * useEditorTheme — Syncs Monaco editor themes with VibeUI's CSS variable theme system.
 *
 * Generates a Monaco theme from the active VibeUI theme's CSS variables, registers it,
 * and returns the theme name for use by <Editor> and <DiffEditor>.
 *
 * Listens for "vibeui-theme-change" custom events dispatched by applyThemeById().
 */
import { useState, useEffect, useRef, useCallback } from "react";
import { THEMES, type ThemeDef } from "../components/SettingsPanel";
import type { editor } from "monaco-editor";

/** Monaco theme name prefix */
const THEME_PREFIX = "vibeui-";

/** Convert any CSS color to a Monaco-compatible #rrggbb hex string.
 *  Monaco requires hex — it rejects rgb(), hsl(), named colors, etc.
 */
function toHex(color: string): string {
  if (!color) return "#808080";
  const s = color.trim();
  if (s.startsWith("#")) return s;
  // Parse rgb(r, g, b) or rgb(r g b)
  const rgbMatch = s.match(/^rgba?\(\s*([\d.]+)[,\s]\s*([\d.]+)[,\s]\s*([\d.]+)/);
  if (rgbMatch) {
    const r = Math.round(Number(rgbMatch[1])).toString(16).padStart(2, "0");
    const g = Math.round(Number(rgbMatch[2])).toString(16).padStart(2, "0");
    const b = Math.round(Number(rgbMatch[3])).toString(16).padStart(2, "0");
    return `#${r}${g}${b}`;
  }
  return s;
}

/** Slightly lighten a hex color for highlights */
function lighten(hex: string, amount: number): string {
  const h = hex.replace("#", "");
  const r = Math.min(255, parseInt(h.substring(0, 2), 16) + amount);
  const g = Math.min(255, parseInt(h.substring(2, 4), 16) + amount);
  const b = Math.min(255, parseInt(h.substring(4, 6), 16) + amount);
  return `#${r.toString(16).padStart(2, "0")}${g.toString(16).padStart(2, "0")}${b.toString(16).padStart(2, "0")}`;
}

/** Slightly darken a hex color */
function darken(hex: string, amount: number): string {
  const h = hex.replace("#", "");
  const r = Math.max(0, parseInt(h.substring(0, 2), 16) - amount);
  const g = Math.max(0, parseInt(h.substring(2, 4), 16) - amount);
  const b = Math.max(0, parseInt(h.substring(4, 6), 16) - amount);
  return `#${r.toString(16).padStart(2, "0")}${g.toString(16).padStart(2, "0")}${b.toString(16).padStart(2, "0")}`;
}

/** Hex to 8-digit hex string for Monaco alpha colors */
function hexToRgba(hex: string, alpha: number): string {
  const h = hex.replace("#", "").substring(0, 6);
  const a = Math.round(alpha * 255).toString(16).padStart(2, "0");
  return `#${h}${a}`;
}

/** Build a full Monaco IStandaloneThemeData from a VibeUI ThemeDef */
function buildMonacoTheme(theme: ThemeDef): editor.IStandaloneThemeData {
  const v = theme.vars;
  const isDark = theme.mode === "dark";
  const base = isDark ? "vs-dark" : "vs";

  const bgPrimary = toHex(v["--bg-primary"]);
  const bgSecondary = toHex(v["--bg-secondary"]);
  const bgTertiary = toHex(v["--bg-tertiary"]);
  const bgElevated = toHex(v["--bg-elevated"]);
  const textPrimary = toHex(v["--text-primary"]);
  const textSecondary = toHex(v["--text-secondary"]);
  const accentBlue = toHex(v["--accent-blue"]);
  const accentGreen = toHex(v["--accent-green"]);
  const accentPurple = toHex(v["--accent-purple"]);
  const accentGold = toHex(v["--accent-gold"]);
  const accentRose = toHex(v["--accent-rose"]);
  const errorColor = toHex(v["--error-color"]);

  // Selection and highlight colors
  const selectionBg = hexToRgba(accentBlue, 0.25);
  const hoverHighlight = hexToRgba(accentBlue, 0.08);
  const lineHighlight = isDark ? lighten(bgPrimary, 8) : darken(bgPrimary, 6);
  const wordHighlight = hexToRgba(accentBlue, 0.15);
  const findMatch = hexToRgba(accentGold, 0.30);
  const findMatchHighlight = hexToRgba(accentGold, 0.15);

  return {
    base: base as "vs" | "vs-dark",
    inherit: true,
    rules: [
      // Language tokens — use theme palette
      { token: "", foreground: textPrimary.replace("#", ""), background: bgPrimary.replace("#", "") },
      { token: "comment", foreground: textSecondary.replace("#", ""), fontStyle: "italic" },
      { token: "keyword", foreground: accentPurple.replace("#", "") },
      { token: "keyword.control", foreground: accentPurple.replace("#", "") },
      { token: "string", foreground: accentGreen.replace("#", "") },
      { token: "string.escape", foreground: accentGold.replace("#", "") },
      { token: "number", foreground: accentGold.replace("#", "") },
      { token: "number.hex", foreground: accentGold.replace("#", "") },
      { token: "regexp", foreground: accentRose.replace("#", "") },
      { token: "type", foreground: accentBlue.replace("#", "") },
      { token: "type.identifier", foreground: accentBlue.replace("#", "") },
      { token: "class", foreground: accentBlue.replace("#", "") },
      { token: "interface", foreground: accentBlue.replace("#", "") },
      { token: "struct", foreground: accentBlue.replace("#", "") },
      { token: "function", foreground: accentGold.replace("#", "") },
      { token: "function.declaration", foreground: accentGold.replace("#", "") },
      { token: "variable", foreground: textPrimary.replace("#", "") },
      { token: "variable.predefined", foreground: accentBlue.replace("#", "") },
      { token: "constant", foreground: accentGold.replace("#", "") },
      { token: "operator", foreground: textPrimary.replace("#", "") },
      { token: "delimiter", foreground: textSecondary.replace("#", "") },
      { token: "delimiter.bracket", foreground: textSecondary.replace("#", "") },
      { token: "tag", foreground: accentRose.replace("#", "") },
      { token: "tag.attribute.name", foreground: accentGold.replace("#", "") },
      { token: "tag.attribute.value", foreground: accentGreen.replace("#", "") },
      { token: "attribute.name", foreground: accentGold.replace("#", "") },
      { token: "attribute.value", foreground: accentGreen.replace("#", "") },
      { token: "annotation", foreground: accentPurple.replace("#", "") },
      { token: "namespace", foreground: accentBlue.replace("#", "") },
      { token: "meta", foreground: textSecondary.replace("#", "") },
      { token: "invalid", foreground: errorColor.replace("#", "") },
    ],
    colors: {
      // Editor background & foreground
      "editor.background": bgPrimary,
      "editor.foreground": textPrimary,

      // Line numbers
      "editorLineNumber.foreground": textSecondary,
      "editorLineNumber.activeForeground": textPrimary,

      // Cursor
      "editorCursor.foreground": accentBlue,

      // Selection
      "editor.selectionBackground": selectionBg,
      "editor.inactiveSelectionBackground": hexToRgba(accentBlue, 0.12),
      "editor.selectionHighlightBackground": wordHighlight,

      // Current line
      "editor.lineHighlightBackground": lineHighlight,
      "editor.lineHighlightBorder": "#00000000",

      // Find match
      "editor.findMatchBackground": findMatch,
      "editor.findMatchHighlightBackground": findMatchHighlight,

      // Word highlight (occurrences)
      "editor.wordHighlightBackground": wordHighlight,
      "editor.wordHighlightStrongBackground": hexToRgba(accentBlue, 0.20),

      // Hover
      "editor.hoverHighlightBackground": hoverHighlight,

      // Bracket matching
      "editorBracketMatch.background": hexToRgba(accentBlue, 0.15),
      "editorBracketMatch.border": accentBlue,

      // Indentation guides
      "editorIndentGuide.background": isDark ? lighten(bgPrimary, 12) : darken(bgPrimary, 10),
      "editorIndentGuide.activeBackground": isDark ? lighten(bgPrimary, 24) : darken(bgPrimary, 20),

      // Whitespace
      "editorWhitespace.foreground": isDark ? lighten(bgPrimary, 18) : darken(bgPrimary, 12),

      // Rulers
      "editorRuler.foreground": isDark ? lighten(bgPrimary, 10) : darken(bgPrimary, 8),

      // Gutter
      "editorGutter.background": bgPrimary,
      "editorGutter.addedBackground": accentGreen,
      "editorGutter.modifiedBackground": accentBlue,
      "editorGutter.deletedBackground": errorColor,

      // Minimap
      "minimap.background": bgSecondary,
      "minimapSlider.background": hexToRgba(accentBlue, 0.10),
      "minimapSlider.hoverBackground": hexToRgba(accentBlue, 0.18),
      "minimapSlider.activeBackground": hexToRgba(accentBlue, 0.25),

      // Scrollbar
      "scrollbar.shadow": "#00000020",
      "scrollbarSlider.background": hexToRgba(textSecondary, 0.15),
      "scrollbarSlider.hoverBackground": hexToRgba(textSecondary, 0.25),
      "scrollbarSlider.activeBackground": hexToRgba(textSecondary, 0.35),

      // Widget (autocomplete, hover tooltip)
      "editorWidget.background": bgElevated,
      "editorWidget.border": isDark ? lighten(bgSecondary, 10) : darken(bgSecondary, 10),
      "editorWidget.foreground": textPrimary,
      "editorSuggestWidget.background": bgElevated,
      "editorSuggestWidget.border": isDark ? lighten(bgSecondary, 10) : darken(bgSecondary, 10),
      "editorSuggestWidget.foreground": textPrimary,
      "editorSuggestWidget.selectedBackground": hexToRgba(accentBlue, 0.20),
      "editorSuggestWidget.highlightForeground": accentBlue,
      "editorHoverWidget.background": bgElevated,
      "editorHoverWidget.border": isDark ? lighten(bgSecondary, 10) : darken(bgSecondary, 10),

      // Peek view
      "peekView.border": accentBlue,
      "peekViewEditor.background": bgSecondary,
      "peekViewResult.background": bgSecondary,
      "peekViewTitle.background": bgTertiary,
      "peekViewEditor.matchHighlightBackground": findMatchHighlight,

      // Diff editor
      "diffEditor.insertedTextBackground": hexToRgba(accentGreen, 0.12),
      "diffEditor.removedTextBackground": hexToRgba(errorColor, 0.12),
      "diffEditor.insertedLineBackground": hexToRgba(accentGreen, 0.06),
      "diffEditor.removedLineBackground": hexToRgba(errorColor, 0.06),

      // Error/warning squiggles
      "editorError.foreground": errorColor,
      "editorWarning.foreground": accentGold,
      "editorInfo.foreground": accentBlue,

      // Input (find/replace bar)
      "input.background": bgSecondary,
      "input.foreground": textPrimary,
      "input.border": isDark ? lighten(bgSecondary, 12) : darken(bgSecondary, 12),
      "inputOption.activeBorder": accentBlue,
      "inputOption.activeBackground": hexToRgba(accentBlue, 0.20),

      // Dropdown
      "dropdown.background": bgElevated,
      "dropdown.foreground": textPrimary,
      "dropdown.border": isDark ? lighten(bgSecondary, 10) : darken(bgSecondary, 10),

      // List (autocomplete list, etc.)
      "list.hoverBackground": hoverHighlight,
      "list.activeSelectionBackground": hexToRgba(accentBlue, 0.22),
      "list.activeSelectionForeground": textPrimary,
      "list.focusBackground": hexToRgba(accentBlue, 0.18),
      "list.highlightForeground": accentBlue,

      // Overview ruler (right-side scrollbar annotations)
      "editorOverviewRuler.errorForeground": errorColor,
      "editorOverviewRuler.warningForeground": accentGold,
      "editorOverviewRuler.infoForeground": accentBlue,
      "editorOverviewRuler.findMatchForeground": accentGold,
      "editorOverviewRuler.selectionHighlightForeground": accentBlue,
      "editorOverviewRuler.modifiedForeground": accentBlue,
      "editorOverviewRuler.addedForeground": accentGreen,
      "editorOverviewRuler.deletedForeground": errorColor,
    },
  };
}

/** Get the current VibeUI theme from localStorage */
function getCurrentTheme(): ThemeDef {
  const id = localStorage.getItem("vibeui-theme-id") || "dark-default";
  return THEMES.find((t) => t.id === id) || THEMES[0];
}

/**
 * Hook: returns the current Monaco editor theme name.
 *
 * On mount and on theme change events, defines/updates the Monaco theme
 * and returns the theme name string for <Editor theme={...}>.
 *
 * Pass the returned `monacoRef` to handleEditorDidMount to capture the
 * monaco instance, or call `defineTheme(monaco)` manually.
 */
export function useEditorTheme() {
  const monacoRef = useRef<typeof import("monaco-editor") | null>(null);
  const [themeName, setThemeName] = useState(() => {
    const theme = getCurrentTheme();
    return `${THEME_PREFIX}${theme.id}`;
  });

  const applyMonacoTheme = useCallback((themeId?: string) => {
    const theme = themeId
      ? THEMES.find((t) => t.id === themeId) || getCurrentTheme()
      : getCurrentTheme();
    const name = `${THEME_PREFIX}${theme.id}`;
    const monaco = monacoRef.current;
    if (monaco) {
      try {
        monaco.editor.defineTheme(name, buildMonacoTheme(theme));
        monaco.editor.setTheme(name);
      } catch {
        // Monaco not ready yet
      }
    }
    setThemeName(name);
  }, []);

  // Listen for theme changes from applyThemeById / ThemeToggle / SettingsPanel
  useEffect(() => {
    const handler = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      applyMonacoTheme(detail?.themeId);
    };
    window.addEventListener("vibeui-theme-change", handler);

    // Also listen for storage events (cross-tab sync)
    const storageHandler = (e: StorageEvent) => {
      if (e.key === "vibeui-theme-id" && e.newValue) {
        applyMonacoTheme(e.newValue);
      }
    };
    window.addEventListener("storage", storageHandler);

    return () => {
      window.removeEventListener("vibeui-theme-change", handler);
      window.removeEventListener("storage", storageHandler);
    };
  }, [applyMonacoTheme]);

  /** Call this from onMount to capture the monaco instance and define the initial theme */
  const defineTheme = useCallback(
    (monaco: typeof import("monaco-editor")) => {
      monacoRef.current = monaco;
      // Pre-define themes for current and paired theme
      const current = getCurrentTheme();
      const name = `${THEME_PREFIX}${current.id}`;
      monaco.editor.defineTheme(name, buildMonacoTheme(current));
      monaco.editor.setTheme(name);
      setThemeName(name);
    },
    []
  );

  return { themeName, defineTheme };
}

/** Utility: build a Monaco theme from a VibeUI theme ID (for external use) */
export function getMonacoThemeData(themeId: string): editor.IStandaloneThemeData | null {
  const theme = THEMES.find((t) => t.id === themeId);
  if (!theme) return null;
  return buildMonacoTheme(theme);
}
