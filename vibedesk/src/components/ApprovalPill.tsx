import { useCallback, useRef, useState } from "react";
import { ShieldAlert, ShieldCheck, ChevronDown } from "lucide-react";
import { useClickAway } from "@vibe/shared/hooks/useClickAway";

export type ApprovalTier = "default" | "auto-review" | "full-access";

const TIERS: { id: ApprovalTier; label: string; hint: string }[] = [
  { id: "default", label: "Ask first", hint: "Approve each edit and command" },
  { id: "auto-review", label: "Auto-review", hint: "Runs, then shows you the diff" },
  { id: "full-access", label: "Full access", hint: "No prompts — edits and runs freely" },
];

const LABELS: Record<ApprovalTier, string> = {
  default: "Ask first",
  "auto-review": "Auto-review",
  "full-access": "Full access",
};

interface ApprovalPillProps {
  value: ApprovalTier;
  onChange: (t: ApprovalTier) => void;
}

/**
 * VX-107 — composer approval-tier control (Codex screenshot 1).
 * Maps to the daemon's approval policy (Suggest → Auto → Full-auto).
 *
 * Lives on the context row under the composer with the other facts about how
 * the run is set up. It is styled as plain text, not an outlined pill: only
 * `full-access` earns colour, because only `full-access` is a standing grant
 * the user should notice. Making every tier shout made the one that matters
 * indistinguishable from the safe default.
 */
export function ApprovalPill({ value, onChange }: ApprovalPillProps) {
  const [open, setOpen] = useState(false);
  const wrapRef = useRef<HTMLDivElement>(null);
  useClickAway(open, wrapRef, useCallback(() => setOpen(false), []));

  const risky = value === "full-access";
  const Icon = risky ? ShieldAlert : ShieldCheck;

  return (
    <div className="vx-pill-wrap" ref={wrapRef}>
      {open && (
        <ul className="vx-pill-menu" role="menu">
          {TIERS.map((t) => (
            <li key={t.id}>
              <button
                role="menuitemradio"
                aria-checked={value === t.id}
                className={`vx-pill-menu__item${value === t.id ? " is-active" : ""}`}
                onClick={() => {
                  onChange(t.id);
                  setOpen(false);
                }}
              >
                <span className="vx-pill-menu__label">{t.label}</span>
                <span className="vx-pill-menu__hint">{t.hint}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
      <button
        className={`vx-chip${risky ? " vx-chip--warn" : ""}`}
        onClick={() => setOpen((v) => !v)}
        aria-label="Approval tier"
        aria-expanded={open}
        title={TIERS.find((t) => t.id === value)?.hint}
      >
        <Icon size={13} />
        <span>{LABELS[value]}</span>
        <ChevronDown size={12} className="vx-chip__caret" />
      </button>
    </div>
  );
}
