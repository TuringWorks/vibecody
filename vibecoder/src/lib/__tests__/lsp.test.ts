/**
 * Tests for the LSP ↔ Monaco bridge.
 *
 * Enums come from Monaco's own `standaloneEnums.js` rather than hand-written
 * constants: the whole point of mapping by name is that Monaco renumbers, and a
 * test with copied numbers would keep passing through exactly the renumbering
 * it exists to catch. (In this version `Snippet` is 28; it used to be 27.)
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  CompletionItemKind,
  CompletionItemInsertTextRule,
  CompletionItemTag,
  MarkerSeverity,
} from "monaco-editor/esm/vs/editor/common/standalone/standaloneEnums.js";
import { URI } from "monaco-editor/esm/vs/base/common/uri.js";
import {
  createLspBridge,
  fileUri,
  lspLanguageForPath,
  parentDirectory,
  toLspPosition,
  toMonacoCompletionItem,
  toMonacoCompletionKind,
  toMonacoCompletionList,
  toMonacoHover,
  toMonacoLocations,
  toMonacoMarkers,
  toMonacoRange,
  toMonacoSeverity,
  toMonacoSignatureHelp,
  triggerCharacters,
  MARKER_OWNER,
  type LspBridge,
  type MonacoLspEnums,
} from "../lsp";

const enums: MonacoLspEnums = {
  completionKinds: CompletionItemKind,
  insertAsSnippet: CompletionItemInsertTextRule.InsertAsSnippet,
  deprecatedTag: CompletionItemTag.Deprecated,
  markerSeverity: MarkerSeverity,
};

const range = (
  startLine: number,
  startChar: number,
  endLine: number,
  endChar: number,
) => ({
  start: { line: startLine, character: startChar },
  end: { line: endLine, character: endChar },
});

const fallbackRange = {
  startLineNumber: 1,
  endLineNumber: 1,
  startColumn: 1,
  endColumn: 1,
};

const itemContext = {
  itemDefaults: undefined,
  fallbackRange,
  enums,
  language: "rust",
  rootPath: "/w",
};

// ── URIs ────────────────────────────────────────────────────────────────────

describe("fileUri", () => {
  it("builds a plain file URI", () => {
    expect(fileUri("/home/dev/project/src/main.rs")).toBe(
      "file:///home/dev/project/src/main.rs",
    );
  });

  it("percent-encodes spaces", () => {
    // The URI is the key the server files the document under. An unencoded
    // space makes it unparseable, and every request for that file returns
    // nothing — with no error anywhere.
    expect(fileUri("/Users/dev/My Code/a.rs")).toBe(
      "file:///Users/dev/My%20Code/a.rs",
    );
  });

  it("percent-encodes characters that would change the URI's structure", () => {
    expect(fileUri("/tmp/we#ird?name.rs")).toBe(
      "file:///tmp/we%23ird%3Fname.rs",
    );
  });

  it("encodes non-ASCII as UTF-8 bytes", () => {
    expect(fileUri("/tmp/café.rs")).toBe("file:///tmp/caf%C3%A9.rs");
  });

  it("agrees with the Rust encoder on every shape we send", () => {
    // Mirrors `vibe_lsp::client::tests` exactly; if these two ever drift, the
    // document the editor asks about is not the one the server was told about.
    expect(fileUri("/home/dev/a.rs")).toBe("file:///home/dev/a.rs");
    expect(fileUri("/Users/dev/My Code/b.rs")).toBe(
      "file:///Users/dev/My%20Code/b.rs",
    );
    expect(fileUri("/tmp/café.rs")).toBe("file:///tmp/caf%C3%A9.rs");
    expect(fileUri("/tmp/we#ird.rs")).toBe("file:///tmp/we%23ird.rs");
  });

  it("is accepted by the URL parser", () => {
    for (const path of [
      "/a/b.rs",
      "/a b/c.rs",
      "/a#b/c.rs",
      "/π/λ.rs",
      "/a'b/c.rs",
    ]) {
      expect(() => new URL(fileUri(path))).not.toThrow();
    }
  });

  it("round-trips unchanged through Monaco's own URI parser", () => {
    // This string is handed to <Editor path=…>, which Monaco parses into the
    // model's URI; the bridge then looks a model up by `model.uri.toString()`.
    // If Monaco's encoder disagrees with ours by a single character the lookup
    // misses, the provider finds no document, and completion returns nothing —
    // for exactly the paths with the awkward characters. Uses the real
    // `vscode-uri` implementation Monaco ships (DOM-free, unlike the full
    // `monaco-editor` entry point).
    for (const path of [
      "/w/src/main.rs",
      "/w/My Code/a.rs",
      "/tmp/café.rs",
      "/tmp/we#ird.rs",
      "/tmp/a'b.rs",
      "/tmp/100%.rs",
      "/tmp/a+b&c.rs",
    ]) {
      const ours = fileUri(path);
      expect(URI.parse(ours).toString()).toBe(ours);
    }
  });
});

describe("parentDirectory", () => {
  it("returns the containing directory", () => {
    expect(parentDirectory("/a/b/c.rs")).toBe("/a/b");
  });
  it("returns root for a top-level file", () => {
    expect(parentDirectory("/c.rs")).toBe("/");
  });
});

// ── Language routing ────────────────────────────────────────────────────────

describe("lspLanguageForPath", () => {
  it("routes common languages to their server id", () => {
    expect(lspLanguageForPath("/w/src/main.rs")).toBe("rust");
    expect(lspLanguageForPath("/w/app.tsx")).toBe("typescript");
    expect(lspLanguageForPath("/w/util.js")).toBe("javascript");
    expect(lspLanguageForPath("/w/main.py")).toBe("python");
    expect(lspLanguageForPath("/w/main.go")).toBe("go");
  });

  it("does not route a file whose highlighting language differs", () => {
    // Monaco has no Zig or Nim grammar, so `detectLanguage` reports cpp/python
    // for these. Routing the *server* by that would answer a Zig file with
    // confident C++ completions from clangd.
    expect(lspLanguageForPath("/w/main.zig")).toBe("zig");
    expect(lspLanguageForPath("/w/main.nim")).toBe("nim");
    expect(lspLanguageForPath("/w/main.cr")).toBe("crystal");
    expect(lspLanguageForPath("/w/main.ml")).toBe("ocaml");
    expect(lspLanguageForPath("/w/main.d")).toBe("d");
  });

  it("returns null for files with no language server", () => {
    expect(lspLanguageForPath("/w/notes.txt")).toBeNull();
    expect(lspLanguageForPath("/w/data.csv")).toBeNull();
    expect(lspLanguageForPath("/w/LICENSE")).toBeNull();
  });

  it("recognises extensionless filenames that do have servers", () => {
    expect(lspLanguageForPath("/w/Dockerfile")).toBe("dockerfile");
  });

  it("is case-insensitive on the extension", () => {
    expect(lspLanguageForPath("/w/Main.PY")).toBe("python");
  });
});

/**
 * TIOBE top 30 (April 2026, mirroring `useLanguageRegistry.ts`) → a real
 * filename and the LSP language id it must route to.
 *
 * This is the frontend half of the coverage commitment; `manager.rs` holds the
 * other half (id → server binary). Both are needed: an extension that routes
 * nowhere gets no IntelliSense however many servers are configured, and that
 * was the actual state of MATLAB, Assembly, Ada, PL/SQL, COBOL, SAS and
 * Objective-C files.
 *
 * `null` = no language server exists to route to. Scratch is a block language
 * whose `.sb3` is a zip, and VB6 has no LSP implementation anywhere.
 */
