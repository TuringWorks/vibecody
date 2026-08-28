import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * The daemon's speech settings, read and written over `/voice/settings`.
 *
 * These live in the daemon's config rather than in each shell's local storage
 * because the daemon is what speaks: the pipeline runs there and a client
 * contributes a microphone and speakers. A per-app copy of "which voice" would
 * be three settings that disagree about one machine.
 *
 * Resolves the daemon's port and token the same way `useVoiceDuplex` does — a
 * local daemon mints a fresh token on every start and stores it nowhere the
 * frontend can see, so asking the settings store returns null.
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

export interface VoiceSettings {
  engine: string;
  engines: VoiceEngine[];
  voice: string;
  language: string;
  voices: VoiceChoice[];
  languages: string[];
}

async function daemonBase(): Promise<{ url: string; token: string }> {
  let url = "http://127.0.0.1:7878";
  try {
    url = `http://127.0.0.1:${await invoke<number>("daemon_port")}`;
  } catch {
    /* a host without the command falls back to the default port */
  }
  let token = "";
  try {
    token = (await invoke<string | null>("daemon_token_effective", { explicit: null })) ?? "";
  } catch {
    /* an unauthenticated daemon */
  }
  return { url, token };
}

export function useVoiceSettings() {
  const [settings, setSettings] = useState<VoiceSettings | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const load = useCallback(async () => {
    try {
      const { url, token } = await daemonBase();
      const res = await fetch(`${url}/voice/settings`, {
        headers: token ? { Authorization: `Bearer ${token}` } : {},
      });
      if (!res.ok) throw new Error(`daemon returned ${res.status}`);
      setSettings(await res.json());
      setError(null);
    } catch (e) {
      // Name the daemon. "Failed to fetch" sends people to their network
      // settings for a process that is simply not running.
      setError(
        `Could not read speech settings from the daemon (${
          e instanceof Error ? e.message : String(e)
        }). Is it running?`,
      );
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const update = useCallback(
    async (patch: Partial<Pick<VoiceSettings, "engine" | "voice" | "language">>) => {
      setSaving(true);
      try {
        const { url, token } = await daemonBase();
        const res = await fetch(`${url}/voice/settings`, {
          method: "PUT",
          headers: {
            "Content-Type": "application/json",
            ...(token ? { Authorization: `Bearer ${token}` } : {}),
          },
          body: JSON.stringify(patch),
        });
        if (!res.ok) {
          // The daemon explains refusals in words — "the neural engine is not
          // installed" is the whole answer, and replacing it with the status
          // code would throw that away.
          const body = await res.json().catch(() => null);
          throw new Error(body?.error ?? `daemon returned ${res.status}`);
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
