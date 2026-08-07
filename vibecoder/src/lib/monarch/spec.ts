/**
 * Monarch grammar factory.
 *
 * Monaco ships grammars for ~80 languages, but not for Zig, Nim, Crystal, V, D,
 * Odin, Gleam, Nix, Elm, PureScript, ReScript, Haskell, CMake, LaTeX, MATLAB,
 * Assembly, COBOL, SAS, Ada, Fortran, Prolog, VHDL or FoxPro — every one of
 * which VibeCody routes to a language server. Registering a bare language *id*
 * (which is what the LSP providers need) leaves those files rendering as flat
 * grey text.
 *
 * Twenty-two hand-written grammars would be twenty-two chances to get string
 * escaping or comment nesting subtly wrong. Instead each language is a small
 * declarative {@link LanguageSpec} — keywords, comment markers, which string
 * forms exist — and {@link monarchFromSpec} assembles the tokenizer. The shared
 * machinery is written once and tested once, per-language specs stay readable,
 * and {@link languageConfigurationFromSpec} gets comment-toggle (⌘/) and
 * bracket matching for free from the same declaration.
 *
 * `extra` is the escape hatch for the genuinely odd: Nix's `''…''` strings,
 * Zig's `\\` multi-line literals, LaTeX's `\command`, COBOL's column-7
 * comments, MATLAB's transpose-vs-quote ambiguity.
 */

// Structural stand-ins for `monaco.languages.*`, so this module needs no
// runtime monaco import and stays testable against the monarch compiler alone.
export type MonarchAction =
  | string
  | { token: string; next?: string; nextEmbedded?: string; log?: string }
  | { cases: Record<string, string | { token: string; next?: string }> }
  /** One token per capture group, for rules that match several parts at once. */
  | readonly (string | { token: string; next?: string })[];

export type MonarchRule =
  | [RegExp, MonarchAction]
  | [RegExp, MonarchAction, string]
  | { include: string };

export interface MonarchLanguage {
  ignoreCase?: boolean;
  defaultToken?: string;
  keywords?: readonly string[];
  types?: readonly string[];
  constants?: readonly string[];
  builtins?: readonly string[];
  tokenizer: Record<string, MonarchRule[]>;
}

export interface CommentSpec {
  /** Line-comment markers, e.g. `["//"]`, `["#"]`, `["--"]`, `["%"]`. */
  line?: readonly string[];
  /** Block comment delimiters, given as an open/close pair. */
  block?: readonly [string, string];
  /**
   * Whether block comments nest. True for D `/+ +/`, Nim `#[ ]#`,
   * Haskell/Elm `{- -}`, and Odin's C-style pair. Getting this wrong means one nested
   * comment un-comments the rest of the file.
   */
  nested?: boolean;
  /** A second block form, e.g. Nim's `#[ ]#` alongside nothing else. */
  block2?: readonly [string, string];
}

export interface StringSpec {
  /** `"…"` with backslash escapes. Default true. */
  double?: boolean;
  /** `'…'` as a *string* (not a char literal). */
  single?: boolean;
  /** `` `…` `` raw/template strings. */
  backtick?: boolean;
  /** `"""…"""` (Nim, Elm, Vala, Python-ish). */
  tripleDouble?: boolean;
  /** `'c'` single-character literals — mutually exclusive with `single`. */
  chars?: boolean;
}

export interface LanguageSpec {
  id: string;
  /** With the leading dot, as Monaco expects. */
  extensions: readonly string[];
  aliases?: readonly string[];
  filenames?: readonly string[];
  keywords: readonly string[];
  /** Built-in type names → `type` token. */
  types?: readonly string[];
  /** `true` / `false` / `nil` / `null` → `constant` token. */
  constants?: readonly string[];
  /** Built-in functions or intrinsics → `predefined` token. */
  builtins?: readonly string[];
  /** Keyword matching ignores case: COBOL, Ada, Fortran, SAS, VHDL, FoxPro. */
  ignoreCase?: boolean;
  comments: CommentSpec;
  strings?: StringSpec;
  /** Identifier pattern. Defaults to a leading letter or underscore. */
  identifier?: RegExp;
  /**
   * Rules spliced in *before* everything generated. Order matters: these win,
   * which is what makes the awkward cases expressible.
   */
  extra?: readonly MonarchRule[];
  /** Rules appended after the generated ones, as a final fallback. */
  trailing?: readonly MonarchRule[];
}