const TIOBE_TOP_30: ReadonlyArray<
  readonly [rank: number, name: string, file: string, language: string | null]
> = [
  [1, "Python", "main.py", "python"],
  [2, "C", "main.c", "c"],
  [3, "C++", "main.cpp", "cpp"],
  [4, "Java", "Main.java", "java"],
  [5, "C#", "Program.cs", "csharp"],
  [6, "JavaScript", "app.js", "javascript"],
  [7, "Visual Basic", "Module.vb", "vb"],
  [8, "SQL", "query.sql", "sql"],
  [9, "R", "analysis.r", "r"],
  [10, "Delphi/Object Pascal", "unit.pas", "pascal"],
  [11, "Scratch", "project.sb3", null],
  [12, "Perl", "script.pl", "perl"],
  [13, "Fortran", "solver.f90", "fortran"],
  [14, "PHP", "index.php", "php"],
  [15, "Go", "main.go", "go"],
  [16, "Rust", "main.rs", "rust"],
  [17, "MATLAB", "model.m", "matlab"],
  [18, "Assembly", "boot.asm", "asm"],
  [19, "Swift", "App.swift", "swift"],
  [20, "Ada", "main.adb", "ada"],
  [21, "PL/SQL", "package.pls", "plsql"],
  [22, "Prolog", "rules.pro", "prolog"],
  [23, "COBOL", "payroll.cbl", "cobol"],
  [24, "Kotlin", "Main.kt", "kotlin"],
  [25, "SAS", "report.sas", "sas"],
  [26, "Classic Visual Basic", "Form1.frm", null],
  [27, "Objective-C", "Bridge.mm", "objective-c"],
  [28, "Dart", "main.dart", "dart"],
  [29, "Ruby", "app.rb", "ruby"],
  [30, "Lua", "init.lua", "lua"],
];

describe("TIOBE top-30 coverage", () => {
  it("routes every top-30 language's files to a language server", () => {
    const unrouted = TIOBE_TOP_30.filter(
      ([, , file, language]) =>
        language !== null && lspLanguageForPath(`/w/${file}`) !== language,
    ).map(
      ([rank, name, file, language]) =>
        `#${rank} ${name}: ${file} → ${lspLanguageForPath(`/w/${file}`)} (want ${language})`,
    );
    expect(unrouted).toEqual([]);
  });

  it("declares the two exemptions deliberately", () => {
    const exempt = TIOBE_TOP_30.filter(([, , , language]) => language === null).map(
      ([, name]) => name,
    );
    expect(exempt).toEqual(["Scratch", "Classic Visual Basic"]);
  });

  it("resolves .pl to Perl and .pro to Prolog", () => {
    // Both languages claim `.pl`. Perl wins (as in every other editor and in
    // GitHub linguist); Prolog reaches its server through `.pro` / `.prolog`.
    // Before this was pinned, every Prolog file was handed to Perl's server.
    expect(lspLanguageForPath("/w/script.pl")).toBe("perl");
    expect(lspLanguageForPath("/w/rules.pro")).toBe("prolog");
    expect(lspLanguageForPath("/w/rules.prolog")).toBe("prolog");
  });

  it("resolves .m to MATLAB and .mm to Objective-C", () => {
    // The same split `detectLanguage` makes for highlighting, so a file never
    // highlights as one language and completes as another.
    expect(lspLanguageForPath("/w/model.m")).toBe("matlab");
    expect(lspLanguageForPath("/w/Bridge.mm")).toBe("objective-c");
  });

  it("keeps SQL dialects distinct so the status bar can name them", () => {
    expect(lspLanguageForPath("/w/q.sql")).toBe("sql");
    expect(lspLanguageForPath("/w/pkg.pls")).toBe("plsql");
    expect(lspLanguageForPath("/w/proc.tsql")).toBe("tsql");
  });
});

describe("triggerCharacters", () => {
  it("splits multi-character server triggers into single characters", () => {
    // Monaco only accepts single characters; `::` passed through verbatim
    // never fires, which is why `Vec::` shows nothing.
    expect(triggerCharacters(["::", "."]).sort()).toEqual([".", ":"]);
  });

  it("falls back to a sane set when the server advertises none", () => {
    expect(triggerCharacters([])).toContain(".");
  });

  it("de-duplicates", () => {
    expect(triggerCharacters([".", ".", "::"])).toEqual([".", ":"]);
  });
});

// ── Completion conversion ───────────────────────────────────────────────────

