/**
 * EditPredictionPanel — RL-trained Edit Prediction.
 *
 * Tabs: Predictions (recent predictions with confidence, accept/reject),
 * Patterns (detected edit patterns with frequency),
 * Model (Q-table stats, exploration rate, acceptance rate, decay).
 * Pure TypeScript — no Tauri commands.
 */
import { useState } from "react";

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

const MOCK_PREDICTIONS: Prediction[] = [
  { id: "pr1", file: "src/main.rs", line: 42, suggestion: "Add error handling with ? operator", confidence: 0.94, state: "pending", timestamp: "10s ago", pattern: "error-propagation" },
  { id: "pr2", file: "src/lib.rs", line: 78, suggestion: "Extract repeated block into helper function", confidence: 0.87, state: "pending", timestamp: "30s ago", pattern: "extract-function" },
  { id: "pr3", file: "src/config.rs", line: 15, suggestion: "Use Default derive instead of manual impl", confidence: 0.76, state: "pending", timestamp: "1m ago", pattern: "derive-default" },
  { id: "pr4", file: "src/utils.rs", line: 33, suggestion: "Replace .unwrap() with .expect(\"msg\")", confidence: 0.92, state: "accepted", timestamp: "2m ago", pattern: "unwrap-to-expect" },
  { id: "pr5", file: "src/handler.rs", line: 55, suggestion: "Add #[must_use] attribute", confidence: 0.68, state: "rejected", timestamp: "3m ago", pattern: "must-use-attr" },
  { id: "pr6", file: "src/main.rs", line: 120, suggestion: "Use if let instead of match with single arm", confidence: 0.81, state: "accepted", timestamp: "5m ago", pattern: "if-let-simplify" },
];

const MOCK_PATTERNS: EditPattern[] = [
  { id: "pt1", name: "error-propagation", description: "Add ? operator for Result/Option error handling", frequency: 142, lastSeen: "10s ago", avgConfidence: 0.91, acceptRate: 0.88 },
  { id: "pt2", name: "extract-function", description: "Extract repeated code blocks into reusable functions", frequency: 87, lastSeen: "30s ago", avgConfidence: 0.84, acceptRate: 0.72 },
  { id: "pt3", name: "unwrap-to-expect", description: "Replace .unwrap() with .expect() for better panic messages", frequency: 64, lastSeen: "2m ago", avgConfidence: 0.93, acceptRate: 0.95 },
  { id: "pt4", name: "derive-default", description: "Use #[derive(Default)] instead of manual Default impl", frequency: 38, lastSeen: "1m ago", avgConfidence: 0.78, acceptRate: 0.65 },
  { id: "pt5", name: "if-let-simplify", description: "Simplify single-arm match to if let", frequency: 51, lastSeen: "5m ago", avgConfidence: 0.82, acceptRate: 0.79 },
  { id: "pt6", name: "must-use-attr", description: "Add #[must_use] to functions returning important values", frequency: 22, lastSeen: "3m ago", avgConfidence: 0.65, acceptRate: 0.45 },
];

const MOCK_MODEL: ModelStats = {
  qTableSize: 1247, totalStates: 89, totalActions: 14,
  explorationRate: 0.15, learningRate: 0.01, discountFactor: 0.95,
  acceptanceRate: 0.78, totalPredictions: 3421, accepted: 2668, rejected: 753,
  decayRate: 0.999,
};

const tabBtn = (active: boolean): React.CSSProperties => ({
  padding: "6px 14px", fontSize: 11, fontWeight: active ? 600 : 400,
  background: active ? "var(--accent-bg, rgba(99,102,241,0.15))" : "transparent",
  border: "1px solid " + (active ? "var(--accent-primary)" : "var(--border-color)"),
  borderRadius: 4, color: active ? "var(--text-info)" : "var(--text-muted)", cursor: "pointer",
});

const confColor = (c: number) => c > 0.85 ? "var(--text-success)" : c > 0.7 ? "var(--text-warning)" : "var(--text-danger)";

