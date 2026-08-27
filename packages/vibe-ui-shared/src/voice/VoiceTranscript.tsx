import type { DuplexState, DuplexTurn } from "./useVoiceDuplex";

/**
 * What is being said, while it is being said.
 *
 * A spoken turn used to leave nothing behind: the microphone heard a question,
 * the speaker answered it, and the window showed neither. The host's chat log
 * gets the *completed* turns via `onTurn`, but a completed turn arrives after
 * the assistant has finished talking — for the seconds in between, the only
 * evidence the feature was working was audio.
 *
 * So this renders the tail of `turns` live: the last thing heard, and the reply
 * as it is spoken sentence by sentence. It is a caption, not a transcript —
 * scrollback belongs to the host's own conversation view.
 */
export interface VoiceTranscriptProps {
  state: DuplexState;
  turns: readonly DuplexTurn[];
  /** Whether a conversation is running. Nothing renders when it is not. */
  active: boolean;
}

/** What the caption says when there is nothing to quote yet. */
const WAITING: Partial<Record<DuplexState["status"], string>> = {
  connecting: "Connecting…",
  listening: "Listening — just start talking.",
  hearing: "Listening…",
  thinking: "Thinking…",
};

export function VoiceTranscript({ state, turns, active }: VoiceTranscriptProps) {
  if (state.status === "error") {
    return (
      <div className="voice-caption voice-caption--error" role="status">
        {state.message}
      </div>
    );
  }
  if (!active) return null;

  // The tail, not the whole conversation: everything older has already been
  // handed to the host's chat log, and repeating it here would show every turn
  // twice.
  const lastAssistant = [...turns].reverse().find((t) => t.role === "assistant");
  const lastUser = [...turns].reverse().find((t) => t.role === "user");
  // Once the assistant has begun answering, the question above it is the one it
  // is answering. Before that, the newest thing said is the user's.
  const showUser =
    lastUser && (!lastAssistant || turns.indexOf(lastUser) > turns.indexOf(lastAssistant));

  const waiting = !lastAssistant || showUser ? WAITING[state.status] : undefined;

  return (
    <div className={`voice-caption state-${state.status}`} aria-live="polite">
      <span className="voice-caption__state">
        <span className="voice-duplex-dot" aria-hidden="true" />
      </span>
      <div className="voice-caption__lines">
        {lastUser && (
          <p className="voice-caption__line voice-caption__line--user">{lastUser.text}</p>
        )}
        {lastAssistant && !showUser && (
          <p className="voice-caption__line voice-caption__line--assistant">
            {lastAssistant.text}
          </p>
        )}
        {waiting && <p className="voice-caption__line voice-caption__line--waiting">{waiting}</p>}
      </div>
    </div>
  );
}
