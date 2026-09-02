import { useCallback, useEffect, useState } from "react";
import { daemonFetch, describeDaemonFailure } from "../lib/daemonFetch";

/**
 * The daemon's speech settings, read and written over `/voice/settings`.
 *
 * These live in the daemon's config rather than in each shell's local storage
 * because the daemon is what speaks: the pipeline runs there and a client
 * contributes a microphone and speakers. A per-app copy of "which voice" would
 * be three settings that disagree about one machine.
 *
 * Talks to the daemon through the shared `daemonFetch`, which owns the port,
 * the bearer, and the 401 re-read — a local daemon mints a fresh token on every
 * start and stores it nowhere the frontend can see, so asking the settings
 * store returns null.
 */

export interface VoiceEngine {
  id: string;
  label: string;
  /** False when nothing on this machine can run it. Offering it anyway
   *  produces a setting that appears to apply and silently does nothing. */
  available: boolean;
  detail: string;
}

export interface VoiceChoice {
  id: string;
  name: string;
  lang: string;
  /** `premium` / `enhanced` are neural; `default` is the compact tier that
   *  people mean when they say a system voice sounds robotic. */
  quality: string;
}

/** One addressable model, as the daemon's catalog reports it. */
export interface VoiceModel {
  id: string;
  /** Absent on the synthetic "active provider" row, which cannot be addressed. */
  name?: string;
  provider: string;
  active?: boolean;
}

export interface VoiceSettings {
  engine: string;
  engines: VoiceEngine[];
  voice: string;
  language: string;
  voices: VoiceChoice[];
  languages: string[];
  /**
   * The model that answers a spoken turn, when it should not be the one the
   * app's composer is set to. Null on both → the client's own selection.
   *
   * Worth having its own setting because the two jobs have different budgets:
   * the composer picks a model to write code with, and a spoken reply is
   * measured in seconds of silence — 5.0 s warm and 42 s cold on a 20B here,
   * against 0.58 s of recognition and milliseconds of speech.
   */
  provider: string | null;
  model: string | null;
  /** Everything this daemon can address — the same rows the model pickers show. */
  models: VoiceModel[];
}

export function useVoiceSettings() {
  const [settings, setSettings] = useState<VoiceSettings | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const load = useCallback(async () => {
    let res: Response | null = null;
    try {
      res = await daemonFetch("/voice/settings");
      if (res.ok) {
        setSettings(await res.json());
        setError(null);
        return;
      }
    } catch (e) {
      setError(await describeDaemonFailure("read speech settings", null, e));
      return;
    }
    // A status code is not a diagnosis. This used to render every failure as
    // "(daemon returned 401). Is it running?" — which was the wrong question
    // for two and a half days, against a daemon whose token file had been
    // overwritten by a second daemon on another port.
    setError(await describeDaemonFailure("read speech settings", res));
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const update = useCallback(
    async (
      patch: Partial<Pick<VoiceSettings, "engine" | "voice" | "language">> & {
        /** Both together, or both `""` to hand the choice back to the client.
         *  The daemon rejects half a pair rather than storing a setting that
         *  reads back as a choice and behaves as none. */
        provider?: string;
        model?: string;
      },
    ) => {
      setSaving(true);
      try {
        const res = await daemonFetch("/voice/settings", {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(patch),
        });
        if (!res.ok) {
          // The daemon explains refusals in words — "the neural engine is not
          // installed" is the whole answer, and replacing it with the status
          // code would throw that away. Only when it says nothing does the
          // transport-level diagnosis take over.
          const body = await res.json().catch(() => null);
          throw new Error(
            body?.error ?? (await describeDaemonFailure("save speech settings", res)),
          );
        }
        // Re-read rather than trusting the patch: changing the engine changes
        // the available voices, and the daemon is the one that knows them.
        await load();
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setSaving(false);
      }
    },
    [load],
  );

  return { settings, error, saving, update, reload: load };
}