describe("toMonacoCompletionKind", () => {
  it("maps LSP kinds onto Monaco's by meaning, not by number", () => {
    // The two enums collide numerically: LSP 1 is Text, Monaco 1 is Function.
    expect(toMonacoCompletionKind(1, CompletionItemKind)).toBe(
      CompletionItemKind.Text,
    );
    expect(toMonacoCompletionKind(2, CompletionItemKind)).toBe(
      CompletionItemKind.Method,
    );
    expect(toMonacoCompletionKind(3, CompletionItemKind)).toBe(
      CompletionItemKind.Function,
    );
    expect(toMonacoCompletionKind(15, CompletionItemKind)).toBe(
      CompletionItemKind.Snippet,
    );
    expect(toMonacoCompletionKind(25, CompletionItemKind)).toBe(
      CompletionItemKind.TypeParameter,
    );
  });

  it("proves the numbers really do differ", () => {
    // Guards the premise of this whole mapping: if these ever coincide the
    // mapping is a no-op and the tests above stop meaning anything.
    expect(CompletionItemKind.Text).not.toBe(1);
    expect(CompletionItemKind.Method).toBe(0);
  });

  it("falls back to Text for unknown or missing kinds", () => {
    expect(toMonacoCompletionKind(undefined, CompletionItemKind)).toBe(
      CompletionItemKind.Text,
    );
    expect(toMonacoCompletionKind(999, CompletionItemKind)).toBe(
      CompletionItemKind.Text,
    );
  });
});

describe("toMonacoCompletionItem", () => {
  it("marks snippet items so Monaco expands rather than types the syntax", () => {
    const item = toMonacoCompletionItem(
      {
        label: "call",
        insertText: "call(${1:arg})",
        insertTextFormat: 2,
      },
      itemContext,
    );
    expect(item.insertTextRules).toBe(
      CompletionItemInsertTextRule.InsertAsSnippet,
    );
    expect(item.insertText).toBe("call(${1:arg})");
  });

  it("leaves plain-text items unflagged", () => {
    const item = toMonacoCompletionItem(
      { label: "value", insertText: "value", insertTextFormat: 1 },
      itemContext,
    );
    expect(item.insertTextRules).toBeUndefined();
  });

  it("prefers the textEdit's newText over insertText", () => {
    const item = toMonacoCompletionItem(
      {
        label: "push",
        insertText: "wrong",
        textEdit: { range: range(2, 4, 2, 8), newText: "push" },
      },
      itemContext,
    );
    expect(item.insertText).toBe("push");
    expect(item.range).toEqual({
      startLineNumber: 3,
      startColumn: 5,
      endLineNumber: 3,
      endColumn: 9,
    });
  });

  it("keeps both ranges of an insert/replace edit", () => {
    const item = toMonacoCompletionItem(
      {
        label: "replaceMe",
        textEdit: {
          newText: "replaceMe",
          insert: range(0, 0, 0, 2),
          replace: range(0, 0, 0, 6),
        },
      },
      itemContext,
    );
    expect(item.range).toEqual({
      insert: {
        startLineNumber: 1,
        startColumn: 1,
        endLineNumber: 1,
        endColumn: 3,
      },
      replace: {
        startLineNumber: 1,
        startColumn: 1,
        endLineNumber: 1,
        endColumn: 7,
      },
    });
  });

  it("always has a range — Monaco drops items without one", () => {
    const item = toMonacoCompletionItem({ label: "bare" }, itemContext);
    expect(item.range).toEqual(fallbackRange);
  });

  it("uses the list's itemDefaults editRange when the item has none", () => {
    const item = toMonacoCompletionItem(
      { label: "fromDefaults" },
      { ...itemContext, itemDefaults: { editRange: range(1, 2, 1, 5) } },
    );
    expect(item.range).toEqual({
      startLineNumber: 2,
      startColumn: 3,
      endLineNumber: 2,
      endColumn: 6,
    });
  });

  it("inherits insertTextFormat from itemDefaults", () => {
    const item = toMonacoCompletionItem(
      { label: "snip", insertText: "a(${1})" },
      { ...itemContext, itemDefaults: { insertTextFormat: 2 } },
    );
    expect(item.insertTextRules).toBe(
      CompletionItemInsertTextRule.InsertAsSnippet,
    );
  });

  it("renders markdown documentation as a markdown string", () => {
    const item = toMonacoCompletionItem(
      {
        label: "documented",
        documentation: { kind: "markdown", value: "**bold**" },
      },
      itemContext,
    );
    expect(item.documentation).toEqual({ value: "**bold**", isTrusted: false });
  });

  it("passes plaintext documentation through as a plain string", () => {
    const item = toMonacoCompletionItem(
      {
        label: "documented",
        documentation: { kind: "plaintext", value: "just text" },
      },
      itemContext,
    );
    expect(item.documentation).toBe("just text");
  });

  it("drops empty documentation so resolve is attempted instead", () => {
    const item = toMonacoCompletionItem(
      { label: "x", documentation: { kind: "markdown", value: "" } },
      itemContext,
    );
    expect(item.documentation).toBeUndefined();
  });

  it("tags deprecated items, from either the flag or the tag list", () => {
    const viaTag = toMonacoCompletionItem(
      { label: "old", tags: [1] },
      itemContext,
    );
    const viaFlag = toMonacoCompletionItem(
      { label: "old", deprecated: true },
      itemContext,
    );
    expect(viaTag.tags).toEqual([CompletionItemTag.Deprecated]);
    expect(viaFlag.tags).toEqual([CompletionItemTag.Deprecated]);
  });

  it("falls back to labelDetails when detail is absent", () => {
    const item = toMonacoCompletionItem(
      { label: "fmt", labelDetails: { detail: "(args: T)" } },
      itemContext,
    );
    expect(item.detail).toBe("(args: T)");
  });

  it("carries sortText, filterText, preselect and commit characters", () => {
    const item = toMonacoCompletionItem(
      {
        label: "z_last",
        sortText: "000",
        filterText: "zl",
        preselect: true,
        commitCharacters: ["("],
      },
      itemContext,
    );
    expect(item.sortText).toBe("000");
    expect(item.filterText).toBe("zl");
    expect(item.preselect).toBe(true);
    expect(item.commitCharacters).toEqual(["("]);
  });

  it("converts additionalTextEdits (auto-imports) into Monaco edits", () => {
    const item = toMonacoCompletionItem(
      {
        label: "HashMap",
        additionalTextEdits: [
          { range: range(0, 0, 0, 0), newText: "use std::collections::HashMap;\n" },
        ],
      },
      itemContext,
    );
    expect(item.additionalTextEdits).toEqual([
      {
        range: {
          startLineNumber: 1,
          startColumn: 1,
          endLineNumber: 1,
          endColumn: 1,
        },
        text: "use std::collections::HashMap;\n",
      },
    ]);
  });

  it("keeps the raw LSP item for resolve", () => {
    const lspItem = { label: "push", kind: 2, data: { id: 42 } };
    const item = toMonacoCompletionItem(lspItem, itemContext);
    expect(item.__lsp).toBe(lspItem);
    expect(item.__language).toBe("rust");
  });
});

