import { useState } from "react";
import { Brain, ChevronUp } from "lucide-react";

export type ReasoningEffort = "minimal" | "low" | "medium" | "high" | "extra-high" | "custom";

const LABELS: Record<ReasoningEffort, string> = {
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
  if (!REASONING_PROVIDERS.has(provider)) return null;

  return (
    <div className="vx-pill-wrap">
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
                {LABELS[r]} {value === r && "✓"}
              </button>
            </li>
          ))}
        </ul>
      )}
      <button className="vx-pill" onClick={() => setOpen((v) => !v)} aria-label="Reasoning effort">
        <Brain size={13} />
        <span>{LABELS[value]}</span>
        <ChevronUp size={12} />
      </button>
    </div>
  );
}
