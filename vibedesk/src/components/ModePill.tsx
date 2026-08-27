/** How a turn runs. Mirrors `RunMode` in the daemon's `serve.rs`. */
export type RunMode = "agent" | "chat" | "sandbox";

const MODES: { id: RunMode; label: string; hint: string }[] = [
  { id: "agent", label: "Agent", hint: "Reads and edits files, runs commands" },
  { id: "chat", label: "Chat", hint: "Answers only — no tools, no file access" },
  {
    id: "sandbox",
    label: "Sandbox",
    hint: "Agent, plus the access you grant outside the workspace",
  },
];

interface ModePillProps {
  value: RunMode;
  onChange: (v: RunMode) => void;
}

/**
 * Composer control for how the next turn runs.
 *
 * The agent's system prompt instructs it to reply *only* with a tool call, so
 * asking a plain question in Agent mode gets an answer shaped like work — it
 * starts reading the workspace instead of answering. Chat mode is how you ask
 * something without starting a task.
 *
 * Shown as a segmented switch rather than a dropdown: this is the one control
 * that decides whether a message *does* something, so what the alternatives
 * are — and which one is live — should not cost a click to find out.
 */
export function ModePill({ value, onChange }: ModePillProps) {
  return (
    <div className="vx-seg" role="radiogroup" aria-label="Run mode">
      {MODES.map((m) => (
        <button
          key={m.id}
          type="button"
          role="radio"
          aria-checked={value === m.id}
          className={`vx-seg__opt${value === m.id ? " is-active" : ""}`}
          title={m.hint}
          onClick={() => onChange(m.id)}
        >
          {m.label}
        </button>
      ))}
    </div>
  );
}