describe("toMonacoCompletionList", () => {
  const listContext = {
    fallbackRange,
    enums,
    language: "rust",
    rootPath: "/w",
  };

  it("accepts a bare array of items", () => {
    const list = toMonacoCompletionList([{ label: "a" }, { label: "b" }], listContext);
    expect(list.suggestions.map((s) => s.label)).toEqual(["a", "b"]);
    expect(list.incomplete).toBe(false);
  });

  it("accepts a CompletionList and preserves isIncomplete", () => {
    const list = toMonacoCompletionList(
      { isIncomplete: true, items: [{ label: "a" }] },
      listContext,
    );
    expect(list.incomplete).toBe(true);
  });

  it("returns no suggestions for a null response", () => {
    expect(toMonacoCompletionList(null, listContext).suggestions).toEqual([]);
  });

  it("survives a list with a missing items array", () => {
    const list = toMonacoCompletionList(
      { isIncomplete: false } as never,
      listContext,
    );
    expect(list.suggestions).toEqual([]);
  });
});

// ── Hover / definition / diagnostics / signature help ───────────────────────

describe("toMonacoHover", () => {
  it("handles MarkupContent", () => {
    const hover = toMonacoHover({
      contents: { kind: "markdown", value: "`fn main()`" },
    });
    expect(hover?.contents).toEqual([{ value: "`fn main()`" }]);
  });

  it("handles a bare string", () => {
    expect(toMonacoHover({ contents: "plain" })?.contents).toEqual([
      { value: "plain" },
    ]);
  });

  it("wraps MarkedString code in a fenced block for its language", () => {
    const hover = toMonacoHover({
      contents: [{ language: "rust", value: "fn main()" }, "and a note"],
    });
    expect(hover?.contents).toEqual([
      { value: "```rust\nfn main()\n```" },
      { value: "and a note" },
    ]);
  });

  it("converts the hover range", () => {
    const hover = toMonacoHover({
      contents: "x",
      range: range(3, 1, 3, 5),
    });
    expect(hover?.range).toEqual({
      startLineNumber: 4,
      startColumn: 2,
      endLineNumber: 4,
      endColumn: 6,
    });
  });

  it("returns null for empty or absent content rather than an empty tooltip", () => {
    expect(toMonacoHover(null)).toBeNull();
    expect(toMonacoHover({ contents: "" })).toBeNull();
    expect(toMonacoHover({ contents: [] })).toBeNull();
    expect(toMonacoHover({ contents: { kind: "markdown", value: "  " } })).toBeNull();
  });
});

describe("toMonacoLocations", () => {
  const parseUri = (uri: string) => ({ toString: () => uri }) as never;

  it("handles a single Location", () => {
    const locations = toMonacoLocations(
      { uri: "file:///w/a.rs", range: range(4, 2, 4, 8) },
      parseUri,
    );
    expect(locations).toHaveLength(1);
    expect(locations[0].range).toEqual({
      startLineNumber: 5,
      startColumn: 3,
      endLineNumber: 5,
      endColumn: 9,
    });
  });

  it("handles an array of Locations", () => {
    const locations = toMonacoLocations(
      [
        { uri: "file:///w/a.rs", range: range(0, 0, 0, 1) },
        { uri: "file:///w/b.rs", range: range(1, 0, 1, 1) },
      ],
      parseUri,
    );
    expect(locations).toHaveLength(2);
  });

  it("handles LocationLinks, preferring the selection range", () => {
    const locations = toMonacoLocations(
      [
        {
          targetUri: "file:///w/a.rs",
          targetRange: range(0, 0, 9, 0),
          targetSelectionRange: range(3, 4, 3, 9),
        },
      ],
      parseUri,
    );
    expect(locations[0].range.startLineNumber).toBe(4);
  });

  it("returns nothing for a null response", () => {
    expect(toMonacoLocations(null, parseUri)).toEqual([]);
  });
});

describe("toMonacoMarkers", () => {
  it("maps LSP severities onto Monaco's", () => {
    expect(toMonacoSeverity(1, MarkerSeverity)).toBe(MarkerSeverity.Error);
    expect(toMonacoSeverity(2, MarkerSeverity)).toBe(MarkerSeverity.Warning);
    expect(toMonacoSeverity(3, MarkerSeverity)).toBe(MarkerSeverity.Info);
    expect(toMonacoSeverity(4, MarkerSeverity)).toBe(MarkerSeverity.Hint);
  });

  it("treats a missing severity as an error, per the LSP spec", () => {
    expect(toMonacoSeverity(undefined, MarkerSeverity)).toBe(
      MarkerSeverity.Error,
    );
  });

  it("converts a diagnostic into a marker", () => {
    const [marker] = toMonacoMarkers(
      [
        {
          range: range(9, 4, 9, 12),
          severity: 1,
          message: "cannot find value `x`",
          source: "rustc",
          code: "E0425",
        },
      ],
      enums,
    );
    expect(marker).toMatchObject({
      startLineNumber: 10,
      startColumn: 5,
      endLineNumber: 10,
      endColumn: 13,
      message: "cannot find value `x`",
      severity: MarkerSeverity.Error,
      source: "rustc",
      code: "E0425",
    });
  });

  it("widens a zero-width range so the squiggle is visible", () => {
    // Servers report point diagnostics (missing semicolon, unexpected EOF);
    // a zero-width marker draws nothing at all.
    const [marker] = toMonacoMarkers(
      [{ range: range(2, 7, 2, 7), message: "expected `;`" }],
      enums,
    );
    expect(marker.endColumn).toBe(marker.startColumn + 1);
  });

  it("stringifies numeric codes", () => {
    const [marker] = toMonacoMarkers(
      [{ range: range(0, 0, 0, 1), message: "m", code: 2304 }],
      enums,
    );
    expect(marker.code).toBe("2304");
  });

  it("converts an empty diagnostic list to no markers", () => {
    expect(toMonacoMarkers([], enums)).toEqual([]);
  });
});

