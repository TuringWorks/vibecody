/**
 * Elapsed-time formatting for panels that report how long a run has been going.
 *
 * Milliseconds below a second, one decimal of seconds up to a minute, then
 * minutes — a running counter is only useful if the digit that moves is the one
 * the eye can follow.
 */
export function formatElapsed(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return "0ms";
  if (ms < 1000) return `${Math.round(ms)}ms`;
  const seconds = ms / 1000;
  if (seconds < 60) return `${seconds.toFixed(1)}s`;
  return `${Math.floor(seconds / 60)}m ${Math.round(seconds % 60)}s`;
}
