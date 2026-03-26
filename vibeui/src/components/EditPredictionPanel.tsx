/**
 * EditPredictionPanel — RL-trained Edit Prediction.
 *
 * Tabs: Predictions (recent predictions with confidence, accept/reject),
 * Patterns (detected edit patterns with frequency),
 * Model (Q-table stats, exploration rate, acceptance rate, decay).
 * Wired to Tauri backend commands.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

type Tab = "predictions" | "patterns" | "model";
type PredictionState = "pending" | "accepted" | "rejected";

interface Prediction {
  id: string;
  file: string;
  line: number;
  suggestion: string;
  confidence: number;
  state: PredictionState;
  timestamp: string;
  pattern: string;
}

interface EditPattern {
  id: string;
  name: string;
  description: string;
  frequency: number;
  lastSeen: string;
  avgConfidence: number;
  acceptRate: number;
}

interface ModelStats {
  qTableSize: number;
  totalStates: number;
  totalActions: number;
  explorationRate: number;
  learningRate: number;
  discountFactor: number;
  acceptanceRate: number;
  totalPredictions: number;
  accepted: number;
  rejected: number;
  decayRate: number;
}

const DEFAULT_MODEL: ModelStats = {
  qTableSize: 0, totalStates: 0, totalActions: 0,
  explorationRate: 0.15, learningRate: 0.01, discountFactor: 0.95,
  acceptanceRate: 0, totalPredictions: 0, accepted: 0, rejected: 0,
  decayRate: 0.999,
};

const tabBtn = (active: boolean): React.CSSProperties => ({
  padding: "6px 14px", fontSize: 11, fontWeight: active ? 600 : 400,
  background: active ? "var(--accent-bg, color-mix(in srgb, var(--accent-blue) 15%, transparent))" : "transparent",
  border: "1px solid " + (active ? "var(--accent-primary)" : "var(--border-color)"),
  borderRadius: 4, color: active ? "var(--text-info)" : "var(--text-secondary)", cursor: "pointer",
});

const confColor = (c: number) => c > 0.85 ? "var(--text-success)" : c > 0.7 ? "var(--text-warning)" : "var(--text-danger)";

export default function EditPredictionPanel() {
  const [tab, setTab] = useState<Tab>("predictions");
  const [predictions, setPredictions] = useState<Prediction[]>([]);
  const [patterns, setPatterns] = useState<EditPattern[]>([]);
  const [model, setModel] = useState<ModelStats>(DEFAULT_MODEL);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const [preds, pats, stats] = await Promise.all([
        invoke<Prediction[]>("get_edit_predictions"),
        invoke<EditPattern[]>("get_edit_patterns"),
        invoke<ModelStats>("get_edit_model_stats"),
      ]);
      setPredictions(preds);
      setPatterns(pats);
      setModel(stats);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  const handlePrediction = async (id: string, action: "accepted" | "rejected") => {
    // Optimistic update
    setPredictions(ps => ps.map(p => p.id === id ? { ...p, state: action } : p));
    try {
      if (action === "accepted") {
        await invoke("accept_prediction", { id });
      } else {
        await invoke("dismiss_prediction", { id });
      }
      // Refresh model stats after feedback
      const stats = await invoke<ModelStats>("get_edit_model_stats");
      setModel(stats);
    } catch (e) {
      // Revert on failure
      setPredictions(ps => ps.map(p => p.id === id ? { ...p, state: "pending" as PredictionState } : p));
      setError(String(e));
    }
  };

  const adjustExploration = (delta: number) => {
    setModel(m => ({ ...m, explorationRate: Math.max(0, Math.min(1, m.explorationRate + delta)) }));
  };

  const adjustDecay = (delta: number) => {
    setModel(m => ({ ...m, decayRate: Math.max(0.99, Math.min(1, m.decayRate + delta)) }));
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      <div style={{ display: "flex", gap: 6, padding: "8px 10px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" }}>
        {(["predictions", "patterns", "model"] as Tab[]).map(t => (
          <button key={t} onClick={() => setTab(t)} style={tabBtn(tab === t)}>
            {t[0].toUpperCase() + t.slice(1)}
          </button>
        ))}
        <span style={{ marginLeft: "auto", fontSize: 10, color: "var(--text-secondary)", alignSelf: "center" }}>
          {(model.acceptanceRate * 100).toFixed(0)}% accept rate
        </span>
      </div>

      <div style={{ flex: 1, overflowY: "auto", padding: 12, display: "flex", flexDirection: "column", gap: 10 }}>
        {loading && (
          <div style={{ textAlign: "center", padding: 20, color: "var(--text-secondary)", fontSize: 12 }}>Loading...</div>
        )}

        {error && (
          <div style={{ padding: 10, background: "rgba(239,68,68,0.1)", border: "1px solid var(--text-danger)", borderRadius: 6, fontSize: 11, color: "var(--text-danger)" }}>
            {error}
          </div>
        )}

        {tab === "predictions" && !loading && predictions.length === 0 && !error && (
          <div style={{ textAlign: "center", padding: 30, color: "var(--text-secondary)", fontSize: 12 }}>
            No predictions yet. Edit some files to generate predictions.
          </div>
        )}

        {tab === "predictions" && predictions.map(p => (
          <div key={p.id} style={{ padding: 10, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)", opacity: p.state !== "pending" ? 0.6 : 1 }}>
            <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 6 }}>
              <span style={{ fontSize: 11, fontFamily: "var(--font-mono)", color: "var(--text-info)" }}>{p.file}:{p.line}</span>
              <span style={{ fontSize: 9, padding: "1px 5px", borderRadius: 3, background: "color-mix(in srgb, var(--accent-blue) 12%, transparent)", color: "var(--text-secondary)" }}>{p.pattern}</span>
              <span style={{ fontSize: 10, color: "var(--text-secondary)", marginLeft: "auto" }}>{p.timestamp}</span>
            </div>
            <div style={{ fontSize: 11, color: "var(--text-primary)", marginBottom: 8 }}>{p.suggestion}</div>
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <div style={{ flex: 1, height: 6, background: "var(--bg-primary)", borderRadius: 3, overflow: "hidden" }}>
                <div style={{ width: `${p.confidence * 100}%`, height: "100%", background: confColor(p.confidence), borderRadius: 3 }} />
              </div>
              <span style={{ fontSize: 10, color: confColor(p.confidence), minWidth: 34, fontWeight: 600 }}>{(p.confidence * 100).toFixed(0)}%</span>
              {p.state === "pending" ? (
                <>
                  <button onClick={() => handlePrediction(p.id, "rejected")}
                    style={{ padding: "3px 10px", fontSize: 10, borderRadius: 3, border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-danger)", cursor: "pointer" }}>Reject</button>
                  <button onClick={() => handlePrediction(p.id, "accepted")}
                    style={{ padding: "3px 10px", fontSize: 10, borderRadius: 3, border: "none", background: "var(--text-success)", color: "var(--bg-primary)", cursor: "pointer", fontWeight: 600 }}>Accept</button>
                </>
              ) : (
                <span style={{ fontSize: 10, color: p.state === "accepted" ? "var(--text-success)" : "var(--text-danger)" }}>{p.state}</span>
              )}
            </div>
          </div>
        ))}

        {tab === "patterns" && !loading && patterns.length === 0 && !error && (
          <div style={{ textAlign: "center", padding: 30, color: "var(--text-secondary)", fontSize: 12 }}>
            No patterns detected yet. Patterns emerge as you edit files.
          </div>
        )}

        {tab === "patterns" && patterns.map(p => (
          <div key={p.id} style={{ padding: 10, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
            <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 4 }}>
              <span style={{ fontSize: 11, fontWeight: 600, fontFamily: "var(--font-mono)", color: "var(--accent-primary)" }}>{p.name}</span>
              <span style={{ fontSize: 10, color: "var(--text-secondary)", marginLeft: "auto" }}>seen {p.frequency}x</span>
            </div>
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 8 }}>{p.description}</div>
            <div style={{ display: "flex", gap: 16, fontSize: 10 }}>
              <span style={{ color: "var(--text-secondary)" }}>Confidence: <span style={{ color: confColor(p.avgConfidence), fontWeight: 600 }}>{(p.avgConfidence * 100).toFixed(0)}%</span></span>
              <span style={{ color: "var(--text-secondary)" }}>Accept: <span style={{ color: confColor(p.acceptRate), fontWeight: 600 }}>{(p.acceptRate * 100).toFixed(0)}%</span></span>
              <span style={{ color: "var(--text-secondary)" }}>Last: {p.lastSeen}</span>
            </div>
            <div style={{ marginTop: 6, height: 3, background: "var(--bg-primary)", borderRadius: 2, overflow: "hidden" }}>
              <div style={{ width: `${p.acceptRate * 100}%`, height: "100%", background: confColor(p.acceptRate), borderRadius: 2 }} />
            </div>
          </div>
        ))}

        {tab === "model" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <div style={{ padding: 14, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
              <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 10 }}>Q-Table Statistics</div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10 }}>
                {[
                  ["Entries", model.qTableSize.toLocaleString()],
                  ["States", model.totalStates.toString()],
                  ["Actions", model.totalActions.toString()],
                ].map(([l, v]) => (
                  <div key={l} style={{ textAlign: "center", padding: 10, background: "var(--bg-primary)", borderRadius: 4 }}>
                    <div style={{ fontSize: 18, fontWeight: 700, color: "var(--text-info)", fontFamily: "var(--font-mono)" }}>{v}</div>
                    <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 4 }}>{l}</div>
                  </div>
                ))}
              </div>
            </div>

            <div style={{ padding: 14, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
              <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 10 }}>Performance</div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10 }}>
                {[
                  ["Total", model.totalPredictions.toLocaleString(), "var(--text-primary)"],
                  ["Accepted", model.accepted.toLocaleString(), "var(--text-success)"],
                  ["Rejected", model.rejected.toLocaleString(), "var(--text-danger)"],
                ].map(([l, v, c]) => (
                  <div key={l} style={{ textAlign: "center", padding: 10, background: "var(--bg-primary)", borderRadius: 4 }}>
                    <div style={{ fontSize: 18, fontWeight: 700, color: c, fontFamily: "var(--font-mono)" }}>{v}</div>
                    <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 4 }}>{l}</div>
                  </div>
                ))}
              </div>
              <div style={{ marginTop: 10, height: 6, background: "var(--bg-primary)", borderRadius: 3, overflow: "hidden" }}>
                <div style={{ width: `${model.acceptanceRate * 100}%`, height: "100%", background: "var(--text-success)", borderRadius: 3 }} />
              </div>
              <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 4, textAlign: "center" }}>
                Acceptance Rate: {(model.acceptanceRate * 100).toFixed(1)}%
              </div>
            </div>

            <div style={{ padding: 14, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
              <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 10 }}>Hyperparameters</div>
              <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                  <span style={{ fontSize: 11, color: "var(--text-secondary)", minWidth: 120 }}>Exploration Rate</span>
                  <button onClick={() => adjustExploration(-0.01)} style={{ padding: "2px 8px", fontSize: 10, border: "1px solid var(--border-color)", borderRadius: 3, background: "var(--bg-primary)", color: "var(--text-secondary)", cursor: "pointer" }}>-</button>
                  <span style={{ fontSize: 12, fontFamily: "var(--font-mono)", fontWeight: 600, color: "var(--text-info)", minWidth: 40, textAlign: "center" }}>{model.explorationRate.toFixed(2)}</span>
                  <button onClick={() => adjustExploration(0.01)} style={{ padding: "2px 8px", fontSize: 10, border: "1px solid var(--border-color)", borderRadius: 3, background: "var(--bg-primary)", color: "var(--text-secondary)", cursor: "pointer" }}>+</button>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                  <span style={{ fontSize: 11, color: "var(--text-secondary)", minWidth: 120 }}>Decay Rate</span>
                  <button onClick={() => adjustDecay(-0.001)} style={{ padding: "2px 8px", fontSize: 10, border: "1px solid var(--border-color)", borderRadius: 3, background: "var(--bg-primary)", color: "var(--text-secondary)", cursor: "pointer" }}>-</button>
                  <span style={{ fontSize: 12, fontFamily: "var(--font-mono)", fontWeight: 600, color: "var(--text-info)", minWidth: 40, textAlign: "center" }}>{model.decayRate.toFixed(3)}</span>
                  <button onClick={() => adjustDecay(0.001)} style={{ padding: "2px 8px", fontSize: 10, border: "1px solid var(--border-color)", borderRadius: 3, background: "var(--bg-primary)", color: "var(--text-secondary)", cursor: "pointer" }}>+</button>
                </div>
                <div style={{ display: "flex", gap: 16, fontSize: 10, color: "var(--text-secondary)", marginTop: 4 }}>
                  <span>Learning Rate: {model.learningRate}</span>
                  <span>Discount Factor: {model.discountFactor}</span>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