describe("toMonacoSignatureHelp", () => {
  it("converts signatures and active indices", () => {
    const help = toMonacoSignatureHelp({
      signatures: [
        {
          label: "fn push(&mut self, value: T)",
          documentation: { kind: "markdown", value: "Appends." },
          parameters: [{ label: "value: T" }],
        },
      ],
      activeSignature: 0,
      activeParameter: 0,
    });
    expect(help?.signatures[0].label).toBe("fn push(&mut self, value: T)");
    expect(help?.signatures[0].parameters).toEqual([
      { label: "value: T", documentation: undefined },
    ]);
    expect(help?.activeParameter).toBe(0);
  });

  it("returns null when there is nothing to show", () => {
    expect(toMonacoSignatureHelp(null)).toBeNull();
    expect(toMonacoSignatureHelp({ signatures: [] })).toBeNull();
  });
});

describe("coordinate systems", () => {
  it("converts LSP 0-based ranges to Monaco 1-based", () => {
    expect(toMonacoRange(range(0, 0, 2, 5))).toEqual({
      startLineNumber: 1,
      startColumn: 1,
      endLineNumber: 3,
      endColumn: 6,
    });
  });

  it("converts Monaco 1-based positions back to LSP 0-based", () => {
    expect(toLspPosition({ lineNumber: 1, column: 1 })).toEqual({
      line: 0,
      character: 0,
    });
  });

  it("round-trips a position", () => {
    const monacoPosition = { lineNumber: 12, column: 34 };
    const lsp = toLspPosition(monacoPosition);
    const back = toMonacoRange({ start: lsp, end: lsp });
    expect(back.startLineNumber).toBe(monacoPosition.lineNumber);
    expect(back.startColumn).toBe(monacoPosition.column);
  });
});

// ── The bridge ──────────────────────────────────────────────────────────────

interface FakeModel {
  uri: { toString(): string };
  isDisposed(): boolean;
  getWordUntilPosition(): {
    word: string;
    startColumn: number;
    endColumn: number;
  };
}

/**
 * A Monaco stand-in that records provider registrations and marker writes.
 * Only the surface `createLspBridge` touches.
 */
function fakeMonaco() {
  const providers = {
    completion: [] as Array<{
      language: string;
      triggerCharacters?: string[];
      provideCompletionItems: (
        model: FakeModel,
        position: { lineNumber: number; column: number },
      ) => Promise<{ suggestions: Array<{ label: string }> }>;
      resolveCompletionItem?: (item: unknown) => Promise<unknown>;
    }>,
    hover: [] as Array<{
      language: string;
      provideHover: (
        model: FakeModel,
        position: { lineNumber: number; column: number },
      ) => Promise<unknown>;
    }>,
    definition: [] as Array<{ language: string }>,
    signatureHelp: [] as Array<{ language: string }>,
  };
  const disposed: string[] = [];
  const markers: Array<{ owner: string; count: number }> = [];
  const models = new Map<string, FakeModel>();

  const disposable = (tag: string) => ({
    dispose: () => disposed.push(tag),
  });

  const monaco = {
    languages: {
      CompletionItemKind,
      CompletionItemInsertTextRule,
      CompletionItemTag,
      registerCompletionItemProvider: (language: string, provider: never) => {
        providers.completion.push({ language, ...(provider as object) } as never);
        return disposable(`completion:${language}`);
      },
      registerHoverProvider: (language: string, provider: never) => {
        providers.hover.push({ language, ...(provider as object) } as never);
        return disposable(`hover:${language}`);
      },
      registerDefinitionProvider: (language: string) => {
        providers.definition.push({ language });
        return disposable(`definition:${language}`);
      },
      registerSignatureHelpProvider: (language: string) => {
        providers.signatureHelp.push({ language });
        return disposable(`signature:${language}`);
      },
    },
    MarkerSeverity,
    Uri: {
      // Enough of vscode-uri for identity: the bridge only needs a stable key.
      parse: (value: string) => ({ toString: () => value }),
      file: (value: string) => ({ toString: () => `file://${value}` }),
    },
    editor: {
      getModel: (uri: { toString(): string }) => models.get(uri.toString()) ?? null,
      setModelMarkers: (
        _model: FakeModel,
        owner: string,
        data: readonly unknown[],
      ) => markers.push({ owner, count: data.length }),
    },
  };

  const addModel = (uri: string): FakeModel => {
    const model: FakeModel = {
      uri: { toString: () => uri },
      isDisposed: () => false,
      getWordUntilPosition: () => ({ word: "", startColumn: 1, endColumn: 1 }),
    };
    models.set(uri, model);
    return model;
  };

  return { monaco: monaco as never, providers, disposed, markers, addModel };
}

const RUST_SUPPORT = {
  language: "rust",
  state: "running" as const,
  detail: "",
  supported: true,
  completionTriggerCharacters: [".", "::"],
  signatureHelpTriggerCharacters: ["(", ","],
};

