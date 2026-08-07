/**
 * Registers the grammars Monaco does not ship, at editor mount.
 *
 * Registration does three things, and all three matter:
 *   * makes the language *id* valid, so `setModelLanguage` accepts it and the
 *     LSP providers keyed on that id are ever consulted;
 *   * supplies the tokenizer, so the file is not flat grey text;
 *   * supplies the language configuration, so ⌘/ comments the right way and
 *     brackets match.
 *
 * Before this existed, only the first happened — files in 22 languages had
 * working IntelliSense and no syntax colour at all.
 */

import type * as Monaco from "monaco-editor";
import {
  languageConfigurationFromSpec,
  monarchFromSpec,
  type LanguageSpec,
} from "./spec";
import { EXTRA_STATES, MONARCH_LANGUAGES } from "./languages";

/** Build the full Monarch grammar for one spec, including its extra states. */
export function grammarFor(spec: LanguageSpec) {
  const grammar = monarchFromSpec(spec);
  const extra = EXTRA_STATES[spec.id];
  if (!extra) return grammar;
  return {
    ...grammar,
    tokenizer: { ...grammar.tokenizer, ...extra },
  };
}

/**
 * Register every grammar that Monaco does not already provide.
 *
 * Idempotent, and never overrides a built-in: if Monaco gains a grammar for one
 * of these in a future version, theirs wins and ours is skipped. Safe to call on
 * every editor mount.
 */
export function registerMonarchLanguages(monaco: typeof Monaco): string[] {
  const existing = new Set(
    monaco.languages.getLanguages().map((language) => language.id),
  );
  const registered: string[] = [];

  for (const spec of MONARCH_LANGUAGES) {
    if (existing.has(spec.id)) continue;

    monaco.languages.register({
      id: spec.id,
      extensions: [...spec.extensions],
      ...(spec.aliases ? { aliases: [...spec.aliases] } : {}),
      ...(spec.filenames ? { filenames: [...spec.filenames] } : {}),
    });
    // `onLanguage` defers the (small) grammar build until a file of that
    // language is actually opened, so mount cost stays flat as languages grow.
    monaco.languages.onLanguage(spec.id, () => {
      monaco.languages.setMonarchTokensProvider(
        spec.id,
        grammarFor(spec) as Monaco.languages.IMonarchLanguage,
      );
      monaco.languages.setLanguageConfiguration(
        spec.id,
        languageConfigurationFromSpec(spec) as Monaco.languages.LanguageConfiguration,
      );
    });
    registered.push(spec.id);
  }

  return registered;
}

export { MONARCH_LANGUAGES } from "./languages";
export type { LanguageSpec } from "./spec";
