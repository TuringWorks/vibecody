import { useState } from "react";
import { ShieldAlert, ChevronUp } from "lucide-react";

export type ApprovalTier = "default" | "auto-review" | "full-access";

const LABELS: Record<ApprovalTier, string> = {
  default: "Default permissions",
  "auto-review": "Auto-review",
  "full-access": "Full access",
};

interface ApprovalPillProps {
  value: ApprovalTier;
  onChange: (t: ApprovalTier) => void;
}

/**
 * VX-107 — composer approval-tier pill (Codex screenshot 1).
 * Maps to the daemon's approval policy (Suggest → Auto → Full-auto).
 */
export function ApprovalPill({ value, onChange }: ApprovalPillProps) {
  const [open, setOpen] = useState(false);
  return (
    <div className="vx-pill-wrap">
      {open && (
        <ul className="vx-pill-menu" role="menu">
          {(Object.keys(LABELS) as ApprovalTier[]).map((t) => (
            <li key={t}>
              <button
                role="menuitemradio"
                aria-checked={value === t}
                className="vx-pill-menu__item"
                onClick={() => {
                  onChange(t);
                  setOpen(false);
                }}
              >
                {LABELS[t]} {value === t && "✓"}
              </button>
            </li>
          ))}
        </ul>
      )}
      <button
        className={`vx-pill vx-pill--approval${value === "full-access" ? " vx-pill--warn" : ""}`}
        onClick={() => setOpen((v) => !v)}
        aria-label="Approval tier"
      >
        <ShieldAlert size={13} />
        <span>{LABELS[value]}</span>
        <ChevronUp size={12} />
      </button>
    </div>
  );
}
