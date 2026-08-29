/**
 * VibeDesk's model list — a thin binding over the shared hook.
 *
 * The fetch-cache-degrade logic lives in `@vibe/shared/hooks/useDaemonModels`
 * because VibeDesk, VibeAIChat and VibeCoder all needed it and each had grown
 * its own copy. This file keeps VibeDesk's call signature and cache key so
 * nothing here had to change shape.
 */
import {
  useDaemonModels,
  type DaemonModel,
} from "@vibe/shared/hooks/useDaemonModels";

export type { DaemonModel };

/**
 * Model list for the picker, sourced entirely from the daemon's `/models`.
 *
 * No fallback list: VibeDesk has never carried a catalog of its own, and the
 * cache it serves when offline is the daemon's own previous answer.
 */
export function useModels(daemonUrl: string, daemonOnline: boolean): DaemonModel[] {
  return useDaemonModels({
    daemonUrl,
    online: daemonOnline,
    cacheKey: "vibedesk:models:v1",
  }).models;
}
