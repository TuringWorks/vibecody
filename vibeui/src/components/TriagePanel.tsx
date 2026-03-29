import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface TriageIssue {
  id: string;
  title: string;
  classification: string;
  severity: "critical" | "high" | "medium" | "low";
  suggestedLabels: string[];
  draftResponse: string;
  confidence: number;
}

interface TriageRule {
  id: string;
  name: string;
  pattern: string;
  action: string;
  enabled: boolean;
}

interface TriageResult {
  id: string;
  issueTitle: string;
  classification: string;
  correct: boolean;
  triageAt: string;
}

const panelStyle: React.CSSProperties = {
  padding: 16,
  height: "100%",
  overflow: "auto",
  color: "var(--text-primary)",
  background: "var(--bg-primary)",
};

const headingStyle: React.CSSProperties = {
  fontSize: 18,
  fontWeight: 600,
  marginBottom: 12,
  color: "var(--text-primary)",
};

const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  borderRadius: 8,
  padding: 12,
  marginBottom: 8,
  border: "1px solid var(--border-color)",
};

const btnStyle: React.CSSProperties = {
  padding: "6px 14px",
  borderRadius: 6,
  border: "1px solid var(--border-color)",
  background: "var(--accent-color)",
  color: "var(--btn-primary-fg, #fff)",
  cursor: "pointer",
  fontSize: 13,
  marginRight: 8,
};

const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 16px",
  cursor: "pointer",
  borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
  color: active ? "var(--accent-color)" : "var(--text-secondary)",
  background: "transparent",
  border: "none",
  fontSize: 13,
  fontWeight: active ? 600 : 400,
});

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: 10,
  fontSize: 11,
  fontWeight: 600,
  background: color,
  color: "var(--btn-primary-fg, #fff)",
  marginRight: 4,
});

const sevColor: Record<string, string> = { critical: "var(--error-color)", high: "var(--error-color)", medium: "var(--warning-color)", low: "var(--success-color)" };

