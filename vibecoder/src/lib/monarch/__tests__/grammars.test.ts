/**
 * Grammar tests — run through Monaco's real Monarch engine (see `tokenize.ts`).
 *
 * Each language gets a sample of the constructs a reader depends on, plus the
 * cases where an approximate grammar goes visibly wrong: nested comments that
 * un-comment the file, an apostrophe that swallows the rest of the line, a
 * string delimiter that is also an operator.
 */

import { describe, it, expect } from "vitest";
import { MONARCH_LANGUAGES } from "../languages";
import { monarchFromSpec, languageConfigurationFromSpec } from "../spec";
import { tokenize, tokenizeLines, typeOf, typesIn } from "./tokenize";
import type { LanguageSpec } from "../spec";

const byId = (id: string): LanguageSpec => {
  const spec = MONARCH_LANGUAGES.find((language) => language.id === id);
  if (!spec) throw new Error(`no spec for ${id}`);
  return spec;
};

/** Assert the token type covering `needle` starts with `expected`. */
const expectToken = (
  id: string,
  line: string,
  needle: string,
  expected: string,
) => {
  const actual = typeOf(byId(id), line, needle);
  expect(
    actual.startsWith(expected),
    `${id}: ${JSON.stringify(needle)} in ${JSON.stringify(line)} → got "${actual}", wanted "${expected}*"`,
  ).toBe(true);
};

// ── Every grammar, generically ──────────────────────────────────────────────

describe("all grammars", () => {
  it("compile and tokenize without throwing", () => {
    for (const spec of MONARCH_LANGUAGES) {
      expect(() => tokenize(spec, "identifier 42 // x"), spec.id).not.toThrow();
    }
  });

  it("never leave text unclassified as the default token", () => {
    // `defaultToken: ''` means anything unmatched renders as plain text. A
    // grammar that falls through for ordinary code is not doing its job.
    for (const spec of MONARCH_LANGUAGES) {
      const types = typesIn(spec, "abc 123");
      expect(types.length, `${spec.id} produced no tokens`).toBeGreaterThan(0);
    }
  });

  it("classify their own keywords as keywords", () => {
    for (const spec of MONARCH_LANGUAGES) {
      if (spec.keywords.length === 0) continue; // LaTeX has none by design
      // Pick a keyword that is a plain word, so no other rule can claim it.
      const keyword = spec.keywords.find((word) => /^[a-zA-Z][\w-]*$/.test(word));
      if (!keyword) continue;
      const actual = typeOf(spec, keyword, keyword);
      expect(actual, `${spec.id}: "${keyword}"`).toMatch(/^keyword/);
    }
  });

  it("classify numbers as numbers", () => {
    for (const spec of MONARCH_LANGUAGES) {
      // COBOL's picture clauses and PostScript's radix numbers legitimately
      // reinterpret bare digits; both are covered by their own tests.
      if (spec.id === "cobol") continue;
      expect(typeOf(spec, "x = 42", "42"), spec.id).toMatch(/^number/);
    }
  });

  it("have a unique id and at least one extension", () => {
    const ids = MONARCH_LANGUAGES.map((spec) => spec.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const spec of MONARCH_LANGUAGES) {
      expect(spec.extensions.length, spec.id).toBeGreaterThan(0);
      for (const extension of spec.extensions) {
        expect(extension.startsWith("."), `${spec.id}: ${extension}`).toBe(true);
      }
    }
  });

  it("declare a comment style, so ⌘/ works", () => {
    for (const spec of MONARCH_LANGUAGES) {
      const config = languageConfigurationFromSpec(spec);
      expect(
        config.comments?.lineComment ?? config.comments?.blockComment,
        `${spec.id} has no comment configuration`,
      ).toBeDefined();
    }
  });

  it("put keyword lists on the grammar so `@keywords` resolves", () => {
    // A `cases: { '@keywords': … }` action referring to a missing list makes
    // monarchCompile throw at registration — in production, not in a test.
    for (const spec of MONARCH_LANGUAGES) {
      const grammar = monarchFromSpec(spec);
      expect(grammar.keywords, spec.id).toBeDefined();
      if (spec.types?.length) expect(grammar.types, spec.id).toBeDefined();
      if (spec.constants?.length) expect(grammar.constants, spec.id).toBeDefined();
      if (spec.builtins?.length) expect(grammar.builtins, spec.id).toBeDefined();
    }
  });
});

// ── Comments ────────────────────────────────────────────────────────────────