/** Escape a literal for use inside a regular expression. */
export function escapeRegExp(literal: string): string {
  return literal.replace(/[.*+?^${}()|[\]\\/-]/g, "\\$&");
}

/** Escape a literal for use inside a `[...]` character class. */
function escapeCharClass(chars: readonly string[]): string {
  // `]`, `\`, `^` and `-` are the only characters with meaning inside a class.
  return chars.map((c) => c.replace(/[\]\\^-]/g, "\\$&")).join("");
}

const uniqueFirstChars = (...literals: string[]): string[] =>
  Array.from(new Set(literals.map((literal) => literal[0]).filter(Boolean)));

/**
 * A rule consuming a run of characters that cannot begin either delimiter.
 *
 * Without it, a commented-out block is tokenized one character at a time.
 */
function bulkRule(open: string, close: string, token: string): MonarchRule {
  const excluded = escapeCharClass(uniqueFirstChars(open, close));
  return [new RegExp(`[^${excluded}]+`), token];
}

/** Tokenizer states for one block-comment form. */
function blockCommentStates(
  open: string,
  close: string,
  stateName: string,
  nested: boolean,
): Record<string, MonarchRule[]> {
  const rules: MonarchRule[] = [bulkRule(open, close, "comment")];
  if (nested) {
    // `@push` re-enters this same state, so depth is tracked by the state stack.
    rules.push([new RegExp(escapeRegExp(open)), "comment", "@push"]);
  }
  rules.push([new RegExp(escapeRegExp(close)), "comment", "@pop"]);
  // Anything left is a delimiter character that did not start a delimiter.
  rules.push([/./, "comment"]);
  return { [stateName]: rules };
}

const DEFAULT_IDENTIFIER = /[a-zA-Z_]\w*/;

/**
 * Numbers, ordered most-specific first: hex/binary/octal before float before
 * integer, so `0x1f` is not read as `0` followed by `x1f`. Digit separators
 * (`_`) are accepted everywhere — Zig, Nim, Rust, D, Odin and Ada all use them.
 */
const NUMBER_RULES: MonarchRule[] = [
  [/0[xX][0-9a-fA-F][0-9a-fA-F_]*/, "number.hex"],
  [/0[bB][01][01_]*/, "number.binary"],
  [/0[oO][0-7][0-7_]*/, "number.octal"],
  [/\d[\d_]*\.\d[\d_]*([eE][-+]?\d+)?/, "number.float"],
  [/\d[\d_]*[eE][-+]?\d+/, "number.float"],
  [/\d[\d_]*/, "number"],
];

