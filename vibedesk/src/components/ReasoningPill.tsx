import { useCallback, useRef, useState } from "react";
import { ChevronDown } from "lucide-react";
import { useClickAway } from "@vibe/shared/hooks/useClickAway";

export type ReasoningEffort =
  | "off"
  | "minimal"
  | "low"
  | "medium"
  | "high"
  | "extra-high"
  | "custom";

/** No extended thinking is requested. The composer default.
 *
 * Sent as "no `reasoning` field at all" rather than a value, because the
 * daemon maps an unrecognised effort to `None` and leaves the provider on its
 * own default (`serve.rs: reasoning_effort_to_budget`). Note this asks for no
 * *extra* thinking budget; a model that always reasons (minimax-m3) still
 * returns a thinking block, which the UI keeps collapsed. */
export const REASONING_OFF: ReasoningEffort = "off";

const LABELS: Record<ReasoningEffort, string> = {
  off: "Off",
  minimal: "Minimal",
  low: "Low",
  medium: "Medium",
  high: "High",
  "extra-high": "Extra High",
  custom: "Custom",
};

/**
 * Providers known to support a reasoning-effort knob. The pill hides itself
 * for providers that don't (VX-108 acceptance criterion). Extend as the
 * daemon reports capabilities (VX-111).
 */
const REASONING_PROVIDERS = new Set(["openai", "anthropic", "vibecli-mistralrs", "ollama"]);

interface ReasoningPillProps {
  provider: string;
  value: ReasoningEffort;
  onChange: (v: ReasoningEffort) => void;
}

/**
 * VX-108 — composer reasoning-effort pill (Codex screenshot 2).
 * Minimal/Low/Medium/High/Extra High/Custom. Sends a `reasoning` param the
 * daemon plumbs into the chat request (VX-111); hidden when unsupported.
 */
export function ReasoningPill({ provider, value, onChange }: ReasoningPillProps) {
  const [open, setOpen] = useState(false);
  const wrapRef = useRef<HTMLDivElement>(null);
  useClickAway(open, wrapRef, useCallback(() => setOpen(false), []));
  if (!REASONING_PROVIDERS.has(provider)) return null;

  return (
    <div className="vx-pill-wrap" ref={wrapRef}>
      {open && (
        <ul className="vx-pill-menu" role="menu">
          {(Object.keys(LABELS) as ReasoningEffort[]).map((r) => (
            <li key={r}>
              <button
                role="menuitemradio"
                aria-checked={value === r}
                className="vx-pill-menu__item"
                onClick={() => {
                  onChange(r);
                  setOpen(false);
                }}
              >
                <span className="vx-pill-menu__label">{LABELS[r]}</span>
                {value === r && <span aria-hidden>✓</span>}
              </button>
            </li>
          ))}
        </ul>
      )}
      <button
        className="vx-pill vx-pill--effort"
        onClick={() => setOpen((v) => !v)}
        aria-label="Reasoning effort"
        aria-expanded={open}
        title={`Thinking effort: ${LABELS[value]}`}
      >
        <span>{LABELS[value]}</span>
        <ChevronDown size={12} className="vx-chip__caret" />
      </button>
    </div>
  );
}
