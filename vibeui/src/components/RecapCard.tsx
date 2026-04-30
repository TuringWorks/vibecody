// RecapCard — F2.2 surface that pins a session-recap above a restored
// chat transcript. Pure presentational: renders the four canonical
// blocks (headline, bullets, next-actions, artifacts) and surfaces a
// "Resume from here" button. Caller wires onResume to whatever
// transport it likes (Tauri `invoke` in production, `vi.fn()` in
// tests). Spec: docs/design/recap-resume/01-session.md § Per-surface UX.

import React, { useState } from "react";
import { ChevronDown, ChevronRight, Play, FileText, Link as LinkIcon, Briefcase, GitBranch } from "lucide-react";
import type { Recap, RecapArtifact } from "../types/recap";

export interface RecapCardProps {
  recap: Recap;
  /** Fired when the user clicks "Resume from here". */
  onResume?: (recap: Recap) => void;
  /** Defaults to false (open). The design says cards open by default
   * on tab restore so the user sees what's there before scrolling. */
  defaultCollapsed?: boolean;
}

const cardStyle: React.CSSProperties = {
  margin: "8px 12px",
  padding: 12,
  background: "var(--bg-secondary)",
  border: "1px solid var(--border-color)",
  borderRadius: "var(--radius-md)",
  fontSize: "var(--font-size-md)",
  color: "var(--text-primary)",
};

const headerBtnStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 8,
  width: "100%",
  background: "none",
  border: "none",
  color: "var(--text-primary)",
  cursor: "pointer",
  padding: 0,
  textAlign: "left",
};

const headlineStyle: React.CSSProperties = {
  fontWeight: 600,
  fontSize: "var(--font-size-md)",
  flex: 1,
  lineHeight: 1.3,
};

const sectionLabelStyle: React.CSSProperties = {
  fontSize: "var(--font-size-sm)",
  color: "var(--text-secondary)",
  textTransform: "uppercase",
  letterSpacing: 0.5,
  margin: "12px 0 4px",
  fontWeight: 600,
};

const listStyle: React.CSSProperties = {
  margin: "0 0 0 18px",
  padding: 0,
  fontSize: "var(--font-size-sm)",
  color: "var(--text-primary)",
  lineHeight: 1.5,
};

const generatorBadgeStyle = (generator: Recap["generator"]): React.CSSProperties => ({
  display: "inline-block",
  fontSize: "var(--font-size-xs, 11px)",
  padding: "1px 6px",
  borderRadius: 8,
  background:
    generator.type === "llm"
      ? "var(--accent-bg, rgba(80,160,255,0.15))"
      : generator.type === "user_edited"
      ? "rgba(255,200,80,0.15)"
      : "var(--bg-tertiary, rgba(255,255,255,0.05))",
  color: "var(--text-secondary)",
  textTransform: "lowercase",
});

function generatorLabel(generator: Recap["generator"]): string {
  if (generator.type === "llm") return `LLM · ${generator.provider}/${generator.model}`;
  if (generator.type === "user_edited") return "user-edited";
  return "heuristic";
}

function ArtifactIcon({ kind }: { kind: RecapArtifact["kind"] }) {
  const size = 12;
  if (kind === "file") return <FileText size={size} />;
  if (kind === "url") return <LinkIcon size={size} />;
  if (kind === "job") return <Briefcase size={size} />;
  if (kind === "diff") return <GitBranch size={size} />;
  return null;
}

export function RecapCard({ recap, onResume, defaultCollapsed = false }: RecapCardProps) {
  const [collapsed, setCollapsed] = useState(defaultCollapsed);

  return (
    <div role="region" aria-label="Session recap" style={cardStyle}>
      <button
        type="button"
        aria-expanded={!collapsed}
        aria-label="Toggle recap"
        onClick={() => setCollapsed(c => !c)}
        style={headerBtnStyle}
      >
        {collapsed ? <ChevronRight size={14} /> : <ChevronDown size={14} />}
        <span style={headlineStyle}>{recap.headline}</span>
        <span aria-label="Generator" style={generatorBadgeStyle(recap.generator)}>
          {generatorLabel(recap.generator)}
        </span>
      </button>

      {!collapsed && (
        <>
          {recap.bullets.length > 0 && (
            <>
              <div style={sectionLabelStyle}>What happened</div>
              <ul style={listStyle}>
                {recap.bullets.map((b, i) => (
                  <li key={i}>{b}</li>
                ))}
              </ul>
            </>
          )}

          {recap.next_actions.length > 0 && (
            <>
              <div style={sectionLabelStyle}>Next</div>
              <ul style={listStyle}>
                {recap.next_actions.map((a, i) => (
                  <li key={i}>{a}</li>
                ))}
              </ul>
            </>
          )}

          {recap.artifacts.length > 0 && (
            <>
              <div style={sectionLabelStyle}>Artifacts</div>
              <ul style={{ ...listStyle, listStyle: "none", marginLeft: 0 }}>
                {recap.artifacts.map((a, i) => (
                  <li key={i} style={{ display: "flex", alignItems: "center", gap: 6, padding: "2px 0" }}>
                    <ArtifactIcon kind={a.kind} />
                    <span>{a.label}</span>
                    <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-xs, 11px)" }}>
                      {a.locator}
                    </span>
                  </li>
                ))}
              </ul>
            </>
          )}

          <div style={{ marginTop: 12, display: "flex", justifyContent: "flex-end" }}>
            <button
              type="button"
              aria-label="Resume from here"
              onClick={() => onResume?.(recap)}
              className="panel-btn"
              style={{
                display: "flex", alignItems: "center", gap: 6,
                padding: "4px 10px",
                background: "var(--accent-color)",
                color: "var(--bg-primary)",
                border: "none",
                borderRadius: "var(--radius-sm)",
                cursor: "pointer",
                fontWeight: 600,
              }}
            >
              <Play size={12} />
              Resume from here
            </button>
          </div>
        </>
      )}
    </div>
  );
}

export default RecapCard;
