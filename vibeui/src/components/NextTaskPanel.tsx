import { useState, useCallback } from "react";

interface PredictedTask {
  id: string;
  action: string;
  description: string;
  confidence: number;
  source: string;
}

interface ActionRecord {
  id: string;
  action: string;
  timestamp: string;
  predicted: boolean;
}

interface TransitionEntry {
  from: string;
  to: string;
  count: number;
  probability: number;
}

interface LearningRule {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
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
  color: "#fff",
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
  color: "#fff",
  marginRight: 4,
});

const confColor = (c: number) => c >= 80 ? "#22c55e" : c >= 50 ? "#f59e0b" : "#6b7280";

export function NextTaskPanel() {
  const [tab, setTab] = useState("suggestions");
  const [predictions, setPredictions] = useState<PredictedTask[]>([
    { id: "p1", action: "Run tests", description: "You edited 3 source files since last test run", confidence: 92, source: "edit-pattern" },
    { id: "p2", action: "Commit changes", description: "5 staged files, all tests passing", confidence: 85, source: "git-state" },
    { id: "p3", action: "Review PR #38", description: "Assigned 2 hours ago, reviewer requested", confidence: 68, source: "github-events" },
    { id: "p4", action: "Update docs", description: "API endpoint added without doc update", confidence: 55, source: "diff-analysis" },
  ]);
  const [history] = useState<ActionRecord[]>([
    { id: "a1", action: "Edit src/auth.rs", timestamp: "10:15", predicted: false },
    { id: "a2", action: "Run cargo test", timestamp: "10:18", predicted: true },
    { id: "a3", action: "Edit src/config.rs", timestamp: "10:22", predicted: false },
    { id: "a4", action: "Git commit", timestamp: "10:25", predicted: true },
    { id: "a5", action: "Push to remote", timestamp: "10:26", predicted: true },
  ]);
  const [transitions] = useState<TransitionEntry[]>([
    { from: "edit", to: "test", count: 42, probability: 0.65 },
    { from: "test", to: "commit", count: 35, probability: 0.58 },
    { from: "commit", to: "push", count: 30, probability: 0.82 },
    { from: "edit", to: "edit", count: 28, probability: 0.20 },
    { from: "push", to: "edit", count: 25, probability: 0.71 },
    { from: "test", to: "edit", count: 20, probability: 0.33 },
  ]);
  const [rules, setRules] = useState<LearningRule[]>([
    { id: "r1", name: "Edit-then-test pattern", description: "Suggest test run after source edits", enabled: true },
    { id: "r2", name: "Stage-then-commit", description: "Suggest commit when files are staged and tests pass", enabled: true },
    { id: "r3", name: "PR review reminders", description: "Suggest reviewing assigned PRs", enabled: true },
    { id: "r4", name: "Doc freshness", description: "Suggest doc updates when API changes detected", enabled: false },
    { id: "r5", name: "Dependency updates", description: "Suggest updating outdated dependencies", enabled: false },
  ]);

  const handleDismiss = useCallback((id: string) => {
    setPredictions((prev) => prev.filter((p) => p.id !== id));
  }, []);

  const toggleRule = useCallback((id: string) => {
    setRules((prev) => prev.map((r) => r.id === id ? { ...r, enabled: !r.enabled } : r));
  }, []);

  const predictedCorrect = history.filter((h) => h.predicted).length;
  const accuracy = history.length > 0 ? ((predictedCorrect / history.length) * 100).toFixed(0) : "0";

  const heatMaxCount = Math.max(...transitions.map((t) => t.count));

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Next-Task Prediction</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        <button style={tabStyle(tab === "suggestions")} onClick={() => setTab("suggestions")}>Suggestions</button>
        <button style={tabStyle(tab === "history")} onClick={() => setTab("history")}>History</button>
        <button style={tabStyle(tab === "learning")} onClick={() => setTab("learning")}>Learning</button>
        <button style={tabStyle(tab === "config")} onClick={() => setTab("config")}>Config</button>
      </div>

      {tab === "suggestions" && (
        <div>
          {predictions.map((p) => (
            <div key={p.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                <strong>{p.action}</strong>
                <span style={badgeStyle(confColor(p.confidence))}>{p.confidence}%</span>
              </div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 6 }}>{p.description}</div>
              <div style={{ background: "var(--bg-primary)", borderRadius: 4, height: 6, marginBottom: 8 }}>
                <div style={{ background: confColor(p.confidence), borderRadius: 4, height: 6, width: `${p.confidence}%` }} />
              </div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>Source: {p.source}</span>
                <div>
                  <button style={btnStyle} onClick={() => handleDismiss(p.id)}>Accept</button>
                  <button style={{ ...btnStyle, background: "#6b7280" }} onClick={() => handleDismiss(p.id)}>Dismiss</button>
                </div>
              </div>
            </div>
          ))}
          {predictions.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No predictions available</div>}
        </div>
      )}

      {tab === "history" && (
        <div>
          {history.map((h) => (
            <div key={h.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong style={{ fontSize: 13 }}>{h.action}</strong>
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{h.timestamp}</div>
              </div>
              {h.predicted && <span style={badgeStyle("#22c55e")}>predicted</span>}
              {!h.predicted && <span style={badgeStyle("#6b7280")}>unpredicted</span>}
            </div>
          ))}
        </div>
      )}

      {tab === "learning" && (
        <div>
          <div style={cardStyle}>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Prediction Accuracy</div>
            <div style={{ fontSize: 24, fontWeight: 700 }}>{accuracy}%</div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{predictedCorrect} / {history.length} actions correctly predicted</div>
          </div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Transition Matrix</div>
            <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
              <thead>
                <tr style={{ borderBottom: "1px solid var(--border-color)" }}>
                  <th style={{ textAlign: "left", padding: 6 }}>From</th>
                  <th style={{ textAlign: "left", padding: 6 }}>To</th>
                  <th style={{ textAlign: "right", padding: 6 }}>Count</th>
                  <th style={{ textAlign: "right", padding: 6 }}>P</th>
                  <th style={{ textAlign: "left", padding: 6, width: 80 }}>Heat</th>
                </tr>
              </thead>
              <tbody>
                {transitions.map((t, i) => {
                  const intensity = Math.round((t.count / heatMaxCount) * 255);
                  return (
                    <tr key={i} style={{ borderBottom: "1px solid var(--border-color)" }}>
                      <td style={{ padding: 6 }}>{t.from}</td>
                      <td style={{ padding: 6 }}>{t.to}</td>
                      <td style={{ padding: 6, textAlign: "right" }}>{t.count}</td>
                      <td style={{ padding: 6, textAlign: "right" }}>{(t.probability * 100).toFixed(0)}%</td>
                      <td style={{ padding: 6 }}>
                        <div style={{ width: "100%", height: 14, borderRadius: 3, background: `rgba(99, 102, 241, ${intensity / 255})` }} />
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {tab === "config" && (
        <div>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>Enabled Rules</div>
          {rules.map((r) => (
            <div key={r.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong style={{ fontSize: 13 }}>{r.name}</strong>
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{r.description}</div>
              </div>
              <label style={{ cursor: "pointer" }}>
                <input type="checkbox" checked={r.enabled} onChange={() => toggleRule(r.id)} />
              </label>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
