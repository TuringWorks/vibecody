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

const badgeStyle = (color: string): React.CSSProperties => ({
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: "var(--radius-md)",
  fontSize: "var(--font-size-sm)",
  fontWeight: 600,
  background: color,
  color: "var(--btn-primary-fg, #fff)",
  marginRight: 4,
});

const confColor = (c: number) => c >= 80 ? "var(--success-color)" : c >= 50 ? "var(--warning-color)" : "var(--text-secondary)";

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
      <div className="panel-container">
        <h2 style={{ fontSize: 18, fontWeight: 600, marginBottom: 12, color: "var(--text-primary)" }}>Next-Task Prediction</h2>
        <div className="panel-loading">Loading...</div>
      </div>
    );
  }

  return (
    <div className="panel-container">
      <h2 style={{ fontSize: 18, fontWeight: 600, marginBottom: 12, color: "var(--text-primary)" }}>Next-Task Prediction</h2>
      <div className="panel-tab-bar" style={{ marginBottom: 16 }}>
        <button className={`panel-tab ${tab === "suggestions" ? "active" : ""}`} onClick={() => setTab("suggestions")}>Suggestions</button>
        <button className={`panel-tab ${tab === "history" ? "active" : ""}`} onClick={() => setTab("history")}>History</button>
        <button className={`panel-tab ${tab === "learning" ? "active" : ""}`} onClick={() => setTab("learning")}>Learning</button>
        <button className={`panel-tab ${tab === "config" ? "active" : ""}`} onClick={() => setTab("config")}>Config</button>
      </div>

      {tab === "suggestions" && (
        <div>
          {predictions.map((p) => (
            <div key={p.id} className="panel-card">
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                <strong>{p.action}</strong>
                <span style={badgeStyle(confColor(p.confidence))}>{p.confidence}%</span>
              </div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 6 }}>{p.description}</div>
              <div style={{ background: "var(--bg-primary)", borderRadius: "var(--radius-xs-plus)", height: 6, marginBottom: 8 }}>
                <div style={{ background: confColor(p.confidence), borderRadius: "var(--radius-xs-plus)", height: 6, width: `${p.confidence}%` }} />
              </div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Source: {p.source}</span>
                <div>
                  <button className="panel-btn panel-btn-primary" onClick={() => handleAccept(p.id)}>Accept</button>
                  <button className="panel-btn panel-btn-secondary" onClick={() => handleDismiss(p.id)}>Dismiss</button>
                </div>
              </div>
            </div>
          ))}
          {predictions.length === 0 && <div className="panel-empty">No predictions available</div>}
        </div>
      )}

      {tab === "history" && (
        <div>
          {history.length === 0 && <div className="panel-empty">No action history recorded yet.</div>}
          {history.map((h) => (
            <div key={h.id} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong style={{ fontSize: "var(--font-size-md)" }}>{h.action}</strong>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{h.timestamp}</div>
              </div>
              {h.predicted && <span style={badgeStyle("var(--success-color)")}>predicted</span>}
              {!h.predicted && <span style={badgeStyle("var(--text-secondary)")}>unpredicted</span>}
            </div>
          ))}
        </div>
      )}

      {tab === "learning" && (
        <div>
          <div className="panel-card">
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>Prediction Accuracy</div>
            <div style={{ fontSize: 24, fontWeight: 700 }}>{accuracy}%</div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{predictedCorrect} / {history.length} actions correctly predicted</div>
          </div>
          <div className="panel-card">
            <div style={{ fontWeight: 600, marginBottom: 8 }}>Transition Matrix</div>
            {transitions.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>No transitions recorded yet.</div>}
            {transitions.length > 0 && (
              <table style={{ width: "100%", fontSize: "var(--font-size-base)", borderCollapse: "collapse" }}>
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
          {rules.length === 0 && <div className="panel-empty">No learning rules configured.</div>}
          {rules.map((r) => (
            <div key={r.id} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <strong style={{ fontSize: "var(--font-size-md)" }}>{r.name}</strong>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>{r.description}</div>
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
