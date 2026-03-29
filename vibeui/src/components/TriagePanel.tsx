import { useState, useCallback } from "react";

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
  const [issues] = useState<TriageIssue[]>([
    { id: "i1", title: "Login page crashes on Safari", classification: "bug", severity: "critical", suggestedLabels: ["bug", "browser-compat", "P0"], draftResponse: "Thanks for reporting. We can reproduce this on Safari 17+. Working on a fix.", confidence: 92 },
    { id: "i2", title: "Add dark mode toggle", classification: "feature", severity: "medium", suggestedLabels: ["enhancement", "ui"], draftResponse: "Thanks for the suggestion! Adding to our backlog.", confidence: 87 },
    { id: "i3", title: "Docs typo in API reference", classification: "docs", severity: "low", suggestedLabels: ["documentation", "good-first-issue"], draftResponse: "Good catch! This is a great first contribution if you'd like to open a PR.", confidence: 95 },
  ]);
  const [rules, setRules] = useState<TriageRule[]>([
    { id: "r1", name: "Crash reports", pattern: "crash|segfault|panic", action: "Label: P0, bug", enabled: true },
    { id: "r2", name: "Feature requests", pattern: "feature|request|add support", action: "Label: enhancement", enabled: true },
    { id: "r3", name: "Docs issues", pattern: "typo|docs|documentation", action: "Label: documentation", enabled: false },
  ]);
  const [history] = useState<TriageResult[]>([
    { id: "h1", issueTitle: "Memory leak in worker", classification: "bug", correct: true, triageAt: "2026-03-25" },
    { id: "h2", issueTitle: "Support ARM builds", classification: "feature", correct: true, triageAt: "2026-03-25" },
    { id: "h3", issueTitle: "Improve startup time", classification: "bug", correct: false, triageAt: "2026-03-24" },
  ]);
  const [newRuleName, setNewRuleName] = useState("");
  const [newRulePattern, setNewRulePattern] = useState("");

  const toggleRule = useCallback((id: string) => {
    setRules((prev) => prev.map((r) => r.id === id ? { ...r, enabled: !r.enabled } : r));
  }, []);

  const addRule = useCallback(() => {
    if (!newRuleName || !newRulePattern) return;
    setRules((prev) => [...prev, { id: `r${Date.now()}`, name: newRuleName, pattern: newRulePattern, action: "Label: triage", enabled: true }]);
    setNewRuleName("");
    setNewRulePattern("");
  }, [newRuleName, newRulePattern]);

  const accuracy = history.length > 0 ? ((history.filter((h) => h.correct).length / history.length) * 100).toFixed(0) : "0";
  const avgConf = issues.length > 0 ? (issues.reduce((s, i) => s + i.confidence, 0) / issues.length).toFixed(0) : "0";
  const inputStyle: React.CSSProperties = { width: "100%", padding: 8, borderRadius: 6, border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: 13, marginBottom: 8 };

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Issue Triage</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        <button style={tabStyle(tab === "queue")} onClick={() => setTab("queue")}>Queue</button>
        <button style={tabStyle(tab === "rules")} onClick={() => setTab("rules")}>Rules</button>
        <button style={tabStyle(tab === "history")} onClick={() => setTab("history")}>History</button>
        <button style={tabStyle(tab === "metrics")} onClick={() => setTab("metrics")}>Metrics</button>
      </div>

      {tab === "queue" && (
        <div>
          {issues.map((i) => (
            <div key={i.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                <strong>{i.title}</strong>
                <div>
                  <span style={badgeStyle("#6366f1")}>{i.classification}</span>
                  <span style={badgeStyle(sevColor[i.severity])}>{i.severity}</span>
                </div>
              </div>
              <div style={{ marginBottom: 6 }}>{i.suggestedLabels.map((l) => <span key={l} style={badgeStyle("#374151")}>{l}</span>)}</div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", fontStyle: "italic", padding: 8, background: "var(--bg-primary)", borderRadius: 4 }}>{i.draftResponse}</div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 4 }}>Confidence: {i.confidence}%</div>
            </div>
          ))}
        </div>
      )}

      {tab === "rules" && (
        <div>
          {rules.map((r) => (
            <div key={r.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong>{r.name}</strong>
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>/{r.pattern}/ &rarr; {r.action}</div>
              </div>
              <label style={{ cursor: "pointer" }}><input type="checkbox" checked={r.enabled} onChange={() => toggleRule(r.id)} /></label>
            </div>
          ))}
          <div style={{ ...cardStyle, marginTop: 12 }}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Add Rule</div>
            <input placeholder="Rule name" style={inputStyle} value={newRuleName} onChange={(e) => setNewRuleName(e.target.value)} />
            <input placeholder="Pattern (regex)" style={inputStyle} value={newRulePattern} onChange={(e) => setNewRulePattern(e.target.value)} />
            <button style={btnStyle} onClick={addRule}>Add Rule</button>
          </div>
        </div>
      )}

      {tab === "history" && (
        <div>
          {history.map((h) => (
            <div key={h.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
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
          <div style={cardStyle}><div style={{ fontSize: 12, color: "var(--text-secondary)" }}>By Type</div>{["bug", "feature", "docs"].map((t) => <div key={t} style={{ display: "flex", justifyContent: "space-between", fontSize: 13, padding: "2px 0" }}><span>{t}</span><strong>{issues.filter((i) => i.classification === t).length}</strong></div>)}</div>
          <div style={cardStyle}><div style={{ fontSize: 12, color: "var(--text-secondary)" }}>By Severity</div>{["critical", "high", "medium", "low"].map((s) => <div key={s} style={{ display: "flex", justifyContent: "space-between", fontSize: 13, padding: "2px 0" }}><span>{s}</span><strong>{issues.filter((i) => i.severity === s).length}</strong></div>)}</div>
          <div style={cardStyle}><div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Accuracy</div><div style={{ fontSize: 24, fontWeight: 700 }}>{accuracy}%</div></div>
          <div style={cardStyle}><div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Avg Confidence</div><div style={{ fontSize: 24, fontWeight: 700 }}>{avgConf}%</div></div>
        </div>
      )}
    </div>
  );
}
