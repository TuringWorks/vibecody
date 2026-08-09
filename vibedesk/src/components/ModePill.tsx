import { useState } from "react";
import { Bot, MessageCircle, FlaskConical, ChevronUp } from "lucide-react";

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

const ICON = { agent: Bot, chat: MessageCircle, sandbox: FlaskConical } as const;

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
 */
export function ModePill({ value, onChange }: ModePillProps) {
  const [open, setOpen] = useState(false);
  const active = MODES.find((m) => m.id === value) ?? MODES[0];
  const Icon = ICON[active.id];

  return (
    <div className="vx-pill-wrap">
      {open && (
        <ul className="vx-pill-menu" role="menu">
          {MODES.map((m) => (
            <li key={m.id}>
              <button
                role="menuitemradio"
                aria-checked={value === m.id}
                className={`vx-pill-menu__item${value === m.id ? " is-active" : ""}`}
                onClick={() => {
                  onChange(m.id);
                  setOpen(false);
                }}
              >
                <span className="vx-pill-menu__label">{m.label}</span>
                <span className="vx-pill-menu__hint">{m.hint}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
      <button
        className="vx-pill"
        onClick={() => setOpen((v) => !v)}
        aria-label="Run mode"
        title={active.hint}
      >
        <Icon size={13} /> {active.label} <ChevronUp size={12} />
      </button>
    </div>
  );
}
