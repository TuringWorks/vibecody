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
import { X, Loader2, RefreshCw } from "lucide-react";

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

const confColor = (c: number) =>
  c > 0.85 ? "var(--success-color)" : c > 0.7 ? "var(--warning-color)" : "var(--error-color)";

const confTag = (c: number) =>
  c > 0.85 ? "panel-tag panel-tag-success" : c > 0.7 ? "panel-tag panel-tag-warning" : "panel-tag panel-tag-danger";

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
    setPredictions(ps => ps.map(p => p.id === id ? { ...p, state: action } : p));
    try {
      await invoke(action === "accepted" ? "accept_prediction" : "dismiss_prediction", { id });
      const stats = await invoke<ModelStats>("get_edit_model_stats");
      setModel(stats);
    } catch (e) {
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
    <div className="panel-container">
      <div className="panel-header">
        <div className="panel-tab-bar" style={{ flex: 1, border: "none" }}>
          {(["predictions", "patterns", "model"] as Tab[]).map(t => (
            <button
              key={t}
              className={`panel-tab ${tab === t ? "active" : ""}`}
              onClick={() => setTab(t)}
            >
              {t[0].toUpperCase() + t.slice(1)}
            </button>
          ))}
        </div>
        <span className="panel-tag panel-tag-neutral" style={{ marginLeft: 8 }}>
          {(model.acceptanceRate * 100).toFixed(0)}% accept rate
        </span>
        <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={loadData} disabled={loading}>
          {loading ? <Loader2 size={13} className="spin" /> : <RefreshCw size={13} />}
        </button>
      </div>

      <div className="panel-body">
        {loading && <div className="panel-loading">Loading predictions…</div>}

        {error && (
          <div className="panel-error">
            {error}
            <button onClick={() => setError(null)}><X size={12} /></button>
          </div>
        )}

        {/* Predictions tab */}
        {tab === "predictions" && !loading && predictions.length === 0 && !error && (
          <div className="panel-empty">No predictions yet. Edit some files to generate predictions.</div>
        )}

        {tab === "predictions" && predictions.map(p => (
          <div key={p.id} className="panel-card" style={{ marginBottom: 8, opacity: p.state !== "pending" ? 0.6 : 1 }}>
            <div className="panel-row" style={{ marginBottom: 6 }}>
              <span className="panel-mono" style={{ fontSize: "var(--font-size-sm)", color: "var(--text-info)" }}>
                {p.file}:{p.line}
              </span>
              <span className="panel-tag panel-tag-neutral">{p.pattern}</span>
              <span style={{ marginLeft: "auto", fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
                {p.timestamp}
              </span>
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-primary)", marginBottom: 8 }}>
              {p.suggestion}
            </div>
            <div className="panel-row">
              <div className="progress-bar" style={{ flex: 1 }}>
                <div
                  className="progress-bar-fill"
                  style={{ width: `${p.confidence * 100}%`, background: confColor(p.confidence) }}
                />
              </div>
              <span style={{ fontSize: "var(--font-size-xs)", color: confColor(p.confidence), fontWeight: "var(--font-semibold)", minWidth: 32 }}>
                {(p.confidence * 100).toFixed(0)}%
              </span>
              {p.state === "pending" ? (
                <>
                  <button
                    className="panel-btn panel-btn-xs panel-btn-secondary"
                    style={{ color: "var(--text-danger)", borderColor: "var(--error-color)" }}
                    onClick={() => handlePrediction(p.id, "rejected")}
                  >
                    Reject
                  </button>
                  <button
                    className="panel-btn panel-btn-xs panel-btn-primary"
                    onClick={() => handlePrediction(p.id, "accepted")}
                  >
                    Accept
                  </button>
                </>
              ) : (
                <span className={p.state === "accepted" ? "panel-tag panel-tag-success" : "panel-tag panel-tag-danger"}>
                  {p.state}
                </span>
              )}
            </div>
          </div>
        ))}

        {/* Patterns tab */}
        {tab === "patterns" && !loading && patterns.length === 0 && !error && (
          <div className="panel-empty">No patterns detected yet. Patterns emerge as you edit files.</div>
        )}

        {tab === "patterns" && patterns.map(p => (
          <div key={p.id} className="panel-card" style={{ marginBottom: 8 }}>
            <div className="panel-row" style={{ marginBottom: 4 }}>
              <span className="panel-mono" style={{ fontSize: "var(--font-size-sm)", fontWeight: "var(--font-semibold)", color: "var(--accent-color)" }}>
                {p.name}
              </span>
              <span style={{ marginLeft: "auto", fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
                seen {p.frequency}×
              </span>
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 8 }}>
              {p.description}
            </div>
            <div className="panel-row" style={{ marginBottom: 6 }}>
              <span className={confTag(p.avgConfidence)}>{(p.avgConfidence * 100).toFixed(0)}% conf</span>
              <span className={confTag(p.acceptRate)}>{(p.acceptRate * 100).toFixed(0)}% accept</span>
              <span style={{ marginLeft: "auto", fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
                {p.lastSeen}
              </span>
            </div>
            <div className="progress-bar progress-bar-sm">
              <div className="progress-bar-fill" style={{ width: `${p.acceptRate * 100}%`, background: confColor(p.acceptRate) }} />
            </div>
          </div>
        ))}

        {/* Model tab */}
        {tab === "model" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            {/* Q-Table stats */}
            <div className="panel-card">
              <div style={{ fontSize: "var(--font-size-sm)", fontWeight: "var(--font-semibold)", marginBottom: 10 }}>
                Q-Table Statistics
              </div>
              <div className="panel-stats-grid-3">
                {[
                  ["Entries", model.qTableSize.toLocaleString(), "var(--text-info)"],
                  ["States", model.totalStates.toString(), "var(--text-info)"],
                  ["Actions", model.totalActions.toString(), "var(--text-info)"],
                ].map(([l, v, c]) => (
                  <div key={l} className="panel-stat" style={{ background: "var(--bg-primary)" }}>
                    <div className="panel-mono" style={{ fontSize: "var(--font-size-2xl)", fontWeight: "var(--font-bold)", color: c }}>
                      {v}
                    </div>
                    <div className="panel-stat-label">{l}</div>
                  </div>
                ))}
              </div>
            </div>

            {/* Performance */}
            <div className="panel-card">
              <div style={{ fontSize: "var(--font-size-sm)", fontWeight: "var(--font-semibold)", marginBottom: 10 }}>
                Performance
              </div>
              <div className="panel-stats-grid-3" style={{ marginBottom: 10 }}>
                {[
                  ["Total", model.totalPredictions.toLocaleString(), "var(--text-primary)"],
                  ["Accepted", model.accepted.toLocaleString(), "var(--text-success)"],
                  ["Rejected", model.rejected.toLocaleString(), "var(--text-danger)"],
                ].map(([l, v, c]) => (
                  <div key={l} className="panel-stat" style={{ background: "var(--bg-primary)" }}>
                    <div className="panel-mono" style={{ fontSize: "var(--font-size-2xl)", fontWeight: "var(--font-bold)", color: c }}>
                      {v}
                    </div>
                    <div className="panel-stat-label">{l}</div>
                  </div>
                ))}
              </div>
              <div className="progress-bar">
                <div className="progress-bar-fill progress-bar-success" style={{ width: `${model.acceptanceRate * 100}%` }} />
              </div>
              <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginTop: 4, textAlign: "center" }}>
                Acceptance Rate: {(model.acceptanceRate * 100).toFixed(1)}%
              </div>
            </div>

            {/* Hyperparameters */}
            <div className="panel-card">
              <div style={{ fontSize: "var(--font-size-sm)", fontWeight: "var(--font-semibold)", marginBottom: 10 }}>
                Hyperparameters
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                {[
                  { label: "Exploration Rate", value: model.explorationRate.toFixed(2), onMinus: () => adjustExploration(-0.01), onPlus: () => adjustExploration(0.01) },
                  { label: "Decay Rate", value: model.decayRate.toFixed(3), onMinus: () => adjustDecay(-0.001), onPlus: () => adjustDecay(0.001) },
                ].map(({ label, value, onMinus, onPlus }) => (
                  <div key={label} className="panel-row">
                    <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", minWidth: 130 }}>{label}</span>
                    <button className="panel-btn panel-btn-xs panel-btn-secondary" onClick={onMinus}>−</button>
                    <span className="panel-mono" style={{ fontSize: "var(--font-size-base)", fontWeight: "var(--font-semibold)", color: "var(--text-info)", minWidth: 44, textAlign: "center" }}>
                      {value}
                    </span>
                    <button className="panel-btn panel-btn-xs panel-btn-secondary" onClick={onPlus}>+</button>
                  </div>
                ))}
                <div style={{ display: "flex", gap: 16, fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginTop: 4 }}>
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
