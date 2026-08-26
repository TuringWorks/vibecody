import { useCallback, useEffect, useState } from "react";

/**
 * Whether full-duplex voice is enabled at all, persisted per machine.
 *
 * **Defaults to off, deliberately.** Duplex holds the microphone open for the
 * whole session — including while the assistant speaks, which is the point —
 * and a feature that opens a microphone should be something a person turned on,
 * not something they have to notice and turn off. "Idle until clicked" is not
 * the same promise: it leaves a live control one misclick from an open mic.
 *
 * Shared key, so a preference set in one shell is respected by the others: a
 * user who turns voice off in VibeCoder does not expect VibeDesk to keep
 * offering it.
 */
const KEY = "vibe.voice.duplexEnabled";

function read(): boolean {
  try {
    return localStorage.getItem(KEY) === "true";
  } catch {
    // Private windows and blocked site data throw on access. Off is the safe
    // answer for a microphone.
    return false;
  }
}

export interface VoiceDuplexPreference {
  enabled: boolean;
  setEnabled: (on: boolean) => void;
}

export function useVoiceDuplexPreference(): VoiceDuplexPreference {
  const [enabled, setEnabledState] = useState(read);

  const setEnabled = useCallback((on: boolean) => {
    setEnabledState(on);
    try {
      localStorage.setItem(KEY, String(on));
    } catch {
      /* preference is in-memory for this session; not worth failing over */
    }
    // Same-window listeners: `storage` only fires in *other* windows.
    window.dispatchEvent(new CustomEvent("vibe-voice-duplex-pref", { detail: on }));
  }, []);

  useEffect(() => {
    const onLocal = (e: Event) => setEnabledState(!!(e as CustomEvent).detail);
    const onStorage = (e: StorageEvent) => {
      if (e.key === KEY) setEnabledState(e.newValue === "true");
    };
    window.addEventListener("vibe-voice-duplex-pref", onLocal);
    window.addEventListener("storage", onStorage);
    return () => {
      window.removeEventListener("vibe-voice-duplex-pref", onLocal);
      window.removeEventListener("storage", onStorage);
    };
  }, []);

  return { enabled, setEnabled };
}