const BRACKET_RULE: MonarchRule = [/[{}()[\]]/, "@brackets"];
const OPERATOR_RULE: MonarchRule = [/[=!<>~?:&|+\-*/^%@#$]+/, "operator"];
const DELIMITER_RULE: MonarchRule = [/[;,.]/, "delimiter"];

/** Build a Monarch grammar from a spec. */
export function monarchFromSpec(spec: LanguageSpec): MonarchLanguage {
  const strings: StringSpec = spec.strings ?? { double: true };
  const identifier = spec.identifier ?? DEFAULT_IDENTIFIER;

  // ── whitespace + comments ────────────────────────────────────────────────
  const whitespace: MonarchRule[] = [[/[ \t\r\n]+/, "white"]];
  for (const marker of spec.comments.line ?? []) {
    whitespace.push([new RegExp(`${escapeRegExp(marker)}.*$`), "comment"]);
  }
  const states: Record<string, MonarchRule[]> = {};
  if (spec.comments.block) {
    const [open, close] = spec.comments.block;
    whitespace.push([
      new RegExp(escapeRegExp(open)),
      { token: "comment", next: "@blockComment" },
    ]);
    Object.assign(
      states,
      blockCommentStates(open, close, "blockComment", spec.comments.nested === true),
    );
  }
  if (spec.comments.block2) {
    const [open, close] = spec.comments.block2;
    whitespace.push([
      new RegExp(escapeRegExp(open)),
      { token: "comment", next: "@blockComment2" },
    ]);
    Object.assign(
      states,
      blockCommentStates(open, close, "blockComment2", spec.comments.nested === true),
    );
  }

  // ── strings ──────────────────────────────────────────────────────────────
  const stringRules: MonarchRule[] = [];
  // Triple-quoted first: `"""` must not be read as an empty `""` plus a quote.
  if (strings.tripleDouble) {
    stringRules.push([/"""/, { token: "string.quote", next: "@tripleString" }]);
    states.tripleString = [
      [/[^"]+/, "string"],
      [/"""/, { token: "string.quote", next: "@pop" }],
      [/"/, "string"],
    ];
  }
  if (strings.double !== false) {
    stringRules.push([/"/, { token: "string.quote", next: "@doubleString" }]);
    states.doubleString = [
      [/[^\\"]+/, "string"],
      [/\\./, "string.escape"],
      [/"/, { token: "string.quote", next: "@pop" }],
    ];
  }
  if (strings.single) {
    stringRules.push([/'/, { token: "string.quote", next: "@singleString" }]);
    states.singleString = [
      [/[^\\']+/, "string"],
      [/\\./, "string.escape"],
      [/'/, { token: "string.quote", next: "@pop" }],
    ];
  }
  if (strings.backtick) {
    stringRules.push([/`/, { token: "string.quote", next: "@backtickString" }]);
    states.backtickString = [
      [/[^\\`]+/, "string"],
      [/\\./, "string.escape"],
      [/`/, { token: "string.quote", next: "@pop" }],
    ];
  }
  if (strings.chars) {
    // A complete char literal only — an unterminated `'` stays punctuation, so
    // `x'Length` (Ada) and `a'` (MATLAB transpose) do not swallow the line.
    stringRules.push([/'(?:[^\\']|\\.)'/, "string"]);
  }

  // ── identifiers ──────────────────────────────────────────────────────────
  const cases: Record<string, string> = {};
  if (spec.keywords.length > 0) cases["@keywords"] = "keyword";
  if (spec.types?.length) cases["@types"] = "type";
  if (spec.constants?.length) cases["@constants"] = "constant";
  if (spec.builtins?.length) cases["@builtins"] = "predefined";
  cases["@default"] = "identifier";

  const root: MonarchRule[] = [
    ...(spec.extra ?? []),
    { include: "@whitespace" },
    [identifier, { cases }],
    ...stringRules,
    ...NUMBER_RULES,
    BRACKET_RULE,
    DELIMITER_RULE,
    OPERATOR_RULE,
    ...(spec.trailing ?? []),
  ];

  return {
    ...(spec.ignoreCase ? { ignoreCase: true } : {}),
    defaultToken: "",
    keywords: spec.keywords,
    ...(spec.types?.length ? { types: spec.types } : {}),
    ...(spec.constants?.length ? { constants: spec.constants } : {}),
    ...(spec.builtins?.length ? { builtins: spec.builtins } : {}),
    tokenizer: { root, whitespace, ...states },
  };
}

// ── Editor behaviour ───────────────────────────────────────────────────────

export interface LanguageConfiguration {
  comments?: {
    lineComment?: string;
    blockComment?: [string, string];
  };
  brackets?: [string, string][];
  autoClosingPairs?: { open: string; close: string; notIn?: string[] }[];
  surroundingPairs?: { open: string; close: string }[];
}

/**
 * Comment-toggle, bracket matching and auto-closing, from the same spec.
 *
 * Without this ⌘/ does nothing in these files — a more noticeable gap than
 * colour, and free once the spec exists.
 */
export function languageConfigurationFromSpec(
  spec: LanguageSpec,
): LanguageConfiguration {
  const strings: StringSpec = spec.strings ?? { double: true };
  const pairs: { open: string; close: string }[] = [
    { open: "{", close: "}" },
    { open: "[", close: "]" },
    { open: "(", close: ")" },
  ];
  if (strings.double !== false) pairs.push({ open: '"', close: '"' });
  if (strings.single) pairs.push({ open: "'", close: "'" });
  if (strings.backtick) pairs.push({ open: "`", close: "`" });

  return {
    comments: {
      ...(spec.comments.line?.[0] ? { lineComment: spec.comments.line[0] } : {}),
      ...(spec.comments.block
        ? { blockComment: [spec.comments.block[0], spec.comments.block[1]] as [string, string] }
        : {}),
    },
    brackets: [
      ["{", "}"],
      ["[", "]"],
      ["(", ")"],
    ],
    // Never auto-close a quote inside a comment or string — that is where it
    // is most often a typo (an apostrophe in prose).
    autoClosingPairs: pairs.map((pair) =>
      pair.open === pair.close
        ? { ...pair, notIn: ["string", "comment"] }
        : pair,
    ),
    surroundingPairs: pairs,
  };
}
