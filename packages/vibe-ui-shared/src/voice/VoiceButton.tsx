import { Loader2, Mic, MicOff } from "lucide-react";
import type { UseVoiceInput } from "./useVoiceInput";

interface VoiceButtonProps {
  voice: UseVoiceInput;
  /** Disable while the composer itself is busy (e.g. a run is streaming). */
  disabled?: boolean;
  className?: string;
  size?: number;
}

/**
 * Composer mic button.
 *
 * Renders nothing when the host can't capture voice — an always-visible button
 * that errors on click is worse than no button. The error, when there is one,
 * is shown as the tooltip and reflected in `aria-label`, so the reason a
 * recording failed is reachable without a toast system the shells don't share.
 */
export function VoiceButton({ voice, disabled, className = "", size = 15 }: VoiceButtonProps) {
  if (!voice.supported) return null;

  const { isListening, isTranscribing, error } = voice;
  const label = error
    ? error
    : isTranscribing
      ? "Transcribing…"
      : isListening
        ? "Stop recording"
        : "Dictate (voice input)";

  const stateClass = error
    ? "is-error"
    : isListening
      ? "is-listening"
      : isTranscribing
        ? "is-transcribing"
        : "";

  return (
    <button
      type="button"
      className={`vx-voice-btn ${stateClass} ${className}`.trim()}
      onClick={voice.toggle}
      disabled={disabled || isTranscribing}
      aria-label={label}
      aria-pressed={isListening}
      title={label}
    >
      {isTranscribing ? (
        <Loader2 size={size} className="vx-voice-btn__spin" />
      ) : error ? (
        <MicOff size={size} />
      ) : (
        <Mic size={size} />
      )}
    </button>
  );
}
