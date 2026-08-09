/**
 * Split model reasoning out of an agent turn.
 *
 * `<thinking>` is an internal transport convention, not something the user
 * should ever read as markup. Providers that return reasoning in a separate
 * field wrap it into the content string so it can travel as one value
 * (`providers/ollama.rs` and `providers/openai_compat.rs` both do this), and
 * the Rust side strips it with `vibe_ai::tools::strip_thinking` before the text
 * is treated as an answer. Nothing did the equivalent on the way to the screen,
 * so the raw tags rendered as literal text.
 *
 * Semantics deliberately mirror `strip_thinking`, plus the unbalanced shapes
 * VibeCoder's `extractThinking` documents as occurring in practice:
 *  - `<think>`, `<thinking>`, and namespaced spellings like minimax-m3's
 *    `<mm:think>` all count — a missed spelling puts raw tags on screen;
 *  - an *unclosed* block swallows the rest of the string — a stream cut mid
 *    reasoning must not spill half a thought into the answer;
 *  - an *orphan closing* tag means the provider consumed the opening tag into
 *    its own reasoning field, so everything before it is reasoning.
 */
export interface SplitTurn {
  /** Reasoning blocks, in order, with their tags removed. */
  reasoning: string[];
  /** What the user is actually meant to read. */
  visible: string;
}

/** Namespace prefix, e.g. the `mm:` in minimax-m3's `<mm:think>`. */
const NS = String.raw`(?:[A-Za-z][\w.-]*:)?`;
const BLOCK = new RegExp(`<${NS}think(?:ing)?>([\\s\\S]*?)</${NS}think(?:ing)?>`, "g");
const ORPHAN_CLOSE = new RegExp(`^([\\s\\S]*?)</${NS}think(?:ing)?>`);
const UNCLOSED = new RegExp(`<${NS}think(?:ing)?>([\\s\\S]*)$`);

export function splitThinking(text: string): SplitTurn {
  const reasoning: string[] = [];
  const keep = (body: string) => {
    const trimmed = body.trim();
    if (trimmed) reasoning.push(trimmed);
    return "";
  };

  // Balanced blocks first, so the unbalanced passes below only ever see the
  // genuinely malformed leftovers.
  let visible = text.replace(BLOCK, (_m, body: string) => keep(body));

  // Orphan close: the provider already ate the opening tag.
  const orphan = visible.match(ORPHAN_CLOSE);
  if (orphan) {
    keep(orphan[1]);
    visible = visible.slice(orphan[0].length);
  }

  // Never closed: the stream was cut mid-thought.
  visible = visible.replace(UNCLOSED, (_m, body: string) => keep(body));

  return { reasoning, visible: visible.trim() };
}

/** True when the turn carried reasoning but nothing else — the model thought out loud. */
export function isReasoningOnly(split: SplitTurn): boolean {
  return split.reasoning.length > 0 && split.visible.length === 0;
}

/**
 * The text to show for an assistant turn.
 *
 * When stripping leaves nothing, the reasoning *was* the reply — some models
 * put their whole answer inside a single block. Rendering an empty bubble there
 * is worse than showing the reasoning, so unwrap instead. Mirrors the daemon's
 * `unwrap_thinking`, which makes the same call server-side for chat mode.
 */
export function visibleAnswer(text: string): string {
  const { reasoning, visible } = splitThinking(text);
  return visible || reasoning.join("\n\n");
}
