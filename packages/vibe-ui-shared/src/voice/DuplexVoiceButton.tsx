import type { DuplexState } from "./useVoiceDuplex";

/**
 * Toggle for a full-duplex conversation, with the turn state visible.
 *
 * The state is worth showing rather than hiding behind a single "on" light:
 * with an open microphone the user cannot otherwise tell whether the assistant
 * is still listening, thinking, or about to talk over them.
 */
export interface DuplexVoiceButtonProps {
  state: DuplexState;
  active: boolean;
  supported: boolean;
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
  onStart,
  onStop,
  unsupportedHint,
}: DuplexVoiceButtonProps) {
  if (!supported) {
    return (
      <button className="voice-duplex-btn" disabled title={unsupportedHint ?? "Voice is not available here"}>
        Voice unavailable
      </button>
    );
  }
  const title = state.status === "error" ? state.message : LABEL[state.status];
  return (
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
  );
}
