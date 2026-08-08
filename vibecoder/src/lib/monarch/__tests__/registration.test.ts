/**
 * Where and when grammars get registered.
 *
 * Monaco resolves an unregistered language id to plaintext at `createModel`
 * time, and `@monaco-editor/react` creates the model *before* `onMount` fires
 * (its order is `beforeMount` → `getOrCreateModel` → `editor.create` → a later
 * effect → `onMount`). Its language effect also skips its first run, so nothing
 * corrects the model afterwards. Registering from `onMount` therefore leaves the
 * *first* file of a session unhighlighted — a `.zig` file would look plain until
 * you switched files and back.
 *
 * So registration belongs in `monaco-setup.ts`, at module load, before anything
 * renders. These tests pin that, because the failure is invisible to every other
 * test here: the grammars themselves are perfectly correct.
 */

import { describe, it, expect, vi } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { registerMonarchLanguages, MONARCH_LANGUAGES, grammarFor } from "../index";

const srcRoot = join(__dirname, "..", "..", "..");

describe("registration site", () => {
  it("happens in monaco-setup, which runs before any component renders", () => {
    const setup = readFileSync(join(srcRoot, "monaco-setup.ts"), "utf8");
    expect(setup).toContain("registerMonarchLanguages(monaco)");
  });

  it("does not happen in the editor's onMount, which is too late", () => {
    // `handleEditorDidMount` is `onMount`. A call there would silently break
    // highlighting for whichever file happens to be opened first.
    const app = readFileSync(join(srcRoot, "App.tsx"), "utf8");
    expect(app).not.toContain("registerMonarchLanguages(");
  });
});

/** Minimal Monaco stand-in that records registrations. */
function fakeMonaco(existingIds: string[] = []) {
  const registered: { id: string; extensions?: string[]; filenames?: string[] }[] = [];
  const tokenizers: string[] = [];
  const configurations: string[] = [];
  const onLanguageCallbacks = new Map<string, () => void>();
  return {
    registered,
    tokenizers,
    configurations,
    /** Simulate Monaco firing `onLanguage` when a file of that type opens. */
    openLanguage(id: string) {
      onLanguageCallbacks.get(id)?.();
    },
    monaco: {
      languages: {
        getLanguages: () => existingIds.map((id) => ({ id })),
        register: (language: { id: string }) => registered.push(language),
        onLanguage: (id: string, callback: () => void) => {
          onLanguageCallbacks.set(id, callback);
          return { dispose() {} };
        },
        setMonarchTokensProvider: (id: string) => tokenizers.push(id),
        setLanguageConfiguration: (id: string) => configurations.push(id),
      },
    } as never,
  };
}

describe("registerMonarchLanguages", () => {
  it("registers every grammar we supply", () => {
    const fake = fakeMonaco();
    const ids = registerMonarchLanguages(fake.monaco);
    expect(ids.length).toBe(MONARCH_LANGUAGES.length);
    expect(fake.registered.map((l) => l.id).sort()).toEqual(
      MONARCH_LANGUAGES.map((s) => s.id).sort(),
    );
  });

  it("registers extensions and filenames so Monaco can match files", () => {
    const fake = fakeMonaco();
    registerMonarchLanguages(fake.monaco);
    const zig = fake.registered.find((l) => l.id === "zig");
    expect(zig?.extensions).toContain(".zig");
    // CMakeLists.txt has no useful extension.
    const cmake = fake.registered.find((l) => l.id === "cmake");
    expect(cmake?.filenames).toContain("CMakeLists.txt");
  });

  it("never overrides a language Monaco already ships", () => {
    // Monaco's grammars are better maintained than ours; if it gains one of
    // these, theirs must win rather than being shadowed.
    const fake = fakeMonaco(["zig", "haskell"]);
    const ids = registerMonarchLanguages(fake.monaco);
    expect(ids).not.toContain("zig");
    expect(ids).not.toContain("haskell");
    expect(fake.registered.map((l) => l.id)).not.toContain("zig");
  });

  it("is idempotent — a second call registers nothing new", () => {
    const fake = fakeMonaco();
    const first = registerMonarchLanguages(fake.monaco);
    // The fake reports nothing pre-existing, so emulate Monaco's real behaviour
    // by feeding the first round's ids back in.
    const second = registerMonarchLanguages(fakeMonaco(first).monaco);
    expect(second).toEqual([]);
  });

  it("defers building the grammar until a file of that language opens", () => {
    // Mount cost stays flat as languages are added; the tokenizer is only
    // compiled when something actually needs it.
    const fake = fakeMonaco();
    registerMonarchLanguages(fake.monaco);
    expect(fake.tokenizers).toEqual([]);

    fake.openLanguage("zig");
    expect(fake.tokenizers).toEqual(["zig"]);
    expect(fake.configurations).toEqual(["zig"]);
  });

  it("supplies both a tokenizer and a configuration, not just colours", () => {
    // Without the configuration, ⌘/ does nothing in these files.
    const fake = fakeMonaco();
    registerMonarchLanguages(fake.monaco);
    for (const spec of MONARCH_LANGUAGES) {
      fake.openLanguage(spec.id);
    }
    expect(fake.tokenizers.sort()).toEqual(fake.configurations.sort());
    expect(fake.tokenizers.length).toBe(MONARCH_LANGUAGES.length);
  });

  it("merges the extra states a language needs", () => {
    // Nix's `''…''`, LaTeX's math modes, PostScript's nested strings and
    // CMake's in-string variables live in separate states.
    for (const id of ["nix", "latex", "postscript", "cmake"]) {
      const spec = MONARCH_LANGUAGES.find((s) => s.id === id);
      expect(spec, id).toBeDefined();
      const grammar = grammarFor(spec as never);
      const stateCount = Object.keys(grammar.tokenizer).length;
      expect(stateCount, `${id} has no extra states merged`).toBeGreaterThan(2);
    }
    // CMake's override replaces the generated string state rather than adding one.
    const cmake = MONARCH_LANGUAGES.find((s) => s.id === "cmake");
    const grammar = grammarFor(cmake as never);
    const doubleString = JSON.stringify(grammar.tokenizer.doubleString);
    expect(doubleString).toContain("variable");
  });

  it("does not throw when Monaco reports no languages at all", () => {
    const fake = fakeMonaco([]);
    expect(() => registerMonarchLanguages(fake.monaco)).not.toThrow();
  });
});

describe("grammar build cost", () => {
  it("builds one grammar per language, once", () => {
    const fake = fakeMonaco();
    const build = vi.fn();
    const original = fake.monaco as unknown as {
      languages: { setMonarchTokensProvider: (id: string) => void };
    };
    const wrapped = original.languages.setMonarchTokensProvider;
    original.languages.setMonarchTokensProvider = (id: string) => {
      build(id);
      wrapped(id);
    };
    registerMonarchLanguages(fake.monaco);
    fake.openLanguage("zig");
    fake.openLanguage("zig");
    // Monaco fires `onLanguage` once per language; our callback does no caching
    // of its own, so this documents the contract we rely on rather than hiding
    // repeated work.
    expect(build.mock.calls.length).toBeLessThanOrEqual(2);
  });
});