describe("createLspBridge", () => {
  let bridge: LspBridge | null = null;

  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    bridge?.dispose();
    bridge = null;
    vi.useRealTimers();
  });

  /** An invoke stub with per-command canned answers and a call log. */
  function stubInvoke(
    handlers: Record<string, (args: Record<string, unknown>) => unknown> = {},
  ) {
    const calls: Array<{ command: string; args: Record<string, unknown> }> = [];
    const invoke = vi.fn(
      async (command: string, args: Record<string, unknown> = {}) => {
        calls.push({ command, args });
        const handler = handlers[command];
        if (handler) return handler(args);
        if (command === "lsp_language_support") return RUST_SUPPORT;
        return null;
      },
    );
    return { invoke: invoke as never, calls };
  }

  it("opens a document with the same URI the providers will use", async () => {
    const { monaco } = fakeMonaco();
    const { invoke, calls } = stubInvoke();
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });

    await bridge.openDocument("/w/src/main.rs", "rust", "fn main() {}");

    const open = calls.find((c) => c.command === "lsp_did_open");
    expect(open?.args).toMatchObject({
      language: "rust",
      rootPath: "/w",
      uri: "file:///w/src/main.rs",
      text: "fn main() {}",
    });
  });

  it("registers providers for the Monaco language with the server's triggers", async () => {
    const { monaco, providers } = fakeMonaco();
    const { invoke } = stubInvoke();
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });

    await bridge.openDocument("/w/src/main.rs", "rust", "");

    expect(providers.completion).toHaveLength(1);
    expect(providers.completion[0].language).toBe("rust");
    expect(providers.completion[0].triggerCharacters?.sort()).toEqual([
      ".",
      ":",
    ]);
    expect(providers.hover).toHaveLength(1);
    expect(providers.definition).toHaveLength(1);
    expect(providers.signatureHelp).toHaveLength(1);
  });

  it("registers providers under the Monaco language, not the server language", async () => {
    // A `.zig` file highlights as `cpp`, so the provider must be attached to
    // `cpp` or it never runs — while still asking the *zig* server.
    const { monaco, providers } = fakeMonaco();
    const { invoke, calls } = stubInvoke({
      lsp_language_support: () => ({ ...RUST_SUPPORT, language: "zig" }),
    });
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });

    await bridge.openDocument("/w/main.zig", "cpp", "");

    expect(providers.completion[0].language).toBe("cpp");
    expect(
      calls.find((c) => c.command === "lsp_did_open")?.args.language,
    ).toBe("zig");
  });

  it("skips languages with no server and never asks again", async () => {
    const { monaco, providers } = fakeMonaco();
    const { invoke, calls } = stubInvoke({
      lsp_language_support: () => ({
        language: "rust",
        state: "unconfigured",
        detail: "",
        supported: false,
        completionTriggerCharacters: [],
        signatureHelpTriggerCharacters: [],
      }),
    });
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });

    await bridge.openDocument("/w/a.rs", "rust", "");
    await bridge.openDocument("/w/b.rs", "rust", "");

    expect(providers.completion).toHaveLength(0);
    expect(
      calls.filter((c) => c.command === "lsp_language_support"),
    ).toHaveLength(1);
  });

  it("leaves Monaco's own languages alone rather than duplicating them", async () => {
    // Monaco's providers cannot be unregistered, so registering ours for
    // TypeScript would show every suggestion twice.
    const { monaco, providers } = fakeMonaco();
    const { invoke, calls } = stubInvoke();
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });

    await bridge.openDocument("/w/src/App.tsx", "typescript", "");
    await bridge.openDocument("/w/style.css", "css", "");
    await bridge.openDocument("/w/data.json", "json", "");

    expect(providers.completion).toHaveLength(0);
    // And no server is started for them either — tsserver on a large repo is
    // hundreds of megabytes we would never read a suggestion from.
    expect(calls).toEqual([]);
  });

  it("does not touch the backend for a file with no possible server", async () => {
    const { monaco } = fakeMonaco();
    const { invoke, calls } = stubInvoke();
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });

    await bridge.openDocument("/w/notes.txt", "plaintext", "hello");

    expect(calls).toEqual([]);
  });

  it("reports an unavailable server once, with the install hint", async () => {
    const { monaco } = fakeMonaco();
    const onLanguageUnavailable = vi.fn();
    const { invoke } = stubInvoke({
      lsp_language_support: () => ({
        language: "rust",
        state: "not_installed",
        detail: "rust-analyzer — install: rustup component add rust-analyzer",
        supported: true,
        completionTriggerCharacters: [],
        signatureHelpTriggerCharacters: [],
      }),
    });
    bridge = createLspBridge(monaco, {
      invoke,
      getWorkspaceRoot: () => "/w",
      onLanguageUnavailable,
    });

    await bridge.openDocument("/w/a.rs", "rust", "");

    expect(onLanguageUnavailable).toHaveBeenCalledTimes(1);
    expect(onLanguageUnavailable.mock.calls[0][0].detail).toContain("rustup");
  });

  it("uses the file's own directory as the root when no folder is open", async () => {
    // Opening a single file still deserves IntelliSense.
    const { monaco } = fakeMonaco();
    const { invoke, calls } = stubInvoke();
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "" });

    await bridge.openDocument("/elsewhere/scratch/main.rs", "rust", "");

    expect(calls.find((c) => c.command === "lsp_did_open")?.args.rootPath).toBe(
      "/elsewhere/scratch",
    );
  });

  it("reads the workspace root at request time, not at construction", async () => {
    // Providers are registered once at mount, when no folder is open yet.
    const { monaco } = fakeMonaco();
    const { invoke, calls } = stubInvoke();
    let root = "";
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => root });

    root = "/opened/later";
    await bridge.openDocument("/opened/later/src/main.rs", "rust", "");

    expect(calls.find((c) => c.command === "lsp_did_open")?.args.rootPath).toBe(
      "/opened/later",
    );
  });

  it("debounces edits into a single didChange", async () => {
    const { monaco } = fakeMonaco();
    const { invoke, calls } = stubInvoke();
    bridge = createLspBridge(monaco, {
      invoke,
      getWorkspaceRoot: () => "/w",
      changeDebounceMs: 50,
    });
    await bridge.openDocument("/w/a.rs", "rust", "v0");

    bridge.changeDocument("/w/a.rs", "v1");
    bridge.changeDocument("/w/a.rs", "v2");
    bridge.changeDocument("/w/a.rs", "v3");
    expect(calls.filter((c) => c.command === "lsp_did_change")).toHaveLength(0);

    await vi.advanceTimersByTimeAsync(60);

    const changes = calls.filter((c) => c.command === "lsp_did_change");
    expect(changes).toHaveLength(1);
    expect(changes[0].args.text).toBe("v3");
  });

  it("flushes the pending edit before answering a completion", async () => {
    // The whole point: Monaco asks for completions immediately after a
    // keystroke. If the debounce were still pending the server would answer
    // against the previous text and the symbol just typed would be missing.
    const { monaco, providers, addModel } = fakeMonaco();
    const { invoke, calls } = stubInvoke({
      lsp_completion: () => [{ label: "from_server" }],
    });
    bridge = createLspBridge(monaco, {
      invoke,
      getWorkspaceRoot: () => "/w",
      changeDebounceMs: 5000,
    });
    await bridge.openDocument("/w/a.rs", "rust", "let a");
    const model = addModel("file:///w/a.rs");

    bridge.changeDocument("/w/a.rs", "let ab");
    const result = await providers.completion[0].provideCompletionItems(model, {
      lineNumber: 1,
      column: 7,
    });

    const commands = calls.map((c) => c.command);
    expect(commands.indexOf("lsp_did_change")).toBeLessThan(
      commands.indexOf("lsp_completion"),
    );
    expect(
      calls.find((c) => c.command === "lsp_did_change")?.args.text,
    ).toBe("let ab");
    expect(result.suggestions.map((s) => s.label)).toEqual(["from_server"]);
  });

  it("sends the document's own URI and 0-based position with a completion", async () => {
    const { monaco, providers, addModel } = fakeMonaco();
    const { invoke, calls } = stubInvoke({ lsp_completion: () => [] });
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });
    await bridge.openDocument("/w/My Code/a.rs", "rust", "");
    const model = addModel("file:///w/My%20Code/a.rs");

    await providers.completion[0].provideCompletionItems(model, {
      lineNumber: 4,
      column: 9,
    });

    const params = calls.find((c) => c.command === "lsp_completion")?.args
      .params as { textDocument: { uri: string }; position: unknown };
    expect(params.textDocument.uri).toBe("file:///w/My%20Code/a.rs");
    expect(params.position).toEqual({ line: 3, character: 8 });
  });

  it("returns no suggestions for a model it does not track", async () => {
    const { monaco, providers, addModel } = fakeMonaco();
    const { invoke, calls } = stubInvoke();
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });
    await bridge.openDocument("/w/a.rs", "rust", "");
    const stranger = addModel("inmemory://model/7");

    const result = await providers.completion[0].provideCompletionItems(
      stranger,
      { lineNumber: 1, column: 1 },
    );

    expect(result.suggestions).toEqual([]);
    expect(calls.some((c) => c.command === "lsp_completion")).toBe(false);
  });

  it("survives a completion request that fails", async () => {
    const { monaco, providers, addModel } = fakeMonaco();
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const { invoke } = stubInvoke({
      lsp_completion: () => {
        throw new Error("server exited");
      },
    });
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });
    await bridge.openDocument("/w/a.rs", "rust", "");
    const model = addModel("file:///w/a.rs");

    const result = await providers.completion[0].provideCompletionItems(model, {
      lineNumber: 1,
      column: 1,
    });

    expect(result.suggestions).toEqual([]);
    expect(warn).toHaveBeenCalled();
    warn.mockRestore();
  });

  it("publishes diagnostics as markers under its own owner", async () => {
    const { monaco, markers, addModel } = fakeMonaco();
    addModel("file:///w/a.rs");
    const { invoke } = stubInvoke({
      lsp_diagnostics: () => [
        { range: range(0, 0, 0, 4), severity: 1, message: "boom" },
      ],
    });
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });

    await bridge.openDocument("/w/a.rs", "rust", "");
    await vi.advanceTimersByTimeAsync(300);

    expect(markers.length).toBeGreaterThan(0);
    expect(markers[0]).toEqual({ owner: MARKER_OWNER, count: 1 });
  });

  it("leaves markers alone when the server has published nothing yet", async () => {
    // `null` means "no data yet", not "the file is clean". Clearing on it would
    // wipe real errors on every early poll.
    const { monaco, markers, addModel } = fakeMonaco();
    addModel("file:///w/a.rs");
    const { invoke } = stubInvoke({ lsp_diagnostics: () => null });
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });

    await bridge.openDocument("/w/a.rs", "rust", "");
    await vi.advanceTimersByTimeAsync(6000);

    expect(markers).toEqual([]);
  });

  it("clears markers on an empty diagnostic list", async () => {
    const { monaco, markers, addModel } = fakeMonaco();
    addModel("file:///w/a.rs");
    const { invoke } = stubInvoke({ lsp_diagnostics: () => [] });
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });

    await bridge.openDocument("/w/a.rs", "rust", "");
    await vi.advanceTimersByTimeAsync(300);

    expect(markers[0]).toEqual({ owner: MARKER_OWNER, count: 0 });
  });

  it("stops polling diagnostics once the bursts elapse", async () => {
    // A standing interval would burn CPU forever while the user just reads.
    const { monaco, addModel } = fakeMonaco();
    addModel("file:///w/a.rs");
    const { invoke, calls } = stubInvoke({ lsp_diagnostics: () => [] });
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });

    await bridge.openDocument("/w/a.rs", "rust", "");
    await vi.advanceTimersByTimeAsync(10_000);
    const afterBurst = calls.filter((c) => c.command === "lsp_diagnostics").length;
    await vi.advanceTimersByTimeAsync(60_000);

    expect(
      calls.filter((c) => c.command === "lsp_diagnostics"),
    ).toHaveLength(afterBurst);
  });

  it("flushes pending edits before didSave", async () => {
    const { monaco } = fakeMonaco();
    const { invoke, calls } = stubInvoke();
    bridge = createLspBridge(monaco, {
      invoke,
      getWorkspaceRoot: () => "/w",
      changeDebounceMs: 5000,
    });
    await bridge.openDocument("/w/a.rs", "rust", "v0");

    bridge.changeDocument("/w/a.rs", "v1");
    await bridge.saveDocument("/w/a.rs");

    const commands = calls.map((c) => c.command);
    expect(commands.indexOf("lsp_did_change")).toBeLessThan(
      commands.indexOf("lsp_did_save"),
    );
  });

  it("closes the document and clears its markers", async () => {
    const { monaco, markers, addModel } = fakeMonaco();
    addModel("file:///w/a.rs");
    const { invoke, calls } = stubInvoke();
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });
    await bridge.openDocument("/w/a.rs", "rust", "");

    await bridge.closeDocument("/w/a.rs");

    expect(calls.find((c) => c.command === "lsp_did_close")?.args.uri).toBe(
      "file:///w/a.rs",
    );
    expect(markers.at(-1)).toEqual({ owner: MARKER_OWNER, count: 0 });
    expect(bridge.languageFor("/w/a.rs")).toBeUndefined();
  });

  it("does not send edits for a closed document", async () => {
    const { monaco } = fakeMonaco();
    const { invoke, calls } = stubInvoke();
    bridge = createLspBridge(monaco, {
      invoke,
      getWorkspaceRoot: () => "/w",
      changeDebounceMs: 10,
    });
    await bridge.openDocument("/w/a.rs", "rust", "v0");
    await bridge.closeDocument("/w/a.rs");

    bridge.changeDocument("/w/a.rs", "v1");
    await vi.advanceTimersByTimeAsync(50);

    expect(calls.some((c) => c.command === "lsp_did_change")).toBe(false);
  });

  it("resolves a completion item for its documentation", async () => {
    const { monaco, providers, addModel } = fakeMonaco();
    const { invoke, calls } = stubInvoke({
      lsp_completion: () => [{ label: "push", kind: 2, data: { id: 1 } }],
      lsp_resolve_completion: () => ({
        label: "push",
        documentation: { kind: "markdown", value: "Appends an element." },
      }),
    });
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });
    await bridge.openDocument("/w/a.rs", "rust", "");
    const model = addModel("file:///w/a.rs");

    const list = await providers.completion[0].provideCompletionItems(model, {
      lineNumber: 1,
      column: 1,
    });
    const resolved = (await providers.completion[0].resolveCompletionItem?.(
      list.suggestions[0],
    )) as { documentation?: { value: string } };

    expect(resolved.documentation).toEqual({
      value: "Appends an element.",
      isTrusted: false,
    });
    expect(
      calls.find((c) => c.command === "lsp_resolve_completion")?.args.item,
    ).toMatchObject({ label: "push", data: { id: 1 } });
  });

  it("does not re-resolve an item that already has documentation", async () => {
    const { monaco, providers, addModel } = fakeMonaco();
    const { invoke, calls } = stubInvoke({
      lsp_completion: () => [
        {
          label: "push",
          documentation: { kind: "markdown", value: "already here" },
        },
      ],
    });
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });
    await bridge.openDocument("/w/a.rs", "rust", "");
    const model = addModel("file:///w/a.rs");

    const list = await providers.completion[0].provideCompletionItems(model, {
      lineNumber: 1,
      column: 1,
    });
    await providers.completion[0].resolveCompletionItem?.(list.suggestions[0]);

    expect(calls.some((c) => c.command === "lsp_resolve_completion")).toBe(
      false,
    );
  });

  it("registers a language's providers only once across many files", async () => {
    const { monaco, providers } = fakeMonaco();
    const { invoke } = stubInvoke();
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });

    await bridge.openDocument("/w/a.rs", "rust", "");
    await bridge.openDocument("/w/b.rs", "rust", "");
    await bridge.openDocument("/w/c.rs", "rust", "");

    expect(providers.completion).toHaveLength(1);
  });

  it("re-registers when a second language widens the trigger set", async () => {
    // Two LSP languages can share one Monaco language id, and Monaco cannot
    // add a trigger character to a live provider.
    const { monaco, providers, disposed } = fakeMonaco();
    const { invoke } = stubInvoke({
      lsp_language_support: (args) =>
        args.language === "c"
          ? { ...RUST_SUPPORT, language: "c", completionTriggerCharacters: ["."] }
          : {
              ...RUST_SUPPORT,
              language: "zig",
              completionTriggerCharacters: ["@", "."],
            },
    });
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });

    await bridge.openDocument("/w/a.c", "cpp", "");
    expect(providers.completion).toHaveLength(1);

    await bridge.openDocument("/w/b.zig", "cpp", "");
    expect(providers.completion).toHaveLength(2);
    expect(disposed).toContain("completion:cpp");
    expect(providers.completion[1].triggerCharacters?.sort()).toEqual([
      ".",
      "@",
    ]);
  });

  it("disposes every provider and timer on teardown", async () => {
    const { monaco, disposed } = fakeMonaco();
    const { invoke, calls } = stubInvoke({ lsp_diagnostics: () => [] });
    const local = createLspBridge(monaco, {
      invoke,
      getWorkspaceRoot: () => "/w",
    });
    await local.openDocument("/w/a.rs", "rust", "");

    local.dispose();
    const before = calls.length;
    await vi.advanceTimersByTimeAsync(10_000);

    expect(disposed).toEqual(
      expect.arrayContaining([
        "completion:rust",
        "hover:rust",
        "definition:rust",
        "signature:rust",
      ]),
    );
    expect(calls).toHaveLength(before);
  });

  it("keeps working when didOpen fails, so a later retry can succeed", async () => {
    const { monaco, providers } = fakeMonaco();
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const { invoke } = stubInvoke({
      lsp_did_open: () => {
        throw new Error("rust-analyzer is not installed");
      },
    });
    bridge = createLspBridge(monaco, { invoke, getWorkspaceRoot: () => "/w" });

    await bridge.openDocument("/w/a.rs", "rust", "");

    expect(providers.completion).toHaveLength(1);
    warn.mockRestore();
  });
});
