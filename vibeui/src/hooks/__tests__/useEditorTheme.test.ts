/**
 * Tests for useEditorTheme hook and its exported utilities.
 *
 * Covers:
 *  - getMonacoThemeData: valid/invalid IDs, dark vs light base, color format
 *  - useEditorTheme: initial theme name, response to custom events & storage events
 *  - defineTheme: Monaco API integration path
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useEditorTheme, getMonacoThemeData } from '../useEditorTheme';

// ── Helpers ───────────────────────────────────────────────────────────────────

/** Returns true if str looks like a 6- or 8-digit hex colour (#rrggbb or #rrggbbaa). */
function isHexColor(str: string): boolean {
  return /^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(str);
}

// ── getMonacoThemeData ────────────────────────────────────────────────────────

describe('getMonacoThemeData', () => {
  it('returns null for an unknown theme ID', () => {
    expect(getMonacoThemeData('does-not-exist')).toBeNull();
  });

  it('returns null for an empty string theme ID', () => {
    expect(getMonacoThemeData('')).toBeNull();
  });

  it('returns a theme data object for "dark-default"', () => {
    const data = getMonacoThemeData('dark-default');
    expect(data).not.toBeNull();
    expect(data).toBeTypeOf('object');
  });

  it('dark theme has base "vs-dark"', () => {
    // All dark themes include "dark" in their id
    const data = getMonacoThemeData('dark-default');
    expect(data?.base).toBe('vs-dark');
  });

  it('light theme has base "vs"', () => {
    // Find a light theme — "light-default" or similar
    const data = getMonacoThemeData('light-default');
    expect(data?.base).toBe('vs');
  });

  it('returned theme data has inherit: true', () => {
    const data = getMonacoThemeData('dark-default');
    expect(data?.inherit).toBe(true);
  });

  it('returned theme data has a non-empty rules array', () => {
    const data = getMonacoThemeData('dark-default');
    expect(Array.isArray(data?.rules)).toBe(true);
    expect(data!.rules.length).toBeGreaterThan(0);
  });

  it('rules contain expected token types', () => {
    const data = getMonacoThemeData('dark-default');
    const tokens = data!.rules.map((r) => r.token);
    expect(tokens).toContain('keyword');
    expect(tokens).toContain('string');
    expect(tokens).toContain('comment');
    expect(tokens).toContain('number');
  });

  it('returned colors map has editor.background', () => {
    const data = getMonacoThemeData('dark-default');
    expect(data?.colors?.['editor.background']).toBeDefined();
  });

  it('all color values in the colors map are hex strings (fully-hex theme)', () => {
    // dark-charcoal has no CSS-variable fallbacks in its vars — all values are real hex
    const data = getMonacoThemeData('dark-charcoal');
    expect(data).not.toBeNull();
    for (const [key, value] of Object.entries(data!.colors)) {
      expect(isHexColor(value), `${key}: "${value}" is not a hex color`).toBe(true);
    }
  });

  it('light theme (fully-hex) all color values are hex strings', () => {
    const data = getMonacoThemeData('light-charcoal');
    expect(data).not.toBeNull();
    for (const [key, value] of Object.entries(data!.colors)) {
      expect(isHexColor(value), `${key}: "${value}" is not a hex color`).toBe(true);
    }
  });

  it('rule foreground values are 6-digit hex (no #) for fully-hex theme', () => {
    // Use dark-charcoal which has no CSS-variable fallbacks
    const data = getMonacoThemeData('dark-charcoal');
    for (const rule of data!.rules) {
      if (rule.foreground) {
        expect(/^[0-9a-fA-F]{6}$/.test(rule.foreground), `"${rule.foreground}" is not 6-digit hex`).toBe(true);
      }
    }
  });

  it('includes diff editor colors', () => {
    const data = getMonacoThemeData('dark-default');
    expect(data?.colors?.['diffEditor.insertedTextBackground']).toBeDefined();
    expect(data?.colors?.['diffEditor.removedTextBackground']).toBeDefined();
  });

  it('includes minimap colors', () => {
    const data = getMonacoThemeData('dark-default');
    expect(data?.colors?.['minimap.background']).toBeDefined();
  });
});

// ── useEditorTheme ────────────────────────────────────────────────────────────

