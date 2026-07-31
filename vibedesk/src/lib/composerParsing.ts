/**
 * Pure parsing for the composer's `@`-mention tokens.
 *
 * Kept free of React and Tauri imports so it can be exercised directly by
 * `scripts/composer-parsing.test.ts` under plain node — the misreads that matter
 * here (a path becoming a command, an email becoming a mention) are silent, so
 * they need to be pinned, and a test shouldn't have to boot a webview to do it.
 */

/** An active `@…` token in the composer: where it starts and what's typed. */
export interface MentionQuery {
  /** Index of the `@` in the text. */
  start: number;
  /** Text between the `@` and the caret. */
  query: string;
}

/**
 * Find the `@`-mention token the caret currently sits in, if any.
 *
 * A mention is only live when the `@` starts a word (beginning of input or
 * after whitespace) and nothing between it and the caret is whitespace — so an
 * email address or a mid-word `@` never opens the picker.
 */
export function findMention(text: string, caret: number): MentionQuery | null {
  const upto = text.slice(0, caret);
  const at = upto.lastIndexOf("@");
  if (at === -1) return null;
  const before = at === 0 ? " " : upto[at - 1];
  if (!/\s/.test(before)) return null;
  const query = upto.slice(at + 1);
  if (/\s/.test(query)) return null;
  return { start: at, query };
}

/** Rank paths for a mention query: basename matches beat mid-path matches. */
export function rankFiles(files: string[], query: string, limit = 8): string[] {
  const needle = query.toLowerCase();
  if (!needle) return files.slice(0, limit);
  return files
    .map((f) => {
      const lower = f.toLowerCase();
      const base = lower.split("/").pop() ?? lower;
      if (base.startsWith(needle)) return { f, s: 4 };
      if (base.includes(needle)) return { f, s: 3 };
      if (lower.includes(needle)) return { f, s: 2 };
      return { f, s: 0 };
    })
    .filter((r) => r.s > 0)
    .sort((a, b) => b.s - a.s || a.f.length - b.f.length)
    .slice(0, limit)
    .map((r) => r.f);
}
