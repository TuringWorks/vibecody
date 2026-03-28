import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

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
  const [predictions, setPredictions] = useState<PredictedTask[]>([]);
  const [history, setHistory] = useState<ActionRecord[]>([]);
  const [transitions, setTransitions] = useState<TransitionEntry[]>([]);
  const [rules, setRules] = useState<LearningRule[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    async function loadData() {
      setLoading(true);
      try {
        const [preds, hist, trans, rls] = await Promise.all([
          invoke<PredictedTask[]>("get_nexttask_predictions"),
          invoke<ActionRecord[]>("get_nexttask_history"),
          invoke<TransitionEntry[]>("get_nexttask_transitions"),
          invoke<LearningRule[]>("get_nexttask_rules"),
        ]);
        if (!cancelled) {
          setPredictions(preds);
          setHistory(hist);
          setTransitions(trans);
          setRules(rls);
        }
      } catch (err) {
        console.error("Failed to load next-task data:", err);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    loadData();
    return () => { cancelled = true; };
  }, []);

  const handleAccept = useCallback(async (id: string) => {
    try {
      await invoke("accept_nexttask", { id });
      setPredictions((prev) => prev.filter((p) => p.id !== id));
    } catch (err) {
      console.error("Failed to accept task:", err);
    }
  }, []);

  const handleDismiss = useCallback((id: string) => {
    setPredictions((prev) => prev.filter((p) => p.id !== id));
  }, []);

  const toggleRule = useCallback(async (id: string) => {
    try {
      await invoke("toggle_nexttask_rule", { id });
      setRules((prev) => prev.map((r) => r.id === id ? { ...r, enabled: !r.enabled } : r));
    } catch (err) {
      console.error("Failed to toggle rule:", err);
    }
  }, []);

  const predictedCorrect = history.filter((h) => h.predicted).length;
  const accuracy = history.length > 0 ? ((predictedCorrect / history.length) * 100).toFixed(0) : "0";

  const heatMaxCount = transitions.length > 0 ? Math.max(...transitions.map((t) => t.count)) : 1;

  if (loading) {
    return (
      <div style={panelStyle}>
        <h2 style={headingStyle}>Next-Task Prediction</h2>
        <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>
      </div>
    );
  }

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
                  <button style={btnStyle} onClick={() => handleAccept(p.id)}>Accept</button>
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
          {history.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No action history recorded yet.</div>}
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
            {transitions.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 12 }}>No transitions recorded yet.</div>}
            {transitions.length > 0 && (
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
            )}
          </div>
        </div>
      )}

      {tab === "config" && (
        <div>
          <div style={{ fontWeight: 600, marginBottom: 8 }}>Enabled Rules</div>
          {rules.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No learning rules configured.</div>}
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
