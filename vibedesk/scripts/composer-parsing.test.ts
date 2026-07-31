/**
 * Unit tests for the composer's `@`-mention and `/`-command parsing.
 *
 * These are the two places where an ordinary prompt can be misread as a UI
 * gesture, and both failure modes are silent: a path in a sentence popping the
 * command menu, or an email address opening the file picker. Pure functions, so
 * they're cheap to pin.
 *
 * Run with `npm run test:composer` (bundles via esbuild, which ships with vite).
 */
// Both imports are deliberately dependency-free modules, so this runs under
// plain node with nothing but `tsc` — no bundler, no webview.
import { findSlash, matchSlash } from "../src/components/slashCommands";
import { findMention, rankFiles } from "../src/lib/composerParsing";

let failures = 0;

function eq(name: string, got: unknown, want: unknown) {
  const g = JSON.stringify(got);
  const w = JSON.stringify(want);
  if (g !== w) {
    console.error(`FAIL ${name}\n  got  ${g}\n  want ${w}`);
    failures++;
  } else {
    console.log(`ok   ${name}`);
  }
}

// ── Slash commands ────────────────────────────────────────────────────────
eq("a bare slash opens the full menu", findSlash("/"), "");
eq("a prefix filters the menu", findSlash("/rev"), "rev");
eq("/rev resolves to review", matchSlash("rev", false).map((c) => c.id), ["review"]);
// The important negative cases: a real prompt must never become a command.
eq("a path inside a sentence is not a command", findSlash("see /usr/bin and report"), null);
eq("a bare path matches no command", matchSlash(findSlash("/usr/bin")!, false).length, 0);
eq("whitespace closes the menu", findSlash("/review now"), null);
eq("ordinary text is not a command", findSlash("hello"), null);
// /stop only makes sense while something is running.
eq("stop is hidden when idle", matchSlash("sto", false).length, 0);
eq("stop is offered when running", matchSlash("sto", true).map((c) => c.id), ["stop"]);

// ── @-mentions ────────────────────────────────────────────────────────────
eq("mention at the start of the input", findMention("@src", 4), { start: 0, query: "src" });
eq("mention after a space", findMention("look at @ma", 11), { start: 8, query: "ma" });
// The important negative cases.
eq("an email address is not a mention", findMention("mail bob@x.com", 14), null);
eq("whitespace ends a mention", findMention("@src/a b", 8), null);
eq("no at-sign, no mention", findMention("plain text", 10), null);
eq("caret before the at-sign", findMention("@src", 0), null);

const files = ["src/main.rs", "src/lib.rs", "docs/main.md", "a/b/mainly.ts"];
eq("a basename prefix outranks a mid-path hit", rankFiles(files, "main")[0], "src/main.rs");
eq("non-matches are dropped", rankFiles(files, "lib"), ["src/lib.rs"]);
eq("an empty query returns the head of the list", rankFiles(files, "", 2).length, 2);

console.log(failures === 0 ? "\n✓ composer parsing: all pass" : `\n✗ ${failures} failure(s)`);
process.exit(failures === 0 ? 0 : 1);
