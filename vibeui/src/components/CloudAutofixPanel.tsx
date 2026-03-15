import React, { useState } from "react";

interface FixAttempt {
  id: string;
  type: "lint" | "typecheck" | "test" | "security" | "style";
  description: string;
  confidence: number;
  testStatus: "passed" | "failed" | "running" | "pending";
  filesChanged: number;
}

const CloudAutofixPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("pipeline");
  const [prNumber, setPrNumber] = useState("");
  const [containerImage, setContainerImage] = useState("node:20-slim");
  const [timeoutMinutes, setTimeoutMinutes] = useState(10);
  const [cpuLimit, setCpuLimit] = useState("2");
  const [memoryLimit, setMemoryLimit] = useState("4Gi");
  const [analyzing, setAnalyzing] = useState(false);
  const [fixes] = useState<FixAttempt[]>([
    { id: "f1", type: "typecheck", description: "Fix missing return type on fetchData()", confidence: 95, testStatus: "passed", filesChanged: 1 },
    { id: "f2", type: "lint", description: "Replace var with const in utils.ts", confidence: 88, testStatus: "passed", filesChanged: 3 },
    { id: "f3", type: "test", description: "Add missing assertion in auth.test.ts", confidence: 72, testStatus: "running", filesChanged: 1 },
    { id: "f4", type: "security", description: "Sanitize user input in query builder", confidence: 81, testStatus: "pending", filesChanged: 2 },
    { id: "f5", type: "style", description: "Normalize indentation in config module", confidence: 99, testStatus: "passed", filesChanged: 5 },
  ]);
  const [strategy, setStrategy] = useState("Minimal");
  const [stats] = useState({ mergeRate: 78, totalAttempts: 142, merged: 111, rejected: 19, pending: 12 });

  const containerStyle: React.CSSProperties = {
    padding: "16px", color: "var(--text-primary)",
    backgroundColor: "var(--bg-primary)",
    fontFamily: "inherit", fontSize: "13px",
    height: "100%", overflow: "auto",
  };
  const tabBar: React.CSSProperties = { display: "flex", gap: "4px", marginBottom: "16px", borderBottom: "1px solid var(--border-color)" };
  const tab = (active: boolean): React.CSSProperties => ({
    padding: "8px 16px", cursor: "pointer", border: "none",
    backgroundColor: active ? "var(--bg-secondary)" : "transparent",
    color: active ? "var(--text-primary)" : "var(--text-secondary)",
    borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
  });
  const btn: React.CSSProperties = {
    padding: "6px 14px", border: "none", borderRadius: "4px", cursor: "pointer",
    backgroundColor: "var(--accent-color)", color: "white",
  };
  const input: React.CSSProperties = {
    padding: "6px 10px", borderRadius: "4px", border: "1px solid var(--border-color)",
    backgroundColor: "var(--bg-secondary)", color: "var(--text-primary)",
  };
  const card: React.CSSProperties = {
    padding: "12px", marginBottom: "8px", borderRadius: "6px",
    backgroundColor: "var(--bg-secondary)", border: "1px solid var(--border-color)",
  };
  const badge = (color: string): React.CSSProperties => ({
    padding: "2px 8px", borderRadius: "10px", fontSize: "11px", fontWeight: 600,
    backgroundColor: color, color: "white",
  });

  const typeColor = (t: string) => t === "typecheck" ? "#1f6feb" : t === "lint" ? "#8957e5" : t === "test" ? "#d29922" : t === "security" ? "#f85149" : "#6e7681";
  const testStatusColor = (s: string) => s === "passed" ? "#2ea043" : s === "failed" ? "#f85149" : s === "running" ? "#d29922" : "#6e7681";

  const handleAnalyze = () => { setAnalyzing(true); setTimeout(() => setAnalyzing(false), 2000); };

  return (
    <div style={containerStyle}>
      <h3 style={{ margin: "0 0 12px" }}>Cloud Autofix</h3>
      <div style={tabBar}>
        {["pipeline", "fixes", "stats"].map(t => (
          <button key={t} style={tab(activeTab === t)} onClick={() => setActiveTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {activeTab === "pipeline" && (
        <div>
          <div style={card}>
            <h4 style={{ margin: "0 0 12px" }}>Analyze Pull Request</h4>
            <div style={{ display: "flex", gap: "8px", marginBottom: "16px" }}>
              <input style={{ ...input, flex: 1 }} placeholder="PR number (e.g., 123)" value={prNumber} onChange={e => setPrNumber(e.target.value)} />
              <button style={btn} onClick={handleAnalyze} disabled={analyzing}>
                {analyzing ? "Analyzing..." : "Analyze"}
              </button>
            </div>
            <h4 style={{ margin: "0 0 12px" }}>Sandbox Configuration</h4>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "12px" }}>
              <div>
                <label style={{ display: "block", marginBottom: "4px", fontWeight: 600 }}>Container Image</label>
                <input style={{ ...input, width: "100%" }} value={containerImage} onChange={e => setContainerImage(e.target.value)} />
              </div>
              <div>
                <label style={{ display: "block", marginBottom: "4px", fontWeight: 600 }}>Timeout (min)</label>
                <input style={{ ...input, width: "100%" }} type="number" value={timeoutMinutes} onChange={e => setTimeoutMinutes(Number(e.target.value))} />
              </div>
              <div>
                <label style={{ display: "block", marginBottom: "4px", fontWeight: 600 }}>CPU Limit</label>
                <input style={{ ...input, width: "100%" }} value={cpuLimit} onChange={e => setCpuLimit(e.target.value)} />
              </div>
              <div>
                <label style={{ display: "block", marginBottom: "4px", fontWeight: 600 }}>Memory Limit</label>
                <input style={{ ...input, width: "100%" }} value={memoryLimit} onChange={e => setMemoryLimit(e.target.value)} />
              </div>
            </div>
          </div>
        </div>
      )}

      {activeTab === "fixes" && (
        <div>
          <h4 style={{ margin: "0 0 12px" }}>Fix Attempts ({fixes.length})</h4>
          {fixes.map(f => (
            <div key={f.id} style={card}>
              <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "8px" }}>
                <span style={badge(typeColor(f.type))}>{f.type}</span>
                <strong>{f.description}</strong>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: "16px", marginBottom: "8px" }}>
                <div style={{ flex: 1 }}>
                  <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "4px" }}>
                    <span>Confidence</span><span>{f.confidence}%</span>
                  </div>
                  <div style={{ height: "6px", borderRadius: "3px", backgroundColor: "var(--border-color)" }}>
                    <div style={{ height: "100%", borderRadius: "3px", width: `${f.confidence}%`, backgroundColor: f.confidence > 80 ? "var(--success-color)" : f.confidence > 60 ? "var(--warning-color)" : "var(--error-color)" }} />
                  </div>
                </div>
                <span style={badge(testStatusColor(f.testStatus))}>{f.testStatus}</span>
                <span style={{ opacity: 0.6 }}>{f.filesChanged} file{f.filesChanged > 1 ? "s" : ""}</span>
              </div>
              <div style={{ display: "flex", gap: "6px", justifyContent: "flex-end" }}>
                <button style={btn}>Propose</button>
                <button style={{ ...btn, backgroundColor: "var(--success-color)" }}>Merge</button>
              </div>
            </div>
          ))}
        </div>
      )}

      {activeTab === "stats" && (
        <div>
          <div style={card}>
            <h4 style={{ margin: "0 0 12px" }}>Merge Rate</h4>
            <div style={{ display: "flex", alignItems: "center", gap: "12px", marginBottom: "8px" }}>
              <div style={{ flex: 1, height: "20px", borderRadius: "10px", backgroundColor: "var(--border-color)" }}>
                <div style={{ height: "100%", borderRadius: "10px", width: `${stats.mergeRate}%`, backgroundColor: "var(--success-color)", display: "flex", alignItems: "center", justifyContent: "center", fontSize: "11px", fontWeight: 700, color: "white" }}>
                  {stats.mergeRate}%
                </div>
              </div>
            </div>
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: "8px", marginBottom: "16px" }}>
            {[
              { label: "Total Attempts", value: stats.totalAttempts, color: "var(--text-primary)" },
              { label: "Merged", value: stats.merged, color: "var(--success-color)" },
              { label: "Rejected", value: stats.rejected, color: "var(--error-color)" },
              { label: "Pending", value: stats.pending, color: "var(--warning-color)" },
            ].map(s => (
              <div key={s.label} style={{ ...card, textAlign: "center" }}>
                <div style={{ fontSize: "24px", fontWeight: 700, color: s.color }}>{s.value}</div>
                <div style={{ opacity: 0.7, fontSize: "12px" }}>{s.label}</div>
              </div>
            ))}
          </div>
          <div style={card}>
            <h4 style={{ margin: "0 0 8px" }}>Fix Strategy</h4>
            <div style={{ display: "flex", gap: "8px" }}>
              {["Direct", "Minimal", "Comprehensive"].map(s => (
                <button key={s} style={{ ...btn, backgroundColor: strategy === s ? "var(--accent-color)" : "var(--bg-secondary)", color: strategy === s ? "white" : "var(--text-primary)" }} onClick={() => setStrategy(s)}>
                  {s}
                </button>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default CloudAutofixPanel;