describe('useEditorTheme', () => {
  beforeEach(() => {
    // Default: "dark-default" theme stored in localStorage
    vi.stubGlobal('localStorage', {
      getItem: vi.fn().mockReturnValue('dark-default'),
      setItem: vi.fn(),
      removeItem: vi.fn(),
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('returns a themeName string starting with "vibeui-"', () => {
    const { result } = renderHook(() => useEditorTheme());
    expect(result.current.themeName).toMatch(/^vibeui-/);
  });

  it('includes the current theme id in the theme name', () => {
    const { result } = renderHook(() => useEditorTheme());
    expect(result.current.themeName).toContain('dark-default');
  });

  it('returns a defineTheme function', () => {
    const { result } = renderHook(() => useEditorTheme());
    expect(typeof result.current.defineTheme).toBe('function');
  });

  it('updates themeName when a "vibeui-theme-change" event fires', async () => {
    const { result } = renderHook(() => useEditorTheme());

    act(() => {
      window.dispatchEvent(
        new CustomEvent('vibeui-theme-change', { detail: { themeId: 'light-default' } }),
      );
    });

    expect(result.current.themeName).toContain('light-default');
  });

  it('updates themeName when a storage event fires with vibeui-theme-id key', async () => {
    const { result } = renderHook(() => useEditorTheme());

    act(() => {
      window.dispatchEvent(
        new StorageEvent('storage', { key: 'vibeui-theme-id', newValue: 'light-default' }),
      );
    });

    expect(result.current.themeName).toContain('light-default');
  });

  it('ignores storage events for other keys', () => {
    const { result } = renderHook(() => useEditorTheme());
    const originalTheme = result.current.themeName;

    act(() => {
      window.dispatchEvent(
        new StorageEvent('storage', { key: 'some-other-key', newValue: 'light-default' }),
      );
    });

    expect(result.current.themeName).toBe(originalTheme);
  });

  it('removes event listeners on unmount', () => {
    const addSpy = vi.spyOn(window, 'addEventListener');
    const removeSpy = vi.spyOn(window, 'removeEventListener');

    const { unmount } = renderHook(() => useEditorTheme());
    unmount();

    // Verify at least one vibeui-theme-change listener was removed
    const removed = removeSpy.mock.calls.some(([type]) => type === 'vibeui-theme-change');
    expect(removed).toBe(true);

    addSpy.mockRestore();
    removeSpy.mockRestore();
  });

  it('defineTheme calls defineTheme and setTheme on the monaco instance', () => {
    const { result } = renderHook(() => useEditorTheme());

    const mockMonaco = {
      editor: {
        defineTheme: vi.fn(),
        setTheme: vi.fn(),
      },
    } as unknown as typeof import('monaco-editor');

    act(() => {
      result.current.defineTheme(mockMonaco);
    });

    expect(mockMonaco.editor.defineTheme).toHaveBeenCalledOnce();
    expect(mockMonaco.editor.setTheme).toHaveBeenCalledOnce();
  });

  it('defineTheme updates themeName from the monaco call', () => {
    const { result } = renderHook(() => useEditorTheme());

    const mockMonaco = {
      editor: {
        defineTheme: vi.fn(),
        setTheme: vi.fn(),
      },
    } as unknown as typeof import('monaco-editor');

    act(() => {
      result.current.defineTheme(mockMonaco);
    });

    // After defineTheme the name should still be vibeui-<something>
    expect(result.current.themeName).toMatch(/^vibeui-/);
  });

  it('handles vibeui-theme-change with no detail gracefully', () => {
    const { result } = renderHook(() => useEditorTheme());
    const originalTheme = result.current.themeName;

    act(() => {
      window.dispatchEvent(new CustomEvent('vibeui-theme-change', { detail: null }));
    });

    // Should keep some valid theme name (falls back to localStorage / THEMES[0])
    expect(result.current.themeName).toMatch(/^vibeui-/);
    void originalTheme;
  });
});

// ── Color utility coverage via getMonacoThemeData ─────────────────────────────

describe('Color utilities (via getMonacoThemeData)', () => {
  it('toHex handles a theme with rgb() var values gracefully', () => {
    // We cannot call toHex directly (not exported) but if a theme uses rgb() values
    // in its CSS vars, getMonacoThemeData should still return valid hex colors.
    // This is a smoke test — any theme that processes correctly validates toHex.
    const data = getMonacoThemeData('dark-default');
    expect(data).not.toBeNull();
    // editor.background must be a valid hex color regardless of source format
    expect(isHexColor(data!.colors['editor.background'])).toBe(true);
  });

  it('lighten/darken values are within [0, 0xffffff] range', () => {
    // lineHighlightBackground is produced by lighten() or darken()
    const darkData = getMonacoThemeData('dark-default');
    const lightData = getMonacoThemeData('light-default');

    if (darkData) {
      const hex = darkData.colors['editor.lineHighlightBackground'];
      expect(isHexColor(hex)).toBe(true);
    }
    if (lightData) {
      const hex = lightData.colors['editor.lineHighlightBackground'];
      expect(isHexColor(hex)).toBe(true);
    }
  });

  it('hexToRgba 8-digit alpha values are valid', () => {
    // Use dark-charcoal which has all hardcoded hex vars (no CSS var references)
    const data = getMonacoThemeData('dark-charcoal');
    expect(data).not.toBeNull();
    // selectionBackground is produced by hexToRgba(accentBlue, 0.25) → 8-digit hex
    const sel = data!.colors['editor.selectionBackground'];
    expect(/^#[0-9a-fA-F]{8}$/.test(sel)).toBe(true);
  });
});
