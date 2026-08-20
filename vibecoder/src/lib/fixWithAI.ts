/**
 * The one place a panel turns findings into a change request for chat.
 *
 * Every panel that reports something wrong needs the same hand-off: write the
 * request into the chat composer over the `vibecoder:inject-context` bridge and
 * let the user read it and press send. Nothing here edits a file — that has
 * been the rule since the security watcher landed, and it is the reason this is
 * a request rather than an action.
 *
 * It exists as a module because the wording is load-bearing and was already
 * drifting between the two panels that had it. A request that omits the path
 * gets a second copy of the file back instead of a fix; one that omits how
 * sure the finding is gets a "fix" applied to a false positive. Both lessons
 * are encoded once, here.
 */

/** One thing a panel wants fixed. */
export interface FixItem {
  /** Workspace-relative path. Absent when the finding names no file. */
  file?: string | null;
  line?: number | null;
  /** The severity word the producer used, not a bucket it was mapped into. */
  severity?: string | null;
  /** Short label — a rule name, a CWE, a check. */
  title?: string | null;
  /** What is wrong. The one field every producer has. */
  message: string;
  /** What to do about it, when the producer offered something. */
  suggestion?: string | null;
  /**
   * Extra lines under the item, verbatim. For what the reader needs in order
   * to judge the finding — evidence, a verification verdict, a rule link.
   */
  notes?: string[];
}

export interface FixRequestOptions {
  /**
   * What produced these, named in the opening line ("security scanner", "code
   * review"). The model treats a linter hit and a reviewed defect differently,
   * and it can only do that if it is told which it has.
   */
  source: string;
  /**
   * The size of the set this batch came from, when the caller capped it.
   * Omitted means the batch is everything.
   */
  total?: number;
  /** Producer-specific lines appended after the shared instructions. */
  instructions?: string[];
}

/**
 * How many items one hand-off carries.
 *
 * A scan of a real workspace returns thousands; pasting all of them into the
 * composer produces a request no model can act on. Callers slice to this and
 * pass the true `total`, so a capped hand-off can never read as the whole set.
 */
export const FIX_BATCH_LIMIT = 25;

/** Where an item points, as a human reads it. */
export function locationOf(item: FixItem): string {
  if (!item.file) return "the reported location";
  return item.line != null && item.line > 0 ? `${item.file}:${item.line}` : item.file;
}

/** The bullet for one item: severity, line, title, message, then its notes. */
function itemLine(item: FixItem): string {
  const severity = item.severity ? `[${item.severity}] ` : "";
  const line = item.line != null && item.line > 0 ? `line ${item.line} — ` : "";
  const title = item.title ? `${item.title}: ` : "";
  const notes = (item.notes ?? []).filter(Boolean).map((n) => `\n  ${n}`);
  const suggestion = item.suggestion ? `\n  Suggested fix: ${item.suggestion}` : "";
  return `- ${severity}${line}${title}${item.message}${notes.join("")}${suggestion}`;
}

/** Group items by file, keeping the order each file first appeared in. */
function byFile(items: FixItem[]): [string, FixItem[]][] {
  return items.reduce<[string, FixItem[]][]>((acc, item) => {
    const key = item.file || "(no file given)";
    const found = acc.find(([f]) => f === key);
    if (found) {
      found[1].push(item);
      return acc;
    }
    return [...acc, [key, [item]] as [string, FixItem[]]];
  }, []);
}

/**
 * The change request handed to chat.
 *
 * It names the existing path explicitly and says what the items are, because
 * the two ways this goes wrong are a model writing a new file instead of
 * editing the named one, and a model "fixing" a finding nobody verified.
 */
export function buildFixRequest(items: FixItem[], options: FixRequestOptions): string {
  const { source, total, instructions = [] } = options;
  const blocks = byFile(items).map(([file, group]) =>
    [`${file}:`, ...group.map(itemLine)].join("\n"),
  );

  const opening =
    total != null && total > items.length
      ? `Fix these ${items.length} ${source} findings. They are the first ${items.length} of ${total}; the remaining ${total - items.length} are not listed here.`
      : items.length === 1
        ? `Fix this ${source} finding in ${locationOf(items[0])}.`
        : `Fix these ${items.length} ${source} findings.`;

  return [
    opening,
    "",
    ...blocks,
    "",
    "Edit each file in place at the path given above — do not create a new file.",
    ...instructions,
    "Keep each change minimal and leave unrelated behaviour alone.",
  ].join("\n");
}

/**
 * Hand a change request to the active chat tab's composer. The user sends it.
 *
 * A no-op for an empty batch: dispatching an empty request would clear whatever
 * the user was already typing for nothing.
 */
export function sendFixToChat(items: FixItem[], options: FixRequestOptions): boolean {
  if (items.length === 0) return false;
  window.dispatchEvent(
    new CustomEvent("vibecoder:inject-context", {
      detail: buildFixRequest(items, options),
    }),
  );
  return true;
}

/** Button label, naming the cap and the true count whenever the cap bites. */
export function fixLabel(count: number): string {
  if (count > FIX_BATCH_LIMIT) return `Fix first ${FIX_BATCH_LIMIT} of ${count} with AI`;
  return count === 1 ? "Fix with AI" : `Fix all ${count} with AI`;
}
