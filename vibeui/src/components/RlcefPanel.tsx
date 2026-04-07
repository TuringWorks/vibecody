import { useState } from "react";

export function RlcefPanel() {
  const [tab, setTab] = useState("dashboard");
  const [exportFormat, setExportFormat] = useState("jsonl");

  const outcomes = { pass: 847, fail: 153, total: 1000 };
  const passRate = ((outcomes.pass / outcomes.total) * 100).toFixed(1);

  const rewards = [
    { range: "-1.0 to -0.5", count: 42, color: "var(--error-color)" },
    { range: "-0.5 to 0.0", count: 111, color: "var(--warning-color)" },
    { range: "0.0 to 0.5", count: 298, color: "var(--warning-color)" },
    { range: "0.5 to 1.0", count: 549, color: "var(--success-color)" },
  ];
  const maxReward = Math.max(...rewards.map((r) => r.count));

  const mistakes = [
    { pattern: "Missing error handling", frequency: 34, category: "Safety" },
    { pattern: "Incorrect import path", frequency: 28, category: "Syntax" },
    { pattern: "Unused variable introduced", frequency: 22, category: "Quality" },
    { pattern: "Off-by-one in loop", frequency: 19, category: "Logic" },
    { pattern: "Hardcoded credentials", frequency: 8, category: "Security" },
    { pattern: "Race condition in async", frequency: 6, category: "Concurrency" },
  ];

  const catColor = (c: string) => {
    const map: Record<string, string> = { Safety: "var(--error-color)", Syntax: "var(--accent-color)", Quality: "var(--accent-purple)", Logic: "var(--warning-color)", Security: "var(--error-color)", Concurrency: "#06b6d4" };
    return map[c] || "var(--text-secondary)";
  };

  const strategies = [
    { name: "Temperature", before: "0.7", after: "0.4", reason: "Reduce hallucination on code edits", time: "2 days ago" },
    { name: "System prompt weight", before: "1.0", after: "1.3", reason: "Improve instruction following", time: "3 days ago" },
    { name: "Max retries", before: "2", after: "3", reason: "Better error recovery rate", time: "5 days ago" },
    { name: "Context window", before: "8K", after: "16K", reason: "Handle larger files", time: "1 week ago" },
    { name: "Top-p sampling", before: "0.95", after: "0.9", reason: "Tighter output distribution", time: "1 week ago" },
  ];

  return (
    <div className="panel-container">
      <h2 style={{ fontSize: 18, fontWeight: 600, marginBottom: 12, color: "var(--text-primary)" }}>RLCEF Training Loop</h2>
      <div className="panel-tab-bar" style={{ marginBottom: 16 }}>
        {["dashboard", "mistakes", "strategies", "export"].map((t) => (
          <button key={t} className={`panel-tab ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {tab === "dashboard" && (
        <div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8, marginBottom: 16 }}>
            <div className="panel-card">
              <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Pass</div>
              <div style={{ fontSize: 22, fontWeight: 700, color: "var(--success-color)" }}>{outcomes.pass}</div>
            </div>
            <div className="panel-card">
              <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Fail</div>
              <div style={{ fontSize: 22, fontWeight: 700, color: "var(--error-color)" }}>{outcomes.fail}</div>
            </div>
            <div className="panel-card">
              <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>Pass Rate</div>
              <div style={{ fontSize: 22, fontWeight: 700, color: "var(--accent-color)" }}>{passRate}%</div>
            </div>
          </div>
          <div className="panel-card" style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Reward Distribution</div>
          {rewards.map((r) => (
            <div key={r.range} className="panel-card" style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <span style={{ fontSize: 12, minWidth: 100, color: "var(--text-secondary)" }}>{r.range}</span>
              <div style={{ flex: 1, height: 8, borderRadius: 4, background: "var(--border-color)" }}>
                <div style={{ width: `${(r.count / maxReward) * 100}%`, height: 8, borderRadius: 4, background: r.color }} />
              </div>
              <span style={{ fontSize: 12, fontWeight: 600, minWidth: 36 }}>{r.count}</span>
            </div>
          ))}
        </div>
      )}

      {tab === "mistakes" && (
        <div>
          {mistakes.map((m, i) => (
            <div key={i} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontSize: 13, fontWeight: 600 }}>{m.pattern}</div>
                <span style={{ fontSize: 11, color: catColor(m.category), fontWeight: 600 }}>{m.category}</span>
              </div>
              <div style={{ textAlign: "right" }}>
                <div style={{ fontSize: 18, fontWeight: 700, color: "var(--text-primary)" }}>{m.frequency}</div>
                <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>occurrences</div>
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "strategies" && (
        <div>
          {strategies.map((s, i) => (
            <div key={i} className="panel-card">
              <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
                <span style={{ fontWeight: 600, fontSize: 13 }}>{s.name}</span>
                <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>{s.time}</span>
              </div>
              <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 4 }}>
                <span style={{ fontSize: 12, color: "var(--error-color)", textDecoration: "line-through" }}>{s.before}</span>
                <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>-&gt;</span>
                <span style={{ fontSize: 12, color: "var(--success-color)", fontWeight: 600 }}>{s.after}</span>
              </div>
              <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{s.reason}</div>
            </div>
          ))}
        </div>
      )}

      {tab === "export" && (
        <div>
          <div className="panel-card">
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Training Data Format</div>
            <div style={{ display: "flex", gap: 8 }}>
              {["jsonl", "parquet", "csv"].map((f) => (
                <button key={f} onClick={() => setExportFormat(f)}
                  className={`panel-btn ${exportFormat === f ? "panel-btn-primary" : "panel-btn-secondary"}`}>
                  {f.toUpperCase()}
                </button>
              ))}
            </div>
          </div>
          <div className="panel-card" style={{ fontSize: 13, color: "var(--text-secondary)" }}>
            {outcomes.total} samples | {outcomes.pass} positive | {outcomes.fail} negative
          </div>
          <button className="panel-btn panel-btn-primary">Export Training Data</button>
        </div>
      )}
    </div>
  );
}
