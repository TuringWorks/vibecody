import type { CSSProperties } from "react";

/** Standard panel container with flex column layout */
export const panelContainer: CSSProperties = {
  padding: 16,
  display: "flex",
  flexDirection: "column",
  gap: 16,
  fontSize: 13,
  color: "var(--text-primary)",
};

/** Card with secondary background and border */
export const sectionCard: CSSProperties = {
  background: "var(--bg-secondary)",
  borderRadius: 6,
  padding: 12,
  border: "1px solid var(--border-color)",
};

/** Section heading (14px, bold) */
export const sectionHeading: CSSProperties = {
  fontSize: 14,
  fontWeight: 600,
  marginBottom: 8,
  color: "var(--text-primary)",
};

/** Secondary label text */
export const labelText: CSSProperties = {
  fontSize: 12,
  color: "var(--text-secondary)",
};

/** Standard table base */
export const tableBase: CSSProperties = {
  width: "100%",
  borderCollapse: "collapse",
  fontSize: 13,
};

/** Table body cell */
export const cellBase: CSSProperties = {
  padding: "6px 8px",
  borderBottom: "1px solid var(--border-color)",
};

/** Table header cell */
export const headerCell: CSSProperties = {
  ...cellBase,
  textAlign: "left",
  fontSize: 12,
  fontWeight: 500,
  color: "var(--text-secondary)",
};

/** Monospace text */
export const monoText: CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 12,
};

/** Muted small text (10px) */
export const mutedSmall: CSSProperties = {
  fontSize: 10,
  fontWeight: 700,
  color: "var(--text-muted)",
};

/** Standard input field */
export const inputField: CSSProperties = {
  padding: "6px 10px",
  fontSize: 13,
  background: "var(--bg-primary)",
  color: "var(--text-primary)",
  border: "1px solid var(--border-color)",
  borderRadius: 4,
  fontFamily: "var(--font-mono)",
  outline: "none",
};
