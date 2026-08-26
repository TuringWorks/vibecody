import type { DuplexState } from "./useVoiceDuplex";

/**
 * The full-duplex voice control, and the switch that governs it.
 *
 * Two states rather than one, because an open microphone deserves an explicit
 * opt-in:
 *
 *   disabled  →  "Voice off". One click enables the feature. It does **not**
 *                open the microphone — that is a second, deliberate click.
 *   enabled   →  a start/stop button showing the turn state, plus a way to
 *                switch the feature back off without hunting through settings.
 *
 * The turn state is worth showing rather than hiding behind one "on" light:
 * with the microphone open the user cannot otherwise tell whether the assistant
 * is listening, thinking, or about to talk over them.
 */
export interface DuplexVoiceButtonProps {
  state: DuplexState;
  active: boolean;
  supported: boolean;
  /** Persisted opt-in. Off by default — see `useVoiceDuplexPreference`. */
  enabled: boolean;
  onEnabledChange: (on: boolean) => void;
  onStart: () => void;
  onStop: () => void;
  /** Hosts that cannot do echo cancellation should say so rather than offer a
   *  control that will make the assistant interrupt itself. */
  unsupportedHint?: string;
}

const LABEL: Record<DuplexState["status"], string> = {
  idle: "Start voice",
  connecting: "Connecting…",
  listening: "Listening",
  hearing: "Hearing you",
  thinking: "Thinking",
  speaking: "Speaking",
  error: "Voice error",
};

export function DuplexVoiceButton({
  state,
  active,
  supported,
  enabled,
  onEnabledChange,
  onStart,
  onStop,
  unsupportedHint,
}: DuplexVoiceButtonProps) {
  if (!supported) {
    return (
      <button
        className="voice-duplex-btn"
        disabled
        title={unsupportedHint ?? "Voice is not available here"}
      >
        Voice unavailable
      </button>
    );
  }

  if (!enabled) {
    return (
      <button
        className="voice-duplex-btn off"
        onClick={() => onEnabledChange(true)}
        title="Turn on voice conversation. The microphone stays open while it is on; you start and stop it separately."
        aria-label="Turn on voice conversation"
        aria-pressed={false}
      >
        <span className="voice-duplex-dot" aria-hidden="true" />
        Voice off
      </button>
    );
  }

  const title = state.status === "error" ? state.message : LABEL[state.status];
  return (
    <span className="voice-duplex-group">
      <button
        className={`voice-duplex-btn ${active ? "active" : ""} state-${state.status}`}
        onClick={active ? onStop : onStart}
        title={title}
        aria-label={active ? "Stop voice conversation" : "Start voice conversation"}
        aria-pressed={active}
      >
        <span className="voice-duplex-dot" aria-hidden="true" />
        {LABEL[state.status]}
      </button>
      <button
        className="voice-duplex-disable"
        onClick={() => {
          // Stop first: switching the feature off must close the microphone,
          // not merely hide the control that was holding it open.
          onStop();
          onEnabledChange(false);
        }}
        title="Turn off voice conversation"
        aria-label="Turn off voice conversation"
      >
        ×
      </button>
    </span>
  );
}
