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

const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
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
    <div className="panel-container">
      <h2 style={headingStyle}>Skill Distillation</h2>
      <div style={{ display: "flex", gap: 4, marginBottom: 12 }}>
        {(["overview", "patterns", "export"] as Tab[]).map(t => (
          <button key={t} className={`panel-btn ${tab === t ? "panel-btn-primary" : "panel-btn-secondary"}`} onClick={() => { setTab(t); if (t === "patterns") loadPatterns(); if (t === "export") doExport(); }}>
            {t === "overview" ? "Overview" : t === "patterns" ? "Patterns" : "Export"}
          </button>
        ))}
      </div>

      {tab === "overview" && status && (
        <>
          <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
            <div style={metricBox}>
              <div style={{ fontSize: 24, fontWeight: 700 }}>{status.sessionsAnalyzed}</div>
              <div className="panel-label">Sessions Analyzed</div>
            </div>
            <div style={metricBox}>
              <div style={{ fontSize: 24, fontWeight: 700 }}>{status.patternsExtracted}</div>
              <div className="panel-label">Patterns Found</div>
            </div>
            <div style={metricBox}>
              <div style={{ fontSize: 24, fontWeight: 700 }}>{status.skillsGenerated}</div>
              <div className="panel-label">Skills Generated</div>
            </div>
          </div>
          <div className="panel-card">
            <div className="panel-label">Improvement Estimate</div>
            <div style={{ fontSize: 18, fontWeight: 600, color: status.improvementEstimate > 0 ? "var(--success-color)" : "var(--text-secondary)" }}>
              {status.improvementEstimate > 0 ? `+${(status.improvementEstimate * 100).toFixed(1)}%` : "Not enough data"}
            </div>
            <div className="panel-label" style={{ marginTop: 4 }}>
              Estimated coding speed improvement from learned patterns.
            </div>
          </div>
        </>
      )}

      {tab === "patterns" && (
        <>
          {patterns.length === 0 && !loading && (
            <div className="panel-empty">No patterns learned yet. Patterns emerge after 3+ coding sessions.</div>
          )}
          {patterns.map(p => (
            <div key={p.id || p.rule} className="panel-card">
              <div style={{ display: "flex", alignItems: "center", marginBottom: 4 }}>
                <span style={{ fontWeight: 600 }}>{p.rule}</span>
                <span style={badgeStyle(confidenceColor(p.confidence))}>{p.confidence}</span>
                <span style={{ ...badgeStyle("var(--bg-tertiary)"), color: "var(--text-secondary)" }}>{p.type}</span>
              </div>
              <div>{p.description}</div>
              <div className="panel-label" style={{ marginTop: 4 }}>Seen {p.occurrences}x</div>
            </div>
          ))}
        </>
      )}

      {tab === "export" && (
        <div className="panel-card">
          <div className="panel-label">Exported Skills (Markdown)</div>
          {exported ? (
            <pre style={{ whiteSpace: "pre-wrap", fontSize: 11, fontFamily: "monospace", margin: 0, maxHeight: 400, overflow: "auto" }}>{exported}</pre>
          ) : (
            <div className="panel-label">{loading ? "Exporting..." : "No skills to export yet."}</div>
          )}
        </div>
      )}
    </div>
  );
}
