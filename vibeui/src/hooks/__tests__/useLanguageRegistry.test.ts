/**
 * BDD / TDD tests for useLanguageRegistry.
 *
 * Scenarios:
 *  - TIOBE_TOP50 data integrity (50 entries, unique ranks, correct structure)
 *  - EXT_TO_LANGUAGE extension-to-language map (conflict resolution)
 *  - getLanguageFromPath utility
 *  - useLanguageRegistry hook return values
 */

import { describe, it, expect } from 'vitest';
import { renderHook } from '@testing-library/react';
import {
  TIOBE_TOP50,
  EXT_TO_LANGUAGE,
  getLanguageFromPath,
  useLanguageRegistry,
} from '../useLanguageRegistry';

// ── TIOBE_TOP50 data ──────────────────────────────────────────────────────────

describe('TIOBE_TOP50 dataset', () => {
  it('contains exactly 50 entries', () => {
    expect(TIOBE_TOP50).toHaveLength(50);
  });

  it('has unique language IDs', () => {
    const ids = TIOBE_TOP50.map((l) => l.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('has unique TIOBE ranks 1-50', () => {
    const ranks = TIOBE_TOP50.map((l) => l.tiobeRank);
    expect(new Set(ranks).size).toBe(50);
    expect(Math.min(...ranks)).toBe(1);
    expect(Math.max(...ranks)).toBe(50);
  });

  it('every entry has a non-empty id, name, monacoId, and commentPrefix', () => {
    for (const lang of TIOBE_TOP50) {
      expect(lang.id, `${lang.id}.id`).toBeTruthy();
      expect(lang.name, `${lang.id}.name`).toBeTruthy();
      expect(lang.monacoId, `${lang.id}.monacoId`).toBeTruthy();
      expect(lang.commentPrefix, `${lang.id}.commentPrefix`).toBeTruthy();
    }
  });

  it('every entry has a non-empty tags array', () => {
    for (const lang of TIOBE_TOP50) {
      expect(Array.isArray(lang.tags), `${lang.id}.tags`).toBe(true);
      expect(lang.tags.length, `${lang.id}.tags length`).toBeGreaterThan(0);
    }
  });

  it('every entry has a non-empty extensions array (or isVisual)', () => {
    for (const lang of TIOBE_TOP50) {
      // Visual languages (Scratch, LabVIEW, Ladder Logic) may have extensions too
      expect(Array.isArray(lang.extensions), `${lang.id}.extensions`).toBe(true);
      expect(lang.extensions.length, `${lang.id}.extensions length`).toBeGreaterThan(0);
    }
  });

  it('every color is a valid hex string (#rrggbb)', () => {
    for (const lang of TIOBE_TOP50) {
      expect(/^#[0-9a-fA-F]{6}$/.test(lang.color), `${lang.id}.color`).toBe(true);
    }
  });

  it('has Python at rank 1', () => {
    const python = TIOBE_TOP50.find((l) => l.id === 'python');
    expect(python).toBeDefined();
    expect(python!.tiobeRank).toBe(1);
  });

  it('has Rust in the list', () => {
    const rust = TIOBE_TOP50.find((l) => l.id === 'rust');
    expect(rust).toBeDefined();
    expect(rust!.monacoId).toBe('rust');
  });

  it('has TypeScript in the list', () => {
    const ts = TIOBE_TOP50.find((l) => l.id === 'typescript');
    expect(ts).toBeDefined();
    expect(ts!.extensions).toContain('ts');
    expect(ts!.extensions).toContain('tsx');
  });

  it('isVisual entries include Scratch and LabVIEW', () => {
    const visualIds = TIOBE_TOP50.filter((l) => l.isVisual).map((l) => l.id);
    expect(visualIds).toContain('scratch');
    expect(visualIds).toContain('labview');
  });
});

// ── EXT_TO_LANGUAGE map ───────────────────────────────────────────────────────

describe('EXT_TO_LANGUAGE extension map', () => {
  it('maps "py" to Python', () => {
    expect(EXT_TO_LANGUAGE['py']?.id).toBe('python');
  });

  it('maps "rs" to Rust', () => {
    expect(EXT_TO_LANGUAGE['rs']?.id).toBe('rust');
  });

  it('maps "ts" to TypeScript', () => {
    expect(EXT_TO_LANGUAGE['ts']?.id).toBe('typescript');
  });

  it('maps "js" to JavaScript', () => {
    expect(EXT_TO_LANGUAGE['js']?.id).toBe('javascript');
  });

  it('maps "go" to Go', () => {
    expect(EXT_TO_LANGUAGE['go']?.id).toBe('go');
  });

  it('maps "kt" to Kotlin', () => {
    expect(EXT_TO_LANGUAGE['kt']?.id).toBe('kotlin');
  });

  it('maps "rb" to Ruby', () => {
    expect(EXT_TO_LANGUAGE['rb']?.id).toBe('ruby');
  });

  it('maps "swift" to Swift', () => {
    expect(EXT_TO_LANGUAGE['swift']?.id).toBe('swift');
  });

  it('maps "sol" to Solidity', () => {
    expect(EXT_TO_LANGUAGE['sol']?.id).toBe('solidity');
  });

  it('maps "lua" to Lua', () => {
    expect(EXT_TO_LANGUAGE['lua']?.id).toBe('lua');
  });

  it('maps "dart" to Dart', () => {
    expect(EXT_TO_LANGUAGE['dart']?.id).toBe('dart');
  });

  it('maps "sql" to SQL (first in TIOBE rank order)', () => {
    // "sql" extension belongs to SQL (rank 8), not TSQL (rank 44)
    expect(EXT_TO_LANGUAGE['sql']?.id).toBe('sql');
  });

  it('maps .m to MATLAB (first-match-wins over Objective-C)', () => {
    // MATLAB is rank 17, Objective-C is rank 27 — first-match-wins
    expect(EXT_TO_LANGUAGE['m']?.id).toBe('matlab');
  });

  it('maps .pl to Perl (first-match-wins over Prolog)', () => {
    // Perl is rank 12, Prolog is rank 22
    expect(EXT_TO_LANGUAGE['pl']?.id).toBe('perl');
  });

  it('does NOT contain unknown extensions', () => {
    expect(EXT_TO_LANGUAGE['doesnotexist']).toBeUndefined();
    expect(EXT_TO_LANGUAGE['xyz']).toBeUndefined();
  });

  it('all mapped values are valid LanguageEntry objects', () => {
    for (const [ext, entry] of Object.entries(EXT_TO_LANGUAGE)) {
      expect(typeof entry.id, `ext=${ext} id`).toBe('string');
      expect(entry.tiobeRank, `ext=${ext} rank`).toBeGreaterThan(0);
    }
  });
});

// ── getLanguageFromPath ───────────────────────────────────────────────────────

describe('getLanguageFromPath', () => {
  it('returns Python for "main.py"', () => {
    expect(getLanguageFromPath('main.py')?.id).toBe('python');
  });

  it('returns Rust for "src/lib.rs"', () => {
    expect(getLanguageFromPath('src/lib.rs')?.id).toBe('rust');
  });

  it('returns TypeScript for "components/App.tsx"', () => {
    expect(getLanguageFromPath('components/App.tsx')?.id).toBe('typescript');
  });

  it('returns Go for "cmd/main.go"', () => {
    expect(getLanguageFromPath('cmd/main.go')?.id).toBe('go');
  });

  it('returns undefined for a file without extension', () => {
    expect(getLanguageFromPath('Makefile')).toBeUndefined();
    expect(getLanguageFromPath('Dockerfile')).toBeUndefined();
  });

  it('returns undefined for an unknown extension', () => {
    expect(getLanguageFromPath('file.unknownext')).toBeUndefined();
  });

  it('is case-insensitive for the extension', () => {
    // Extension lookup lowercases before map lookup
    expect(getLanguageFromPath('Main.PY')?.id).toBe('python');
    expect(getLanguageFromPath('lib.RS')?.id).toBe('rust');
  });

  it('uses the last segment of a dotted filename (e.g. ".d.ts" → "ts")', () => {
    // "index.d.ts" — last extension is "ts"
    expect(getLanguageFromPath('index.d.ts')?.id).toBe('typescript');
  });

  it('returns Ruby for "Gemfile.rake"', () => {
    expect(getLanguageFromPath('tasks.rake')?.id).toBe('ruby');
  });

  it('returns Swift for "App.swift"', () => {
    expect(getLanguageFromPath('App.swift')?.id).toBe('swift');
  });

  it('returns Kotlin for "Main.kt"', () => {
    expect(getLanguageFromPath('Main.kt')?.id).toBe('kotlin');
  });

  it('returns Dart for "widget.dart"', () => {
    expect(getLanguageFromPath('widget.dart')?.id).toBe('dart');
  });

  it('handles deeply nested paths', () => {
    expect(getLanguageFromPath('a/b/c/d/e/f/main.py')?.id).toBe('python');
  });
});

// ── useLanguageRegistry hook ──────────────────────────────────────────────────

describe('useLanguageRegistry hook', () => {
  it('returns languages array equal to TIOBE_TOP50', () => {
    const { result } = renderHook(() => useLanguageRegistry());
    expect(result.current.languages).toBe(TIOBE_TOP50);
    expect(result.current.languages).toHaveLength(50);
  });

  it('getByExtension("py") returns Python', () => {
    const { result } = renderHook(() => useLanguageRegistry());
    expect(result.current.getByExtension('py')?.id).toBe('python');
  });

  it('getByExtension(".ts") strips leading dot and returns TypeScript', () => {
    const { result } = renderHook(() => useLanguageRegistry());
    expect(result.current.getByExtension('.ts')?.id).toBe('typescript');
  });

  it('getByExtension("TS") is case-insensitive', () => {
    const { result } = renderHook(() => useLanguageRegistry());
    expect(result.current.getByExtension('TS')?.id).toBe('typescript');
  });

  it('getByExtension("unknown") returns undefined', () => {
    const { result } = renderHook(() => useLanguageRegistry());
    expect(result.current.getByExtension('unknown')).toBeUndefined();
  });

  it('getByPath("main.rs") returns Rust', () => {
    const { result } = renderHook(() => useLanguageRegistry());
    expect(result.current.getByPath('main.rs')?.id).toBe('rust');
  });

  it('getById("python") returns Python', () => {
    const { result } = renderHook(() => useLanguageRegistry());
    const lang = result.current.getById('python');
    expect(lang?.id).toBe('python');
    expect(lang?.name).toBe('Python');
  });

  it('getById("nonexistent") returns undefined', () => {
    const { result } = renderHook(() => useLanguageRegistry());
    expect(result.current.getById('nonexistent')).toBeUndefined();
  });

  it('textLanguages excludes visual languages', () => {
    const { result } = renderHook(() => useLanguageRegistry());
    const visualInText = result.current.textLanguages.filter((l) => l.isVisual);
    expect(visualInText).toHaveLength(0);
  });

  it('textLanguages contains Python, Rust, TypeScript', () => {
    const { result } = renderHook(() => useLanguageRegistry());
    const ids = result.current.textLanguages.map((l) => l.id);
    expect(ids).toContain('python');
    expect(ids).toContain('rust');
    expect(ids).toContain('typescript');
  });

  it('textLanguages has fewer entries than the full list (visual langs excluded)', () => {
    const { result } = renderHook(() => useLanguageRegistry());
    const visualCount = TIOBE_TOP50.filter((l) => l.isVisual).length;
    expect(result.current.textLanguages.length).toBe(TIOBE_TOP50.length - visualCount);
  });
});
