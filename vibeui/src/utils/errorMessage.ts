/**
 * Extract a human-readable message from an unknown thrown value — without `any`.
 *
 * Tauri / HTTP boundaries reject with either a plain string (most vibecli
 * commands surface errors as strings across IPC) or an `Error`-like object with
 * a `.message`. This narrows both totally, replacing the repeated
 * `typeof e === "string" ? e : e?.message` pattern that previously forced
 * `catch (e: any)` at every call site.
 *
 * Returns `undefined` when no message can be recovered, so callers keep their
 * own fallback: `errorMessage(e) || "Failed to load board"`.
 */
export function errorMessage(e: unknown): string | undefined {
  if (typeof e === "string") return e;
  if (e != null && typeof e === "object" && "message" in e) {
    const m = (e as { message?: unknown }).message;
    if (typeof m === "string") return m;
    if (m != null) return String(m);
  }
  return undefined;
}
