/** A file the user attached to the next message. */
export interface Attachment {
  path: string;
  name: string;
  bytes: number;
  text: string;
  truncated: boolean;
}

/** `12 B` / `4.2 KB` / `1.1 MB` — size label for an attachment chip. */
export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * Pick a fence long enough that the file's own backticks can't terminate it.
 *
 * Attaching a markdown file — the obvious thing to do — embeds ``` sequences.
 * With a fixed three-backtick fence the block would close early and the rest of
 * the file would be read as prose, silently corrupting the prompt.
 */
function fenceFor(text: string): string {
  const longest = [...text.matchAll(/`+/g)].reduce((n, m) => Math.max(n, m[0].length), 0);
  return "`".repeat(Math.max(3, longest + 1));
}

/**
 * Compose the final prompt: attachments as labelled fenced blocks, then the
 * user's message.
 *
 * Attachments lead so the model reads the material before the instruction that
 * refers to it, and each block is labelled with its path so "the second file"
 * and "in config.toml" both resolve. Truncation is stated inline — a silently
 * clipped file would have the model reasoning about a tail that isn't there.
 */
export function composeWithAttachments(text: string, attachments: Attachment[]): string {
  if (attachments.length === 0) return text;
  const blocks = attachments.map((a) => {
    const fence = fenceFor(a.text);
    const note = a.truncated ? ` (truncated — first ${formatBytes(a.text.length)} of ${formatBytes(a.bytes)})` : "";
    return `Attached file: ${a.path}${note}\n${fence}\n${a.text}\n${fence}`;
  });
  return `${blocks.join("\n\n")}\n\n${text}`;
}
