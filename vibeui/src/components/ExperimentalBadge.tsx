import { ReactNode } from "react";

/**
 * ExperimentalBadge — small amber pill marking surfaces that are real
 * but may change or break. Different semantics from `SimulationModeBadge`:
 *
 *   - SimulationModeBadge = "this panel renders fake / illustrative data"
 *   - ExperimentalBadge   = "this feature is real but may change or break"
 *
 * Per `docs/design/feature-flags/README.md` Day-1 matrix, Experimental
 * surfaces are off by default behind a feature flag once the flag system
 * lands. Until then the badge is the visible signal that the surface
 * isn't GA — users can opt into it but shouldn't build production
 * workflows on top until it's promoted to Beta.
 *
 * Two render shapes:
 *   - Inline pill (default) — for tabs, list items, headers
 *   - Block banner          — when `as="banner"` — for the top of a panel
 *
 * Either shape includes a `title` tooltip and a screen-reader hint, so
 * AT users hear "experimental: <feature> may change or break" rather
 * than just "experimental".
 */
export interface ExperimentalBadgeProps {
  /** Short feature name for the screen-reader hint, e.g. "RL-OS dashboard". */
  feature?: string;
  /** Custom tooltip text. Default: "This feature may change or break." */
  tooltip?: string;
  /** Pill (inline, default) or banner (full-width). */
  as?: "pill" | "banner";
  /** Optional trailing content rendered after the label, e.g. a link or "?". */
  children?: ReactNode;
}

const PILL_STYLE: React.CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  gap: 4,
  padding: "1px 6px",
  fontSize: "var(--font-size-xs, 11px)",
  fontWeight: 600,
  letterSpacing: 0.5,
  textTransform: "uppercase",
  borderRadius: 4,
  // Amber on a transparent tint so the pill works on dark + light themes.
  background: "rgba(245, 158, 11, 0.18)",
  color: "var(--accent-amber, #f59e0b)",
  border: "1px solid rgba(245, 158, 11, 0.35)",
  cursor: "help",
};

const BANNER_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 8,
  padding: "6px 12px",
  margin: "8px 0",
  background: "rgba(245, 158, 11, 0.10)",
  color: "var(--text-primary)",
  border: "1px solid rgba(245, 158, 11, 0.35)",
  borderLeft: "3px solid var(--accent-amber, #f59e0b)",
  borderRadius: "var(--radius-sm)",
  fontSize: "var(--font-size-sm)",
};

export function ExperimentalBadge({
  feature,
  tooltip = "This feature may change or break.",
  as = "pill",
  children,
}: ExperimentalBadgeProps) {
  const srText = feature
    ? `Experimental — ${feature} may change or break.`
    : `Experimental — ${tooltip}`;

  if (as === "banner") {
    return (
      <div role="note" aria-label={srText} style={BANNER_STYLE}>
        <span style={{ ...PILL_STYLE, cursor: "default" }} aria-hidden>
          Experimental
        </span>
        <span style={{ color: "var(--text-secondary)" }}>{tooltip}</span>
        {children}
      </div>
    );
  }

  return (
    <span
      role="note"
      aria-label={srText}
      title={tooltip}
      style={PILL_STYLE}
    >
      Experimental
      {children}
    </span>
  );
}

export default ExperimentalBadge;
