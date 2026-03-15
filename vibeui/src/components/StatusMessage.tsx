/**
 * StatusMessage — Shared component for error, loading, and empty states.
 *
 * Replaces the ad-hoc inline-styled divs scattered across panels with a
 * consistent look. Drop-in usage:
 *   <StatusMessage variant="error" message="Something broke" />
 *   <StatusMessage variant="loading" message="Analyzing…" detail="15–30 s" />
 *   <StatusMessage variant="empty" icon="🔍" message="No results" detail="Try adjusting filters" />
 */

type Variant = "error" | "loading" | "empty" | "success" | "warning";

interface StatusMessageProps {
  variant: Variant;
  message: string;
  detail?: string;
  icon?: string;
  /** Render inline (no centering/padding) for tight layouts */
  inline?: boolean;
}

const STYLES: Record<Variant, { bg: string; fg: string; defaultIcon: string }> = {
  error:   { bg: "var(--error-bg)", fg: "var(--text-danger)", defaultIcon: "⚠" },
  warning: { bg: "var(--warning-bg)",  fg: "var(--text-warning)", defaultIcon: "⚠" },
  loading: { bg: "transparent",            fg: "var(--text-secondary)",        defaultIcon: "⏳" },
  empty:   { bg: "transparent",            fg: "var(--text-secondary)",        defaultIcon: "📭" },
  success: { bg: "var(--success-bg)",    fg: "var(--text-success)", defaultIcon: "✓" },
};

export function StatusMessage({ variant, message, detail, icon, inline }: StatusMessageProps) {
  const s = STYLES[variant];
  const displayIcon = icon ?? s.defaultIcon;

  if (inline) {
    return (
      <div
        role={variant === "error" ? "alert" : "status"}
        style={{
          padding: "6px 10px",
          background: s.bg,
          color: s.fg,
          borderRadius: 4,
          fontSize: 12,
          lineHeight: 1.5,
        }}
      >
        {displayIcon} {message}
        {detail && <span style={{ opacity: 0.7, marginLeft: 6 }}>— {detail}</span>}
      </div>
    );
  }

  return (
    <div
      role={variant === "error" ? "alert" : "status"}
      style={{
        textAlign: "center",
        padding: "32px 16px",
        color: s.fg,
        lineHeight: 1.7,
      }}
    >
      <div style={{ fontSize: 28, marginBottom: 8 }}>{displayIcon}</div>
      <div style={{ fontSize: 13 }}>{message}</div>
      {detail && (
        <div style={{ fontSize: 11, marginTop: 4, opacity: 0.7 }}>{detail}</div>
      )}
    </div>
  );
}