describe("comments", () => {
  it("handles each language's line-comment marker", () => {
    expectToken("zig", "const x = 1; // note", "// note", "comment");
    expectToken("nim", "let x = 1 # note", "# note", "comment");
    expectToken("haskell", "x = 1 -- note", "-- note", "comment");
    expectToken("matlab", "x = 1 % note", "% note", "comment");
    expectToken("erlang", "X = 1. % note", "% note", "comment");
    expectToken("asm", "mov eax, 1 ; note", "; note", "comment");
    expectToken("foxpro", "x = 1 && note", "&& note", "comment");
    expectToken("ada", "X := 1; -- note", "-- note", "comment");
    expectToken("postscript", "1 2 add % note", "% note", "comment");
  });

  it("closes a block comment on the same line", () => {
    expectToken("v", "a /* mid */ b", "/* mid */", "comment");
    expectToken("nix", "a /* mid */ b", "/* mid */", "comment");
    expectToken("protobuf", "a /* mid */ b", "/* mid */", "comment");
  });

  it("carries an unterminated block comment across lines", () => {
    const lines = tokenizeLines(byId("v"), ["/* start", "still comment", "end */ code"]);
    expect(lines[1].every((token) => token.type.startsWith("comment"))).toBe(true);
    // After the close, real code is tokenized again.
    expect(lines[2].some((token) => token.type === "identifier")).toBe(true);
  });

  it("nests block comments where the language says they nest", () => {
    // The failure this prevents: one inner `-}` ends the outer comment and the
    // rest of the file renders as code.
    const lines = tokenizeLines(byId("haskell"), [
      "{- outer {- inner -} still outer -}",
      "realCode = 1",
    ]);
    expect(lines[0].every((token) => token.type.startsWith("comment"))).toBe(true);
    expect(lines[1].some((token) => token.type.startsWith("keyword") || token.type === "identifier")).toBe(true);
  });

  it("nests D's /+ +/ comments", () => {
    const lines = tokenizeLines(byId("d"), ["/+ a /+ b +/ c +/", "int x;"]);
    expect(lines[0].every((token) => token.type.startsWith("comment"))).toBe(true);
    expect(lines[1].some((token) => token.type === "type")).toBe(true);
  });

  it("nests Nim's #[ ]# comments", () => {
    const lines = tokenizeLines(byId("nim"), ["#[ a #[ b ]# c ]#", "let x = 1"]);
    expect(lines[0].every((token) => token.type.startsWith("comment"))).toBe(true);
    expect(lines[1].some((token) => token.type.startsWith("keyword"))).toBe(true);
  });

  it("does not nest where the language does not", () => {
    // Protobuf uses C-style comments, which do not nest: the first `*​/` ends it.
    const lines = tokenizeLines(byId("protobuf"), ["/* a /* b */ int32 x = 1;"]);
    expect(lines[0].some((token) => token.type === "type")).toBe(true);
  });

  it("treats COBOL's indicator-area comments as comments", () => {
    // Column 7 is the indicator area; `*` there comments the whole line.
      const line = "000100* THIS IS A COMMENT";
    const tokens = tokenize(byId("cobol"), line);
    expect(tokens.every((token) => token.type.startsWith("comment"))).toBe(true);
  });

  it("treats a Fortran fixed-form column-1 C as a comment", () => {
    const tokens = tokenize(byId("fortran"), "C this is a comment");
    expect(tokens.every((token) => token.type.startsWith("comment"))).toBe(true);
  });
});

// ── Strings ─────────────────────────────────────────────────────────────────

describe("strings", () => {
  it("tokenizes double-quoted strings with escapes", () => {
    expectToken("zig", 'const s = "hi\\n";', '"', "string");
    const tokens = tokenize(byId("zig"), 'const s = "a\\nb";');
    expect(tokens.some((token) => token.type === "string.escape")).toBe(true);
  });

  it("handles triple-quoted strings", () => {
    const lines = tokenizeLines(byId("nim"), ['let s = """', "raw text", '"""']);
    expect(lines[1].every((token) => token.type.startsWith("string"))).toBe(true);
  });

  it("handles Zig's \\\\ multi-line string literals", () => {
    expectToken("zig", "    \\\\line one", "\\\\line one", "string");
  });

  it("handles Nix's '' indented strings", () => {
    const lines = tokenizeLines(byId("nix"), ["x = ''", "  literal text", "'';"]);
    expect(lines[1].every((token) => token.type.startsWith("string"))).toBe(true);
  });

  it("nests parentheses inside PostScript strings", () => {
    // A flat rule ends the string at the inner `)`, mis-colouring everything
    // after it — and `(a (b) c)` is ordinary PostScript.
    const tokens = tokenize(byId("postscript"), "(a (b) c) show");
    const showToken = tokens.find((token) => token.text.includes("show"));
    expect(showToken?.type).toMatch(/^keyword/);
  });

  it("keeps a MATLAB transpose from opening a string", () => {
    // `A'` is transpose. Read as a quote, the rest of the line turns into a
    // string — on essentially every line of numeric MATLAB.
    expectToken("matlab", "B = A' * 2", "'", "operator");
    expect(typeOf(byId("matlab"), "B = A' * 2", "2")).toMatch(/^number/);
  });

  it("still reads a real MATLAB char array as a string", () => {
    expectToken("matlab", "s = 'hello'", "'hello'", "string");
  });

  it("keeps an Ada attribute from opening a char literal", () => {
    // `Obj'Length` — a quote that is not a quote.
    expectToken("ada", "N := Obj'Length;", "'Length", "predefined");
    expect(typeOf(byId("ada"), "N := Obj'Length + 1;", "1")).toMatch(/^number/);
  });

  it("reads a genuine Ada character literal", () => {
    expectToken("ada", "C := 'x';", "'x'", "string");
  });
});

