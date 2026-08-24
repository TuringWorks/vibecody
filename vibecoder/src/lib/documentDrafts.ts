/**
 * documentDrafts — unsaved document buffers, kept alive across tab switches.
 *
 * The editor area renders only the active file, so a DOCX/EPUB/Pages buffer is
 * unmounted the moment another tab is clicked. Without this the edit would be
 * gone, silently, with the document still showing its old text — the buffer is
 * not the file on disk, so nothing else in the app is holding it.
 *
 * Session-scoped and in memory on purpose: a draft that outlived the process
 * would be a second, invisible copy of a document people believe they saved.
 */

const drafts = new Map<string, string>();
const modes = new Map<string, "view" | "text">();

/** The unsaved buffer for a path, if there is one. */
export function getDraft(path: string): string | undefined {
  return drafts.get(path);
}

/** Remember an unsaved buffer, or forget it once it matches what was saved. */
export function setDraft(path: string, text: string, savedText: string): void {
  if (text === savedText) {
    drafts.delete(path);
    return;
  }
  drafts.set(path, text);
}

/** Forget a path's draft — after a successful save, or when it is discarded. */
export function clearDraft(path: string): void {
  drafts.delete(path);
}

/** Whether a path has edits that have not been written to the document. */
export function hasDraft(path: string): boolean {
  return drafts.has(path);
}

/** Which pane a document was left in, so returning to the tab lands back there. */
export function getMode(path: string): "view" | "text" | undefined {
  return modes.get(path);
}

export function setMode(path: string, mode: "view" | "text"): void {
  modes.set(path, mode);
}

/** Drop everything for a path — used when its tab is closed. */
export function forgetDocument(path: string): void {
  drafts.delete(path);
  modes.delete(path);
}
