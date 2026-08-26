/**
 * Single-letter codes for git file status.
 *
 * The backend sends whole words (`vibe_core::git::FileStatus`), which are too
 * wide for a file row. Abbreviating with `charAt(0)` happens to work today only
 * because the seven variants have distinct initials — add "Untracked" beside
 * "Unknown" and two different states silently render the same letter. So the
 * mapping is written out, and an unrecognised status falls through to `?`
 * rather than to a letter that means something else.
 */
export type GitFileStatus =
  | "Modified"
  | "New"
  | "Deleted"
  | "Renamed"
  | "Ignored"
  | "Conflicted"
  | "Unknown";

const CODES: Record<GitFileStatus, string> = {
  Modified: "M",
  New: "N",
  Deleted: "D",
  Renamed: "R",
  Ignored: "I",
  Conflicted: "C",
  Unknown: "?",
};

/** `"Modified"` → `"M"`. Unrecognised input gives `"?"`, never a wrong letter. */
export function gitStatusCode(status: string): string {
  return CODES[status as GitFileStatus] ?? "?";
}

/**
 * The full word, for a tooltip and for assistive technology.
 *
 * A bare letter is not self-explanatory — it needs an accessible name, or the
 * only thing a screen reader announces for a changed file is "M".
 */
export function gitStatusLabel(status: string): string {
  return status in CODES ? status : `Unrecognised status: ${status}`;
}
