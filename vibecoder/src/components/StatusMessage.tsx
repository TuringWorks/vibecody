/**
 * StatusMessage — Shared component for error, loading, and empty states.
 *
 * Replaces the ad-hoc inline-styled divs scattered across panels with a
 * consistent look. Drop-in usage:
 *   <StatusMessage variant="error" message="Something broke" />
 *   <StatusMessage variant="loading" message="Analyzing…" detail="15–30 s" />
 *   <StatusMessage variant="empty" icon={<Search size={20} />} message="No results" detail="Try adjusting filters" />
 */

import React from "react";

type Variant = "error" | "loading" | "empty" | "success" | "warning";

interface StatusMessageProps {
  variant: Variant;
  message: string;
  detail?: string;
  icon?: React.ReactNode;
  /** Render inline (no centering/padding) for tight layouts */
  inline?: boolean;
}

import { AlertTriangle, Loader2, Inbox, CheckCircle } from "lucide-react";

const DEFAULT_ICONS: Record<Variant, React.ReactNode> = {
  error:   <AlertTriangle size={20} strokeWidth={1.5} />,
  warning: <AlertTriangle size={20} strokeWidth={1.5} />,
  loading: <Loader2 size={20} strokeWidth={1.5} className="spin" />,
  empty:   <Inbox size={20} strokeWidth={1.5} />,
  success: <CheckCircle size={20} strokeWidth={1.5} />,
};

const STYLES: Record<Variant, { bg: string; fg: string }> = {
  error:   { bg: "var(--error-bg)", fg: "var(--text-danger)" },
  warning: { bg: "var(--warning-bg)",  fg: "var(--text-warning)" },
  loading: { bg: "transparent",            fg: "var(--text-secondary)" },
  empty:   { bg: "transparent",            fg: "var(--text-secondary)" },
  success: { bg: "var(--success-bg)",    fg: "var(--text-success)" },
};

export function StatusMessage({ variant, message, detail, icon, inline }: StatusMessageProps) {
  const s = STYLES[variant];
  const displayIcon = icon ?? DEFAULT_ICONS[variant];

  if (inline) {
    return (
      <div
        role={variant === "error" ? "alert" : "status"}
        style={{
          padding: "6px 10px",
          background: s.bg,
          color: s.fg,
          borderRadius: "var(--radius-xs-plus)",
          fontSize: "var(--font-size-base)",
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
      <div style={{ fontSize: "var(--font-size-md)" }}>{message}</div>
      {detail && (
        <div style={{ fontSize: "var(--font-size-sm)", marginTop: 4, opacity: 0.7 }}>{detail}</div>
      )}
    </div>
  );
}
