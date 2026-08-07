/**
 * Test harness that runs Monaco's real Monarch engine.
 *
 * Grammar tests are worthless if they only assert the shape of the definition
 * object — that verifies the spec was typed correctly, not that the tokenizer
 * behaves. This compiles a grammar with Monaco's own `monarchCompile` and runs
 * its `MonarchTokenizer`, so a rule ordering mistake or a bad regex shows up as
 * a wrong token, exactly as it would in the editor.
 *
 * The tokenizer needs four platform services. Monaco only touches a narrow part
 * of each during plain tokenization (no embedded languages, no theming), so
 * minimal stubs are enough; if a future Monaco reaches further, these tests fail
 * loudly rather than silently degrading.
 */

import { compile } from "monaco-editor/esm/vs/editor/standalone/common/monarch/monarchCompile.js";
import { MonarchTokenizer } from "monaco-editor/esm/vs/editor/standalone/common/monarch/monarchLexer.js";
import { grammarFor } from "../index";
import type { LanguageSpec } from "../spec";

export interface Token {
  /** 0-based offset into the line. */
  offset: number;
  /** Monarch token type with the `.language` suffix stripped. */
  type: string;
  /** The source text this token covers. */
  text: string;
}

const languageService = {
  languageIdCodec: {
    encodeLanguageId: () => 1,
    decodeLanguageId: () => "test",
  },
};
const themeService = {
  getColorTheme: () => ({ tokenTheme: { match: () => 0, getColorMap: () => [] } }),
  onDidColorThemeChange: () => ({ dispose() {} }),
};
const configurationService = {
  getValue: () => ({}),
  onDidChangeConfiguration: () => ({ dispose() {} }),
};

type TokenizerLike = {
  getInitialState(): unknown;
  tokenize(
    line: string,
    hasEOL: boolean,
    state: unknown,
  ): { tokens: { offset: number; type: string }[]; endState: unknown };
};

function tokenizerFor(spec: LanguageSpec): TokenizerLike {
  const lexer = compile(spec.id, grammarFor(spec) as never);
  return new (MonarchTokenizer as never as new (
    a: unknown,
    b: unknown,
    c: string,
    d: unknown,
    e: unknown,
  ) => TokenizerLike)(
    languageService,
    themeService,
    spec.id,
    lexer,
    configurationService,
  );
}

/** Tokenize one line. */
export function tokenize(spec: LanguageSpec, line: string): Token[] {
  return tokenizeLines(spec, [line])[0];
}

/**
 * Tokenize several lines, threading the end state through — the only way to
 * test multi-line constructs like block comments and heredocs.
 */
export function tokenizeLines(spec: LanguageSpec, lines: string[]): Token[][] {
  const tokenizer = tokenizerFor(spec);
  let state = tokenizer.getInitialState();
  return lines.map((line) => {
    const result = tokenizer.tokenize(line, true, state);
    state = result.endState;
    return result.tokens.map((token, index) => {
      const end = result.tokens[index + 1]?.offset ?? line.length;
      return {
        offset: token.offset,
        // Monarch appends `.<languageId>` to every token type.
        type: token.type.replace(new RegExp(`\\.${spec.id}$`), ""),
        text: line.slice(token.offset, end),
      };
    });
  });
}

/** The token type covering `text`'s first occurrence in `line`. */
export function typeOf(spec: LanguageSpec, line: string, text: string): string {
  const offset = line.indexOf(text);
  if (offset < 0) throw new Error(`${JSON.stringify(text)} is not in the line`);
  const tokens = tokenize(spec, line);
  const hit = [...tokens].reverse().find((token) => token.offset <= offset);
  if (!hit) throw new Error(`no token covers offset ${offset}`);
  return hit.type;
}

/** Every distinct token type produced for a line, in order of first appearance. */
export function typesIn(spec: LanguageSpec, line: string): string[] {
  return Array.from(new Set(tokenize(spec, line).map((token) => token.type)));
}
