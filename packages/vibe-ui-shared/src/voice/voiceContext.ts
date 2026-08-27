/**
 * What the voice assistant is told about the workspace, and how much of it.
 *
 * The spoken path gets one round trip — no tool calls, no follow-up read — so
 * whatever this block does not say, the assistant cannot find out. That is why
 * a bare path listing produced "just a collection of directories and files, I
 * couldn't tell what Gbrain is": the answer was in a README the model had no
 * way to open.
 *
 * Every block is labelled and every block is bounded. The caps are deliberately
 * small: this is prepended to a system prompt on a latency-sensitive path, and
 * a 200 KB source file pasted into it would push the actual question out of the
 * window while costing a second of time-to-first-token.
 */

/** Caps, in characters unless the name says otherwise. */
export const VOICE_CONTEXT_LIMITS = {
  /** Enough for a README's title, tagline and first section. */
  readme: 1500,
  /** The file on screen: enough to talk about, not enough to recite. */
  openFile: 2000,
  /** Paths, not characters. The daemon bounds the whole block again. */
  treeEntries: 400,
  /** Host-supplied extras (editor selection, pinned notes). */
  extra: 2000,
  /** Belt and braces — the daemon's own clamp is the real guard. */
  total: 24_000,
} as const;

export interface VoiceContextParts {
  /** Absolute path of the open workspace, if there is one. */
  root?: string | null;
  /** Head of the project's README, already read by the host. */
  readme?: string | null;
  /** Path of the file on screen. */
  openFile?: string | null;
  /** Its contents, if the host has them. A path alone says nothing. */
  openFileText?: string | null;
  /** Tracked files, project-relative. */
  tree?: readonly string[];
  /** Pinned memory / rules the host already formats for the typed path. */
  pinned?: string | null;
  /** Anything else the host wants said — editor selection, current diff. */
  extra?: string | null;
}

/** Trim to `max` characters, marking the cut so the model knows it is partial. */
function clip(text: string, max: number): string {
  const t = text.trim();
  return t.length <= max ? t : `${t.slice(0, max).trimEnd()}\n… (truncated)`;
}

/** The last path segment — a project's name as a human would say it. */
function baseName(path: string): string {
  const parts = path.replace(/[/\\]+$/, "").split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

/**
 * Compose the `<workspace>` body sent over `set_context`.
 *
 * Returns "" when there is nothing to say — an empty block *clears* the
 * daemon's stored context rather than pinning something stale, so returning a
 * header with no content underneath would be worse than returning nothing.
 */
export function buildVoiceContext(parts: VoiceContextParts): string {
  const blocks: string[] = [];

  if (parts.root) {
    blocks.push(`Project: ${baseName(parts.root)}\nPath: ${parts.root}`);
  }
  if (parts.pinned?.trim()) {
    blocks.push(clip(parts.pinned, VOICE_CONTEXT_LIMITS.extra));
  }
  if (parts.readme?.trim()) {
    blocks.push(`README:\n${clip(parts.readme, VOICE_CONTEXT_LIMITS.readme)}`);
  }
  if (parts.openFile) {
    const body = parts.openFileText?.trim()
      ? `\n${clip(parts.openFileText, VOICE_CONTEXT_LIMITS.openFile)}`
      : "";
    blocks.push(`Open file: ${parts.openFile}${body}`);
  }
  if (parts.extra?.trim()) {
    blocks.push(clip(parts.extra, VOICE_CONTEXT_LIMITS.extra));
  }
  const tree = parts.tree ?? [];
  if (tree.length > 0) {
    const shown = tree.slice(0, VOICE_CONTEXT_LIMITS.treeEntries);
    const of = tree.length > shown.length ? ` of ${tree.length}` : "";
    blocks.push(`Project files (${shown.length}${of}):\n${shown.join("\n")}`);
  }

  return clip(blocks.join("\n\n"), VOICE_CONTEXT_LIMITS.total);
}

/**
 * Pick the README a project would want read aloud.
 *
 * Case matters on Linux and not on macOS, and a repo may carry several — take
 * the shallowest, shortest name, which is the root `README.md` in practice.
 */
export function findReadme(tree: readonly string[]): string | undefined {
  return tree
    .filter((p) => /(^|[/\\])readme(\.(md|markdown|rst|txt))?$/i.test(p))
    .sort((a, b) => {
      const depth = (p: string) => p.split(/[/\\]/).length;
      return depth(a) - depth(b) || a.length - b.length;
    })[0];
}
