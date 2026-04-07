/**
 * SkillDistillationPanel — Cross-Session Learning dashboard.
 *
 * View distilled patterns from coding sessions, inspect pattern types and
 * confidence levels, and export learned skills as reusable skill files.
 */
import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface DistillStatus {
  sessionsAnalyzed: number;
  patternsExtracted: number;
  skillsGenerated: number;
  improvementEstimate: number;
}

interface Pattern {
  id: string;
  rule: string;
  type: string;
  confidence: string;
  occurrences: number;
  description: string;
}

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", flex: 1, minHeight: 0, overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12, marginRight: 8 };
const tabRow: React.CSSProperties = { display: "flex", gap: 4, marginBottom: 12 };
const metricBox: React.CSSProperties = { textAlign: "center", padding: 12, borderRadius: 6, background: "var(--bg-tertiary)", flex: 1 };
const badgeStyle = (color: string): React.CSSProperties => ({ fontSize: 10, padding: "2px 6px", borderRadius: 3, background: color, color: "#fff", marginLeft: 6 });

type Tab = "overview" | "patterns" | "export";

export default function SkillDistillationPanel() {
  const [tab, setTab] = useState<Tab>("overview");
  const [status, setStatus] = useState<DistillStatus | null>(null);
  const [patterns, setPatterns] = useState<Pattern[]>([]);
  const [exported, setExported] = useState("");
  const [loading, setLoading] = useState(false);

  const loadStatus = useCallback(async () => {
    try {
      const res = await invoke<DistillStatus>("distill_status");
      setStatus(res);
    } catch (e) { console.error(e); }
  }, []);

  const loadPatterns = useCallback(async () => {
    setLoading(true);
    try {
      const res = await invoke<{ patterns: Pattern[] }>("distill_patterns");
      setPatterns(res.patterns);
    } catch (e) { console.error(e); }
    setLoading(false);
  }, []);

  const doExport = useCallback(async () => {
    setLoading(true);
    try {
      const res = await invoke<string>("distill_export");
      setExported(res);
    } catch (e) { console.error(e); }
    setLoading(false);
  }, []);

  useEffect(() => { loadStatus(); }, [loadStatus]);

  const confidenceColor = (c: string) => c.includes("Confident") ? "var(--success-color)" : c.includes("Tentative") ? "var(--warning-color)" : "var(--error-color)";

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Skill Distillation</h2>
      <div style={tabRow}>
        {(["overview", "patterns", "export"] as Tab[]).map(t => (
          <button key={t} style={{ ...btnStyle, background: tab === t ? "var(--accent-color)" : "var(--bg-tertiary)", color: tab === t ? "#fff" : "var(--text-primary)" }} onClick={() => { setTab(t); if (t === "patterns") loadPatterns(); if (t === "export") doExport(); }}>
            {t === "overview" ? "Overview" : t === "patterns" ? "Patterns" : "Export"}
          </button>
        ))}
      </div>

      {tab === "overview" && status && (
        <>
          <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
            <div style={metricBox}>
              <div style={{ fontSize: 24, fontWeight: 700 }}>{status.sessionsAnalyzed}</div>
              <div style={labelStyle}>Sessions Analyzed</div>
            </div>
            <div style={metricBox}>
              <div style={{ fontSize: 24, fontWeight: 700 }}>{status.patternsExtracted}</div>
              <div style={labelStyle}>Patterns Found</div>
            </div>
            <div style={metricBox}>
              <div style={{ fontSize: 24, fontWeight: 700 }}>{status.skillsGenerated}</div>
              <div style={labelStyle}>Skills Generated</div>
            </div>
          </div>
          <div style={cardStyle}>
            <div style={labelStyle}>Improvement Estimate</div>
            <div style={{ fontSize: 18, fontWeight: 600, color: status.improvementEstimate > 0 ? "var(--success-color)" : "var(--text-secondary)" }}>
              {status.improvementEstimate > 0 ? `+${(status.improvementEstimate * 100).toFixed(1)}%` : "Not enough data"}
            </div>
            <div style={{ ...labelStyle, marginTop: 4 }}>
              Estimated coding speed improvement from learned patterns.
            </div>
          </div>
        </>
      )}

      {tab === "patterns" && (
        <>
          {patterns.length === 0 && !loading && (
            <div style={labelStyle}>No patterns learned yet. Patterns emerge after 3+ coding sessions.</div>
          )}
          {patterns.map(p => (
            <div key={p.id || p.rule} style={cardStyle}>
              <div style={{ display: "flex", alignItems: "center", marginBottom: 4 }}>
                <span style={{ fontWeight: 600 }}>{p.rule}</span>
                <span style={badgeStyle(confidenceColor(p.confidence))}>{p.confidence}</span>
                <span style={{ ...badgeStyle("var(--bg-tertiary)"), color: "var(--text-secondary)" }}>{p.type}</span>
              </div>
              <div>{p.description}</div>
              <div style={{ ...labelStyle, marginTop: 4 }}>Seen {p.occurrences}x</div>
            </div>
          ))}
        </>
      )}

      {tab === "export" && (
        <div style={cardStyle}>
          <div style={labelStyle}>Exported Skills (Markdown)</div>
          {exported ? (
            <pre style={{ whiteSpace: "pre-wrap", fontSize: 11, fontFamily: "monospace", margin: 0, maxHeight: 400, overflow: "auto" }}>{exported}</pre>
          ) : (
            <div style={labelStyle}>{loading ? "Exporting..." : "No skills to export yet."}</div>
          )}
        </div>
      )}
    </div>
  );
}
