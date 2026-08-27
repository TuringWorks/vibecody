/**
 * "May I write src/main.rs?" — asked out loud, answered with a click.
 *
 * The assistant speaks the question because the user may not be looking at the
 * window; the answer is a button because a spoken "yes" is a word the
 * microphone *thought* it heard, and the cost of getting that wrong is an
 * overwritten file. The two halves are deliberate: hearing is how you learn
 * there is a question, clicking is how you consent.
 *
 * Refusal is the default everywhere — the daemon treats a timeout, a closed
 * socket and a malformed answer all as "no", so nothing here needs to guess.
 */
export interface VoiceApprovalProps {
  /** The pending question, or null when there is none. */
  approval: { question: string } | null;
  onRespond: (approved: boolean) => void;
}

export function VoiceApproval({ approval, onRespond }: VoiceApprovalProps) {
  if (!approval) return null;

  return (
    <div className="voice-approval" role="alertdialog" aria-label="The assistant wants to make a change">
      <p className="voice-approval__question">{approval.question}</p>
      <div className="voice-approval__actions">
        <button
          type="button"
          className="voice-approval__btn voice-approval__btn--deny"
          onClick={() => onRespond(false)}
        >
          No
        </button>
        <button
          type="button"
          className="voice-approval__btn voice-approval__btn--allow"
          onClick={() => onRespond(true)}
          autoFocus
        >
          Yes, do it
        </button>
      </div>
    </div>
  );
}