export default function EditPredictionPanel() {
  const [tab, setTab] = useState<Tab>("predictions");
  const [predictions, setPredictions] = useState(MOCK_PREDICTIONS);
  const [model, setModel] = useState(MOCK_MODEL);

  const handlePrediction = (id: string, action: "accepted" | "rejected") => {
    setPredictions(ps => ps.map(p => p.id === id ? { ...p, state: action } : p));
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
        <span style={{ marginLeft: "auto", fontSize: 10, color: "var(--text-muted)", alignSelf: "center" }}>
          {(model.acceptanceRate * 100).toFixed(0)}% accept rate
        </span>
      </div>

      <div style={{ flex: 1, overflowY: "auto", padding: 12, display: "flex", flexDirection: "column", gap: 10 }}>
        {tab === "predictions" && predictions.map(p => (
          <div key={p.id} style={{ padding: 10, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)", opacity: p.state !== "pending" ? 0.6 : 1 }}>
            <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 6 }}>
              <span style={{ fontSize: 11, fontFamily: "monospace", color: "var(--text-info)" }}>{p.file}:{p.line}</span>
              <span style={{ fontSize: 9, padding: "1px 5px", borderRadius: 3, background: "rgba(99,102,241,0.12)", color: "var(--text-muted)" }}>{p.pattern}</span>
              <span style={{ fontSize: 10, color: "var(--text-muted)", marginLeft: "auto" }}>{p.timestamp}</span>
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
                    style={{ padding: "3px 10px", fontSize: 10, borderRadius: 3, border: "none", background: "var(--text-success)", color: "#1e1e2e", cursor: "pointer", fontWeight: 600 }}>Accept</button>
                </>
              ) : (
                <span style={{ fontSize: 10, color: p.state === "accepted" ? "var(--text-success)" : "var(--text-danger)" }}>{p.state}</span>
              )}
            </div>
          </div>
        ))}

        {tab === "patterns" && MOCK_PATTERNS.map(p => (
          <div key={p.id} style={{ padding: 10, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
            <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 4 }}>
              <span style={{ fontSize: 11, fontWeight: 600, fontFamily: "monospace", color: "var(--accent-primary)" }}>{p.name}</span>
              <span style={{ fontSize: 10, color: "var(--text-muted)", marginLeft: "auto" }}>seen {p.frequency}x</span>
            </div>
            <div style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 8 }}>{p.description}</div>
            <div style={{ display: "flex", gap: 16, fontSize: 10 }}>
              <span style={{ color: "var(--text-muted)" }}>Confidence: <span style={{ color: confColor(p.avgConfidence), fontWeight: 600 }}>{(p.avgConfidence * 100).toFixed(0)}%</span></span>
              <span style={{ color: "var(--text-muted)" }}>Accept: <span style={{ color: confColor(p.acceptRate), fontWeight: 600 }}>{(p.acceptRate * 100).toFixed(0)}%</span></span>
              <span style={{ color: "var(--text-muted)" }}>Last: {p.lastSeen}</span>
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
                    <div style={{ fontSize: 18, fontWeight: 700, color: "var(--text-info)", fontFamily: "monospace" }}>{v}</div>
                    <div style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 4 }}>{l}</div>
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
                    <div style={{ fontSize: 18, fontWeight: 700, color: c, fontFamily: "monospace" }}>{v}</div>
                    <div style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 4 }}>{l}</div>
                  </div>
                ))}
              </div>
              <div style={{ marginTop: 10, height: 6, background: "var(--bg-primary)", borderRadius: 3, overflow: "hidden" }}>
                <div style={{ width: `${model.acceptanceRate * 100}%`, height: "100%", background: "var(--text-success)", borderRadius: 3 }} />
              </div>
              <div style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 4, textAlign: "center" }}>
                Acceptance Rate: {(model.acceptanceRate * 100).toFixed(1)}%
              </div>
            </div>

            <div style={{ padding: 14, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
              <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 10 }}>Hyperparameters</div>
              <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                  <span style={{ fontSize: 11, color: "var(--text-muted)", minWidth: 120 }}>Exploration Rate</span>
                  <button onClick={() => adjustExploration(-0.01)} style={{ padding: "2px 8px", fontSize: 10, border: "1px solid var(--border-color)", borderRadius: 3, background: "var(--bg-primary)", color: "var(--text-muted)", cursor: "pointer" }}>-</button>
                  <span style={{ fontSize: 12, fontFamily: "monospace", fontWeight: 600, color: "var(--text-info)", minWidth: 40, textAlign: "center" }}>{model.explorationRate.toFixed(2)}</span>
                  <button onClick={() => adjustExploration(0.01)} style={{ padding: "2px 8px", fontSize: 10, border: "1px solid var(--border-color)", borderRadius: 3, background: "var(--bg-primary)", color: "var(--text-muted)", cursor: "pointer" }}>+</button>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                  <span style={{ fontSize: 11, color: "var(--text-muted)", minWidth: 120 }}>Decay Rate</span>
                  <button onClick={() => adjustDecay(-0.001)} style={{ padding: "2px 8px", fontSize: 10, border: "1px solid var(--border-color)", borderRadius: 3, background: "var(--bg-primary)", color: "var(--text-muted)", cursor: "pointer" }}>-</button>
                  <span style={{ fontSize: 12, fontFamily: "monospace", fontWeight: 600, color: "var(--text-info)", minWidth: 40, textAlign: "center" }}>{model.decayRate.toFixed(3)}</span>
                  <button onClick={() => adjustDecay(0.001)} style={{ padding: "2px 8px", fontSize: 10, border: "1px solid var(--border-color)", borderRadius: 3, background: "var(--bg-primary)", color: "var(--text-muted)", cursor: "pointer" }}>+</button>
                </div>
                <div style={{ display: "flex", gap: 16, fontSize: 10, color: "var(--text-muted)", marginTop: 4 }}>
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
