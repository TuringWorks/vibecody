/**
 * Unit tests for reasoning-block splitting.
 *
 * `<thinking>` is transport markup that leaked to the screen as literal text.
 * These pin the split, and in particular the two cases that made the leak
 * user-visible: a turn that is *only* reasoning, and a block the stream cut
 * off before its closing tag.
 *
 * Semantics must track `vibe_ai::tools::strip_thinking` — if that changes and
 * this does not, the UI starts disagreeing with what the agent treated as an
 * answer.
 *
 * Run with `npm run test:thinking`.
 */
import { splitThinking, isReasoningOnly } from "../src/lib/thinking";

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

// The exact shape that put raw tags on screen.
eq(
  "reasoning-only turn yields no visible prose",
  splitThinking(
    "<thinking>The user wants a small program that displays the Mandelbrot set in color. Let me first check the workspace.</thinking>",
  ),
  {
    reasoning: [
      "The user wants a small program that displays the Mandelbrot set in color. Let me first check the workspace.",
    ],
    visible: "",
  },
);

eq(
  "reasoning is separated from the answer",
  splitThinking("<thinking>plan it</thinking>\nHere is the answer."),
  { reasoning: ["plan it"], visible: "Here is the answer." },
);

eq("plain prose is untouched", splitThinking("Just an answer."), {
  reasoning: [],
  visible: "Just an answer.",
});

// `<think>` is the other spelling strip_thinking accepts.
eq("the <think> spelling also counts", splitThinking("<think>hm</think>done"), {
  reasoning: ["hm"],
  visible: "done",
});

// A cut stream must not spill half a thought into the answer.
eq(
  "an unclosed block swallows its tail",
  splitThinking("Answer so far.<thinking>I should also che"),
  { reasoning: ["I should also che"], visible: "Answer so far." },
);

// The provider consumed the opening tag into its own reasoning field, leaving
// only the close — VibeCoder's extractThinking documents this as real.
eq(
  "an orphan closing tag makes the prefix reasoning",
  splitThinking("I should check the workspace.</thinking>\nHere is the answer."),
  { reasoning: ["I should check the workspace."], visible: "Here is the answer." },
);

// Mismatched spellings must not pair up into a bogus block.
eq(
  "a <think> is not closed by a </thinking>",
  splitThinking("<think>a</thinking>b").reasoning.length > 0,
  true,
);

// Verbatim from a minimax-m3 turn that rendered `</mm:think>` on screen: the
// provider ate the opening tag, leaving a namespaced orphan close.
eq(
  "a namespaced orphan close is reasoning, not prose",
  splitThinking("Let me write a single Python file.</mm:think>def fib(n): pass"),
  { reasoning: ["Let me write a single Python file."], visible: "def fib(n): pass" },
);

eq("namespaced blocks are stripped", splitThinking("<mm:think>plan</mm:think>answer"), {
  reasoning: ["plan"],
  visible: "answer",
});

eq("a namespaced unclosed opener swallows its tail", splitThinking("answer<mm:think>cut"), {
  reasoning: ["cut"],
  visible: "answer",
});

// A closing tag in ordinary prose must not eat the sentence before it.
eq(
  "prose containing an unrelated closing tag is untouched",
  splitThinking("Use </div> in the template."),
  { reasoning: [], visible: "Use </div> in the template." },
);

eq(
  "multiple blocks are kept in order",
  splitThinking("<thinking>one</thinking>mid<thinking>two</thinking>end"),
  { reasoning: ["one", "two"], visible: "midend" },
);

eq("empty reasoning is dropped", splitThinking("<thinking>   </thinking>text"), {
  reasoning: [],
  visible: "text",
});

eq(
  "isReasoningOnly detects the narrated-intent turn",
  isReasoningOnly(splitThinking("<thinking>let me look</thinking>")),
  true,
);
eq(
  "isReasoningOnly is false when an answer exists",
  isReasoningOnly(splitThinking("<thinking>x</thinking>answer")),
  false,
);

if (failures > 0) {
  console.error(`\n${failures} test(s) failed`);
  process.exit(1);
}
console.log("\nall thinking tests passed");