export function TriagePanel() {
  const [tab, setTab] = useState("queue");
  const [issues, setIssues] = useState<TriageIssue[]>([]);
  const [rules, setRules] = useState<TriageRule[]>([]);
  const [history, setHistory] = useState<TriageResult[]>([]);
  const [metrics, setMetrics] = useState<Record<string, unknown> | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [newTitle, setNewTitle] = useState("");
  const [newBody, setNewBody] = useState("");

  const fetchData = useCallback(async () => {
    try {
      const [rulesData, historyData, metricsData] = await Promise.all([
        invoke<unknown>("triage_get_rules"),
        invoke<unknown>("triage_get_history"),
        invoke<unknown>("triage_get_metrics"),
      ]);
      setRules(Array.isArray(rulesData) ? rulesData as TriageRule[] : []);
      const histList = Array.isArray(historyData) ? historyData : (historyData as any)?.history ?? [];
      setHistory(histList);
      setMetrics(metricsData as Record<string, unknown>);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    setLoading(true);
    setError(null);
    fetchData().finally(() => setLoading(false));
  }, [fetchData]);

  const handleTriage = useCallback(async () => {
    if (!newTitle.trim()) return;
    try {
      const result = await invoke<TriageIssue>("triage_issue", { title: newTitle, body: newBody });
      if (result) {
        setIssues((prev) => [result, ...prev]);
      }
      setNewTitle("");
      setNewBody("");
      await fetchData();
    } catch (e) {
      console.error("triage_issue failed:", e);
    }
  }, [newTitle, newBody, fetchData]);

  const inputStyle: React.CSSProperties = { width: "100%", padding: 8, borderRadius: 6, border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: 13, marginBottom: 8 };

  const allIssues = issues;
  const accuracy = history.length > 0 ? ((history.filter((h) => h.correct).length / history.length) * 100).toFixed(0) : "0";
  const avgConf = allIssues.length > 0 ? (allIssues.reduce((s, i) => s + i.confidence, 0) / allIssues.length).toFixed(0) : "0";

  if (loading) return <div style={panelStyle}><div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Loading triage data...</div></div>;
  if (error) return <div style={panelStyle}><div style={{ color: "var(--error-color)", fontSize: 13 }}>Error: {error}</div></div>;

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Issue Triage</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        <button style={tabStyle(tab === "queue")} onClick={() => setTab("queue")}>Queue</button>
        <button style={tabStyle(tab === "submit")} onClick={() => setTab("submit")}>Submit</button>
        <button style={tabStyle(tab === "rules")} onClick={() => setTab("rules")}>Rules</button>
        <button style={tabStyle(tab === "history")} onClick={() => setTab("history")}>History</button>
        <button style={tabStyle(tab === "metrics")} onClick={() => setTab("metrics")}>Metrics</button>
      </div>

      {tab === "queue" && (
        <div>
          {allIssues.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No issues triaged yet. Submit an issue to get started.</div>}
          {allIssues.map((i) => (
            <div key={i.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                <strong>{i.title}</strong>
                <div>
                  <span style={badgeStyle("#6366f1")}>{i.classification}</span>
                  <span style={badgeStyle(sevColor[i.severity])}>{i.severity}</span>
                </div>
              </div>
              <div style={{ marginBottom: 6 }}>{(i.suggestedLabels || []).map((l) => <span key={l} style={badgeStyle("#374151")}>{l}</span>)}</div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", fontStyle: "italic", padding: 8, background: "var(--bg-primary)", borderRadius: 4 }}>{i.draftResponse}</div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 4 }}>Confidence: {i.confidence}%</div>
            </div>
          ))}
        </div>
      )}

      {tab === "submit" && (
        <div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Submit Issue for Triage</div>
            <input placeholder="Issue title" style={inputStyle} value={newTitle} onChange={(e) => setNewTitle(e.target.value)} />
            <textarea placeholder="Issue body / description" style={{ ...inputStyle, height: 80, resize: "vertical" as const }} value={newBody} onChange={(e) => setNewBody(e.target.value)} />
            <button style={btnStyle} onClick={handleTriage} disabled={!newTitle.trim()}>Triage</button>
          </div>
        </div>
      )}

      {tab === "rules" && (
        <div>
          {rules.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No triage rules configured.</div>}
          {rules.map((r, idx) => (
            <div key={r.id || idx} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong>{r.name || r.pattern}</strong>
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>/{r.pattern}/ &rarr; {r.action || `Label: ${r.name}`}</div>
              </div>
              {r.enabled !== undefined && (
                <label style={{ cursor: "pointer" }}><input type="checkbox" checked={r.enabled} readOnly /></label>
              )}
            </div>
          ))}
        </div>
      )}

      {tab === "history" && (
        <div>
          {history.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No triage history yet.</div>}
          {history.map((h, idx) => (
            <div key={h.id || idx} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong>{h.issueTitle}</strong>
                <span style={{ ...badgeStyle("#6366f1"), marginLeft: 8 }}>{h.classification}</span>
              </div>
              <span style={badgeStyle(h.correct ? "var(--success-color)" : "var(--error-color)")}>{h.correct ? "correct" : "wrong"}</span>
            </div>
          ))}
        </div>
      )}

      {tab === "metrics" && (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
          <div style={cardStyle}><div style={{ fontSize: 12, color: "var(--text-secondary)" }}>By Type</div>{["bug", "feature", "docs"].map((t) => <div key={t} style={{ display: "flex", justifyContent: "space-between", fontSize: 13, padding: "2px 0" }}><span>{t}</span><strong>{allIssues.filter((i) => i.classification === t).length}</strong></div>)}</div>
          <div style={cardStyle}><div style={{ fontSize: 12, color: "var(--text-secondary)" }}>By Severity</div>{["critical", "high", "medium", "low"].map((s) => <div key={s} style={{ display: "flex", justifyContent: "space-between", fontSize: 13, padding: "2px 0" }}><span>{s}</span><strong>{allIssues.filter((i) => i.severity === s).length}</strong></div>)}</div>
          <div style={cardStyle}><div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Accuracy</div><div style={{ fontSize: 24, fontWeight: 700 }}>{accuracy}%</div></div>
          <div style={cardStyle}><div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Avg Confidence</div><div style={{ fontSize: 24, fontWeight: 700 }}>{avgConf}%</div></div>
          {metrics && (
            <div style={{ ...cardStyle, gridColumn: "1 / -1" }}>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>Backend Metrics</div>
              <pre style={{ fontSize: 12, color: "var(--text-secondary)", whiteSpace: "pre-wrap" }}>{JSON.stringify(metrics, null, 2)}</pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
