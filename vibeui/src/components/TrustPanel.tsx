import { useState } from "react";

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

const scoreBarColor = (score: number) => score >= 80 ? "#22c55e" : score >= 50 ? "#eab308" : "#ef4444";

export function TrustPanel() {
  const [tab, setTab] = useState("scores");
  const [decayRate, setDecayRate] = useState(5);
  const [recoveryRate, setRecoveryRate] = useState(10);
  const [autoMergeThreshold, setAutoMergeThreshold] = useState(85);
  const [manualReviewThreshold, setManualReviewThreshold] = useState(50);

  const scores = [
    { model: "Claude Opus", score: 94, tasks: 412 },
    { model: "GPT-4o", score: 87, tasks: 289 },
    { model: "Gemini Pro", score: 78, tasks: 156 },
    { model: "Ollama Llama3", score: 62, tasks: 98 },
    { model: "Mistral Large", score: 71, tasks: 134 },
    { model: "DeepSeek V3", score: 55, tasks: 67 },
  ];

  const events = [
    { model: "Claude Opus", action: "Code generation", result: "success", delta: "+2", time: "3 min ago" },
    { model: "GPT-4o", action: "Test generation", result: "failure", delta: "-5", time: "8 min ago" },
    { model: "Claude Opus", action: "Bug fix", result: "success", delta: "+3", time: "15 min ago" },
    { model: "Gemini Pro", action: "Refactor", result: "success", delta: "+1", time: "22 min ago" },
    { model: "Ollama Llama3", action: "Documentation", result: "failure", delta: "-3", time: "30 min ago" },
    { model: "DeepSeek V3", action: "API endpoint", result: "success", delta: "+2", time: "45 min ago" },
  ];

  const domains = [
    { domain: "Code Generation", scores: { "Claude Opus": 96, "GPT-4o": 88, "Gemini Pro": 74 } },
    { domain: "Bug Fixing", scores: { "Claude Opus": 92, "GPT-4o": 85, "Gemini Pro": 80 } },
    { domain: "Testing", scores: { "Claude Opus": 91, "GPT-4o": 82, "Gemini Pro": 76 } },
    { domain: "Documentation", scores: { "Claude Opus": 94, "GPT-4o": 90, "Gemini Pro": 82 } },
    { domain: "Refactoring", scores: { "Claude Opus": 89, "GPT-4o": 83, "Gemini Pro": 71 } },
  ];

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Agent Trust Scoring</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        {["scores", "events", "domains", "config"].map((t) => (
          <button key={t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {tab === "scores" && (
        <div>
          {scores.map((s) => (
            <div key={s.model} style={{ ...cardStyle, display: "flex", alignItems: "center", gap: 12 }}>
              <div style={{ minWidth: 120, fontWeight: 600, fontSize: 13 }}>{s.model}</div>
              <div style={{ flex: 1, height: 8, borderRadius: 4, background: "var(--border-color)" }}>
                <div style={{ width: `${s.score}%`, height: 8, borderRadius: 4, background: scoreBarColor(s.score) }} />
              </div>
              <span style={{ fontWeight: 600, fontSize: 13, color: scoreBarColor(s.score), minWidth: 36 }}>{s.score}</span>
              <span style={{ fontSize: 11, color: "var(--text-secondary)", minWidth: 60 }}>{s.tasks} tasks</span>
            </div>
          ))}
        </div>
      )}

      {tab === "events" && (
        <div>
          {events.map((e, i) => (
            <div key={i} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <span style={{ fontWeight: 600, fontSize: 13 }}>{e.model}</span>
                <span style={{ fontSize: 12, color: "var(--text-secondary)", marginLeft: 8 }}>{e.action}</span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>{e.time}</span>
                <span style={{
                  padding: "2px 8px", borderRadius: 4, fontSize: 11, fontWeight: 600,
                  background: e.result === "success" ? "#22c55e20" : "#ef444420",
                  color: e.result === "success" ? "#22c55e" : "#ef4444",
                }}>{e.result}</span>
                <span style={{ fontWeight: 600, fontSize: 12, color: e.delta.startsWith("+") ? "#22c55e" : "#ef4444" }}>{e.delta}</span>
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "domains" && (
        <div>
          {domains.map((d) => (
            <div key={d.domain} style={cardStyle}>
              <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>{d.domain}</div>
              {Object.entries(d.scores).map(([model, score]) => (
                <div key={model} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                  <span style={{ fontSize: 12, minWidth: 100, color: "var(--text-secondary)" }}>{model}</span>
                  <div style={{ flex: 1, height: 6, borderRadius: 3, background: "var(--border-color)" }}>
                    <div style={{ width: `${score}%`, height: 6, borderRadius: 3, background: scoreBarColor(score) }} />
                  </div>
                  <span style={{ fontSize: 11, fontWeight: 600, color: scoreBarColor(score), minWidth: 28 }}>{score}</span>
                </div>
              ))}
            </div>
          ))}
        </div>
      )}

      {tab === "config" && (
        <div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Decay Rate: {decayRate} pts/day</div>
            <input type="range" min={1} max={20} value={decayRate} onChange={(e) => setDecayRate(Number(e.target.value))} style={{ width: "100%" }} />
          </div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Recovery Rate: {recoveryRate} pts/success</div>
            <input type="range" min={1} max={25} value={recoveryRate} onChange={(e) => setRecoveryRate(Number(e.target.value))} style={{ width: "100%" }} />
          </div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Auto-Merge Threshold: {autoMergeThreshold}</div>
            <input type="range" min={50} max={100} value={autoMergeThreshold} onChange={(e) => setAutoMergeThreshold(Number(e.target.value))} style={{ width: "100%" }} />
          </div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Manual Review Below: {manualReviewThreshold}</div>
            <input type="range" min={10} max={80} value={manualReviewThreshold} onChange={(e) => setManualReviewThreshold(Number(e.target.value))} style={{ width: "100%" }} />
          </div>
        </div>
      )}
    </div>
  );
}
