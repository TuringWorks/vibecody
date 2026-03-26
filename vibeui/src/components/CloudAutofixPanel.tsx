import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface FixAttempt {
  id: string;
  type: string;
  description: string;
  confidence: number;
  testStatus: string;
  filesChanged: number;
  status: string;
  prNumber: string;
  createdAt: string;
}

interface AutofixStats {
  mergeRate: number;
  totalAttempts: number;
  merged: number;
  rejected: number;
  pending: number;
}

interface AutofixConfig {
  containerImage: string;
  timeoutMinutes: number;
  cpuLimit: string;
  memoryLimit: string;
}

const CloudAutofixPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("pipeline");
  const [prNumber, setPrNumber] = useState("");
  const [containerImage, setContainerImage] = useState("node:20-slim");
  const [timeoutMinutes, setTimeoutMinutes] = useState(10);
  const [cpuLimit, setCpuLimit] = useState("2");
  const [memoryLimit, setMemoryLimit] = useState("4Gi");
  const [analyzing, setAnalyzing] = useState(false);
  const [fixes, setFixes] = useState<FixAttempt[]>([]);
  const [strategy, setStrategy] = useState("Minimal");
  const [stats, setStats] = useState<AutofixStats>({ mergeRate: 0, totalAttempts: 0, merged: 0, rejected: 0, pending: 0 });

  const loadFixes = useCallback(async () => {
    try {
      const attempts = await invoke<FixAttempt[]>("list_autofix_attempts");
      setFixes(attempts);
    } catch (e) {
      console.error("Failed to load autofix attempts:", e);
    }
  }, []);

  const loadStats = useCallback(async () => {
    try {
      const s = await invoke<AutofixStats>("get_autofix_stats");
      setStats(s);
    } catch (e) {
      console.error("Failed to load autofix stats:", e);
    }
  }, []);

  const loadConfig = useCallback(async () => {
    try {
      const cfg = await invoke<AutofixConfig>("get_autofix_config");
      setContainerImage(cfg.containerImage);
      setTimeoutMinutes(cfg.timeoutMinutes);
      setCpuLimit(cfg.cpuLimit);
      setMemoryLimit(cfg.memoryLimit);
    } catch (e) {
      console.error("Failed to load autofix config:", e);
    }
  }, []);

  useEffect(() => {
    loadFixes();
    loadStats();
    loadConfig();
  }, [loadFixes, loadStats, loadConfig]);

  const containerStyle: React.CSSProperties = {
    padding: "16px", color: "var(--text-primary)",
    backgroundColor: "var(--bg-primary)",
    fontFamily: "inherit", fontSize: "13px",
    height: "100%", overflow: "auto",
  };
  const tabBar: React.CSSProperties = { display: "flex", gap: 2, borderBottom: "1px solid var(--border-color)", padding: "0 16px", flexShrink: 0 };
  const tab = (active: boolean): React.CSSProperties => ({
    padding: "8px 16px", cursor: "pointer", border: "none",
    backgroundColor: active ? "var(--bg-secondary)" : "transparent",
    color: active ? "var(--text-primary)" : "var(--text-secondary)",
    borderBottom: active ? "2px solid var(--accent-blue)" : "2px solid transparent",
  });
  const btn: React.CSSProperties = {
    padding: "6px 14px", border: "none", borderRadius: "4px", cursor: "pointer",
    backgroundColor: "var(--accent-color)", color: "var(--btn-primary-fg)",
  };
  const input: React.CSSProperties = {
    padding: "6px 10px", borderRadius: "4px", border: "1px solid var(--border-color)",
    backgroundColor: "var(--bg-secondary)", color: "var(--text-primary)", boxSizing: "border-box",
  };
  const card: React.CSSProperties = {
    padding: "12px", marginBottom: "8px", borderRadius: "6px",
    backgroundColor: "var(--bg-secondary)", border: "1px solid var(--border-color)",
  };
  const badge = (color: string): React.CSSProperties => ({
    padding: "2px 8px", borderRadius: "10px", fontSize: "11px", fontWeight: 600,
    backgroundColor: color, color: "var(--btn-primary-fg)",
  });

  const typeColor = (t: string) => t === "typecheck" ? "#1f6feb" : t === "lint" ? "#8957e5" : t === "test" ? "#d29922" : t === "security" ? "#f85149" : "#6e7681";
  const testStatusColor = (s: string) => s === "passed" ? "#2ea043" : s === "failed" ? "#f85149" : s === "running" ? "#d29922" : "#6e7681";

  const handleAnalyze = async () => {
    if (!prNumber.trim()) return;
    setAnalyzing(true);
    try {
      await invoke<FixAttempt>("create_autofix_attempt", {
        prNumber: prNumber.trim(),
        fixType: "typecheck",
        description: `Autofix analysis for PR #${prNumber.trim()}`,
        confidence: 80,
        filesChanged: 1,
      });
      await loadFixes();
      await loadStats();
    } catch (e) {
      console.error("Failed to create autofix attempt:", e);
    } finally {
      setAnalyzing(false);
    }
  };

  const handleSaveConfig = async () => {
    try {
      await invoke("save_autofix_config", {
        config: { containerImage, timeoutMinutes, cpuLimit, memoryLimit },
      });
    } catch (e) {
      console.error("Failed to save autofix config:", e);
    }
  };

  const handleMerge = async (attemptId: string) => {
    try {
      await invoke<FixAttempt>("update_autofix_status", { attemptId, status: "merged" });
      await loadFixes();
      await loadStats();
    } catch (e) {
      console.error("Failed to merge attempt:", e);
    }
  };

  const handleReject = async (attemptId: string) => {
    try {
      await invoke<FixAttempt>("update_autofix_status", { attemptId, status: "rejected" });
      await loadFixes();
      await loadStats();
    } catch (e) {
      console.error("Failed to reject attempt:", e);
    }
  };

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
            <div style={{ marginTop: "12px", display: "flex", justifyContent: "flex-end" }}>
              <button style={btn} onClick={handleSaveConfig}>Save Config</button>
            </div>
          </div>
        </div>
      )}

      {activeTab === "fixes" && (
        <div>
          <h4 style={{ margin: "0 0 12px" }}>Fix Attempts ({fixes.length})</h4>
          {fixes.length === 0 && (
            <div style={{ ...card, textAlign: "center", opacity: 0.6 }}>No autofix attempts yet. Use the Pipeline tab to analyze a PR.</div>
          )}
          {fixes.map(f => (
            <div key={f.id} style={card}>
              <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "8px" }}>
                <span style={badge(typeColor(f.type))}>{f.type}</span>
                <strong>{f.description}</strong>
                {f.status !== "pending" && (
                  <span style={badge(f.status === "merged" ? "#2ea043" : "#f85149")}>{f.status}</span>
                )}
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
              {f.status === "pending" && (
                <div style={{ display: "flex", gap: "6px", justifyContent: "flex-end" }}>
                  <button style={{ ...btn, backgroundColor: "var(--error-color)" }} onClick={() => handleReject(f.id)}>Reject</button>
                  <button style={{ ...btn, backgroundColor: "var(--success-color)" }} onClick={() => handleMerge(f.id)}>Merge</button>
                </div>
              )}
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
                <div style={{ height: "100%", borderRadius: "10px", width: `${Math.round(stats.mergeRate)}%`, backgroundColor: "var(--success-color)", display: "flex", alignItems: "center", justifyContent: "center", fontSize: "11px", fontWeight: 700, color: "var(--btn-primary-fg)" }}>
                  {Math.round(stats.mergeRate)}%
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
              <div key={s.label} style={{ ...card, textAlign: "center" as const }}>
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