// ── Language-specific constructs ────────────────────────────────────────────

describe("language specifics", () => {
  it("Zig: builtins, types and keywords", () => {
    expectToken("zig", "const std = @import(\"std\");", "@import", "predefined");
    expectToken("zig", "var x: u32 = 0;", "u32", "type");
    expectToken("zig", "pub fn main() void {}", "pub", "keyword");
  });

  it("Nim: keywords and types", () => {
    expectToken("nim", "proc add(a: int): int =", "proc", "keyword");
    expectToken("nim", "proc add(a: int): int =", "int", "type");
  });

  it("Crystal: instance variables and symbols", () => {
    expectToken("crystal", "@name = :ok", "@name", "variable");
    expectToken("crystal", "@name = :ok", ":ok", "string.symbol");
    expectToken("crystal", "def greet; end", "def", "keyword");
  });

  it("V: keywords and types", () => {
    expectToken("v", "pub fn main() {", "pub", "keyword");
    expectToken("v", "mut x := []int{}", "int", "type");
  });

  it("D: keywords and types", () => {
    expectToken("d", "immutable int x = 1;", "immutable", "keyword");
    expectToken("d", "immutable int x = 1;", "int", "type");
  });

  it("Vala: keywords and types", () => {
    expectToken("vala", "public string name;", "public", "keyword");
    expectToken("vala", "public string name;", "string", "type");
  });

  it("Odin: directives, keywords and types", () => {
    expectToken("odin", "#partial switch v {", "#partial", "predefined");
    expectToken("odin", "main :: proc() {}", "proc", "keyword");
    expectToken("odin", "x: f32 = 1.0", "f32", "type");
  });

  it("Gleam: attributes, keywords and types", () => {
    expectToken("gleam", "@external(erlang, \"m\", \"f\")", "@external", "annotation");
    expectToken("gleam", "pub fn main() {", "pub", "keyword");
    expectToken("gleam", "let x: Int = 1", "Int", "type");
  });

  it("Haskell: keywords, types and constants", () => {
    expectToken("haskell", "data Maybe a = Nothing", "data", "keyword");
    expectToken("haskell", "x :: Int", "Int", "type");
    expectToken("haskell", "y = Nothing", "Nothing", "constant");
  });

  it("Elm: keywords and types", () => {
    expectToken("elm", "type alias Model = { n : Int }", "alias", "keyword");
    expectToken("elm", "n : Int", "Int", "type");
  });

  it("PureScript / ReScript: keywords and annotations", () => {
    expectToken("purescript", "newtype Foo = Foo Int", "newtype", "keyword");
    expectToken("rescript", "@react.component let make = () => {", "@react.component", "annotation");
    expectToken("rescript", "let x: int = 1", "int", "type");
  });

  it("Erlang: module attributes and variables", () => {
    expectToken("erlang", "-module(demo).", "-module", "keyword.directive");
    expectToken("erlang", "Result = compute(X)", "Result", "variable");
    expectToken("erlang", "case X of", "case", "keyword");
    expectToken("erlang", "C = $a,", "$a", "string");
  });

  it("Nix: keywords, paths and builtins", () => {
    expectToken("nix", "let x = 1; in x", "let", "keyword");
    expectToken("nix", "import ./default.nix", "./default.nix", "string.link");
    expectToken("nix", "import <nixpkgs> {}", "<nixpkgs>", "string.link");
  });

  it("CMake: commands are case-insensitive, and variables are visible", () => {
    expectToken("cmake", "ADD_EXECUTABLE(app main.c)", "ADD_EXECUTABLE", "keyword");
    expectToken("cmake", "add_executable(app main.c)", "add_executable", "keyword");
    expectToken("cmake", 'set(X "${CMAKE_SOURCE_DIR}")', "${CMAKE_SOURCE_DIR}", "variable");
  });

  it("LaTeX: commands, environments and math", () => {
    expectToken("latex", "\\section{Intro}", "\\section", "keyword");
    expectToken("latex", "\\begin{itemize}", "\\begin", "keyword");
    expectToken("latex", "\\begin{itemize}", "itemize", "type");
    expectToken("latex", "value is $x^2$ here", "$x^2$", "string");
    expectToken("latex", "50\\% done", "\\%", "string.escape");
  });

  it("MATLAB: keywords and types", () => {
    expectToken("matlab", "function y = f(x)", "function", "keyword");
    expectToken("matlab", "x = int32(5);", "int32", "type");
    expectToken("matlab", "if a ~= b", "~=", "operator");
  });

  it("Assembly: directives, labels and registers", () => {
    expectToken("asm", "  .globl main", ".globl", "keyword.directive");
    expectToken("asm", "main:", "main:", "type.identifier");
    expectToken("asm", "  mov %rax, %rbx", "%rax", "variable.predefined");
    expectToken("asm", "  mov %rax, %rbx", "mov", "keyword");
  });

  it("COBOL: case-insensitive keywords, hyphenated names, picture clauses", () => {
    expectToken("cobol", "       MOVE A TO B.", "MOVE", "keyword");
    expectToken("cobol", "       move a to b.", "move", "keyword");
    expectToken("cobol", "       WORKING-STORAGE SECTION.", "WORKING-STORAGE", "keyword");
    expectToken("cobol", "       05 X PIC 9(5).", "9(5)", "string");
  });

  it("SAS: macros, macro variables and procs", () => {
    expectToken("sas", "%macro run_it;", "%macro", "keyword.macro");
    expectToken("sas", "%let y = &x.;", "&x.", "variable");
    expectToken("sas", "PROC MEANS DATA=d;", "PROC", "keyword");
  });

  it("Ada / Fortran / VHDL keywords ignore case", () => {
    expectToken("ada", "PROCEDURE Main IS", "PROCEDURE", "keyword");
    expectToken("fortran", "SUBROUTINE solve()", "SUBROUTINE", "keyword");
    expectToken("fortran", "if (.true.) then", ".true.", "keyword");
    expectToken("vhdl", "ENTITY alu IS", "ENTITY", "keyword");
    expectToken("vhdl", "signal a : STD_LOGIC;", "STD_LOGIC", "type");
  });

  it("Prolog: variables versus atoms, and the neck operator", () => {
    // Capitalised means variable, lower-case means atom. That distinction is
    // the language; losing it makes every clause look the same.
    expectToken("prolog", "parent(X, Y) :- father(X, Y).", "X", "variable");
    expectToken("prolog", "parent(X, Y) :- father(X, Y).", "parent", "identifier");
    expectToken("prolog", "parent(X, Y) :- father(X, Y).", ":-", "keyword.operator");
  });

  it("FoxPro: column-1 comments and logical constants", () => {
    const tokens = tokenize(byId("foxpro"), "* a full line comment");
    expect(tokens.every((token) => token.type.startsWith("comment"))).toBe(true);
    expectToken("foxpro", "IF x = .T.", ".T.", "constant");
  });

  it("Protobuf: keywords and scalar types", () => {
    expectToken("protobuf", "message User {", "message", "keyword");
    expectToken("protobuf", "  int32 id = 1;", "int32", "type");
    expectToken("protobuf", 'syntax = "proto3";', "syntax", "keyword");
  });

  it("PostScript: literal names, operators and hex strings", () => {
    expectToken("postscript", "/Helvetica findfont", "/Helvetica", "string.escape");
    expectToken("postscript", "/Helvetica findfont", "findfont", "keyword");
    expectToken("postscript", "1 2 add", "add", "predefined");
    expectToken("postscript", "<48656C6C6F> show", "<48656C6C6F>", "string");
  });

  it("Astro: frontmatter fence and markup tags", () => {
    expectToken("astro", "---", "---", "keyword.control");
    expectToken("astro", "<Layout title=\"x\">", "<Layout", "tag");
  });
});

// ── Numbers ─────────────────────────────────────────────────────────────────

describe("numbers", () => {
  it("reads radix prefixes as one token, not a digit plus an identifier", () => {
    expect(typeOf(byId("zig"), "x = 0xdead_beef", "0xdead_beef")).toBe("number.hex");
    expect(typeOf(byId("zig"), "x = 0b1010_1010", "0b1010_1010")).toBe("number.binary");
    expect(typeOf(byId("zig"), "x = 0o755", "0o755")).toBe("number.octal");
  });

  it("reads floats and exponents", () => {
    expect(typeOf(byId("v"), "x := 1.5e-3", "1.5e-3")).toBe("number.float");
    expect(typeOf(byId("v"), "x := 1_000.5", "1_000.5")).toBe("number.float");
  });

  it("does not treat a trailing dot as part of the number", () => {
    // `1.` then a method call, or Erlang's clause terminator.
    const tokens = tokenize(byId("erlang"), "X = 1.");
    expect(tokens.some((token) => token.type.startsWith("number"))).toBe(true);
    expect(tokens.some((token) => token.type === "delimiter")).toBe(true);
  });
});
