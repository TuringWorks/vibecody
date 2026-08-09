/**
 * Reasoning-splitting tests for VibeAIChat.
 *
 * These mirror `vibedesk/scripts/thinking.test.ts`. The rules exist in three
 * places (Rust `strip_thinking`, VibeDesk, VibeAIChat) because the two Tauri apps
 * share no package — so each copy needs its own test, or a fix to one silently
 * leaves the others rendering raw markup.
 *
 * The case that motivated this: VibeAIChat printed a `<thinking>` block verbatim,
 * including the model reasoning about whether it was allowed to answer.
 *
 * Run with `npm run test:thinking`.
 */
import { splitThinking, visibleAnswer } from "../../packages/vibe-ui-shared/src/lib/thinking";

let failures = 0;

function eq(name: string, got: unknown, want: unknown) {
  const g = JSON.stringify(got);
  const w = JSON.stringify(want);
  if (g !== w) {
    console.error(`FAIL ${name}\n  got  ${g}\n  want ${w}`);
    failures++;
  } else {
    console.log(`  ok  ${name}`);
  }
}

eq("reasoning is separated from the answer", splitThinking("<thinking>plan</thinking>Hello!"), {
  reasoning: ["plan"],
  visible: "Hello!",
});

eq("plain prose is untouched", splitThinking("Hi there!"), {
  reasoning: [],
  visible: "Hi there!",
});

// minimax-m3's namespaced spelling — the one that leaked in VibeCoder.
eq("namespaced blocks are stripped", splitThinking("<mm:think>plan</mm:think>Hi"), {
  reasoning: ["plan"],
  visible: "Hi",
});

eq(
  "an orphan closing tag makes the prefix reasoning",
  splitThinking("still thinking.</mm:think>Hi"),
  { reasoning: ["still thinking."], visible: "Hi" },
);

eq("an unclosed opener swallows its tail", splitThinking("Hi<thinking>cut off"), {
  reasoning: ["cut off"],
  visible: "Hi",
});

eq("prose containing an unrelated closing tag survives", splitThinking("Use </div> here."), {
  reasoning: [],
  visible: "Use </div> here.",
});

// The screenshot case: the whole turn was reasoning, so the bubble was raw
// markup. Showing nothing would be worse — unwrap instead.
eq(
  "a reasoning-only turn falls back to its text, not an empty bubble",
  visibleAnswer("<thinking>The user just said hi. I should greet them.</thinking>"),
  "The user just said hi. I should greet them.",
);

eq("an ordinary answer passes through visibleAnswer", visibleAnswer("<thinking>x</thinking>Hi!"), "Hi!");

if (failures > 0) {
  // `throw` rather than `process.exit` — VibeAIChat has no @types/node, and an
  // uncaught throw exits non-zero all the same, so the script still fails CI.
  throw new Error(`${failures} test(s) failed`);
}
console.log("\nall thinking tests passed");
