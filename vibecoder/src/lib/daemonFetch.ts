/**
 * daemonFetch — re-exported from `@vibe/shared`.
 *
 * This file used to hold VibeCoder's own implementation. It was the *good* one
 * — cache, 401 re-read, shared in-flight read — but it was the second of two:
 * `packages/vibe-ui-shared` grew three partial copies of the same thing for
 * VibeDesk and VibeAIChat, none of which retried, so a daemon restart left the
 * speech-settings and harness panels 401ing until the app was relaunched while
 * VibeCoder's own panels recovered fine. One implementation, one behaviour.
 *
 * Kept as a module rather than deleted because ~20 panels import from here, and
 * a path is a cheaper thing to keep stable than an import site is to churn.
 */
export {
  daemonFetch,
  daemonBase,
  daemonUrl,
  getDaemonToken,
  resetDaemonTokenCache,
  daemonReadiness,
  describeDaemonFailure,
  type DaemonReadiness,
  type TokenState,
} from "@vibe/shared/lib/daemonFetch";
