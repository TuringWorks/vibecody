/**
 * BisectPanel — AI-assisted git bisect workflow.
 *
 * Three views: setup (bad/good SHA) → running (good/bad/skip steps) → done (culprit found).
 * Includes AI analysis of the bisect session.
 */
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface BisectStepResult {
  current_commit: string;
  commit_message: string;
  commits_remaining: number | null;
  is_done: boolean;
  culprit_commit: string | null;
}

interface BisectPanelProps {
  workspacePath: string | null;
}

type BisectView = "setup" | "running" | "done";

export function BisectPanel({ workspacePath }: BisectPanelProps) {
  const [view, setView] = useState<BisectView>("setup");
  const [bad, setBad] = useState("");
  const [good, setGood] = useState("");
  const [current, setCurrent] = useState<BisectStepResult | null>(null);
  const [bisectLog, setBisectLog] = useState("");
  const [analysis, setAnalysis] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showLog, setShowLog] = useState(false);

  if (!workspacePath) {
    return (
      <div style={{ padding: 16, opacity: 0.6, textAlign: "center" }}>
        <p>Open a workspace folder to use git bisect.</p>
      </div>
    );
  }

  const handleStart = async () => {
    if (!bad.trim() || !good.trim()) {
      setError("Both bad and good commits are required");
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<string>("git_bisect_start", { workspace: workspacePath, bad: bad.trim(), good: good.trim() });
      // After start, the first bisect step info is in the output
      setView("running");
      setCurrent({
        current_commit: "",
        commit_message: result,
        commits_remaining: null,
        is_done: false,
        culprit_commit: null,
      });
    } catch (e: unknown) {
      setError(String(e));
    }
    setLoading(false);
  };

  const handleStep = async (verdict: "good" | "bad" | "skip") => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<BisectStepResult>("git_bisect_step", { workspace: workspacePath, verdict });
      setCurrent(result);
      if (result.is_done) {
        setView("done");
      }
    } catch (e: unknown) {
      setError(String(e));
    }
    setLoading(false);
  };

  const handleReset = async () => {
    setLoading(true);
    try {
      await invoke<string>("git_bisect_reset", { workspace: workspacePath });
    } catch (_) { /* ignore */ }
    setView("setup");
    setCurrent(null);
    setBisectLog("");
    setAnalysis("");
    setShowLog(false);
    setError(null);
    setLoading(false);
  };

  const handleLog = async () => {
    try {
      const log = await invoke<string>("git_bisect_log", { workspace: workspacePath });
      setBisectLog(log);
      setShowLog(true);
    } catch (e: unknown) {
      setError(String(e));
    }
  };

  const handleAnalyze = async () => {
    setLoading(true);
    setError(null);
    try {
      let log = bisectLog;
      if (!log) {
        log = await invoke<string>("git_bisect_log", { workspace: workspacePath });
        setBisectLog(log);
      }
      const result = await invoke<string>("ai_bisect_analyze", { workspace: workspacePath, bisectLog: log });
      setAnalysis(result);
    } catch (e: unknown) {
      setError(String(e));
    }
    setLoading(false);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Header */}
      <div style={{
        display: "flex", gap: 6, padding: "8px 12px", alignItems: "center",
        borderBottom: "1px solid var(--border-color)",
      }}>
        <span style={{ fontSize: 12, fontWeight: 600 }}>Git Bisect</span>
        <div style={{ flex: 1 }} />
        {view !== "setup" && (
          <button onClick={handleReset} style={{ ...btnStyle, color: "var(--text-danger)" }}>Reset</button>
        )}
      </div>

      {error && (
        <div style={{ padding: "6px 12px", fontSize: 11, color: "var(--text-danger)", background: "rgba(243,139,168,0.05)" }}>
          {error}
        </div>
      )}

      <div style={{ flex: 1, overflowY: "auto", padding: "8px 12px" }}>
        {/* Setup view */}
        {view === "setup" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            <div style={{ fontSize: 11, opacity: 0.7, lineHeight: 1.5 }}>
              Enter the known bad commit (has the bug) and a known good commit (before the bug).
              Git bisect will binary-search to find the exact commit that introduced the issue.
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              <label style={{ fontSize: 10, fontWeight: 600, opacity: 0.6 }}>Bad commit (has bug)</label>
              <input
                value={bad}
                onChange={(e) => setBad(e.target.value)}
                placeholder="HEAD or commit SHA..."
                style={inputStyle}
              />
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              <label style={{ fontSize: 10, fontWeight: 600, opacity: 0.6 }}>Good commit (before bug)</label>
              <input
                value={good}
                onChange={(e) => setGood(e.target.value)}
                placeholder="Commit SHA or tag..."
                style={inputStyle}
              />
            </div>
            <button onClick={handleStart} disabled={loading} style={{ ...btnStyle, background: "var(--accent-color)", color: "var(--text-primary)" }}>
              {loading ? "Starting..." : "Start Bisect"}
            </button>
          </div>
        )}

        {/* Running view */}
        {view === "running" && current && (
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            {/* Current commit */}
            <div style={{
              padding: "8px 10px", borderRadius: 6,
              background: "var(--bg-primary)", border: "1px solid var(--border-color)",
            }}>
              <div style={{ fontSize: 10, fontWeight: 600, opacity: 0.6, marginBottom: 4 }}>Current Commit</div>
              {current.current_commit && (
                <div style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--text-warning)" }}>
                  {current.current_commit.substring(0, 12)}
                </div>
              )}
              <div style={{ fontSize: 11, marginTop: 2, whiteSpace: "pre-wrap" }}>
                {current.commit_message}
              </div>
              {current.commits_remaining != null && (
                <div style={{ fontSize: 10, opacity: 0.5, marginTop: 4 }}>
                  ~{current.commits_remaining} revisions remaining
                </div>
              )}
            </div>

            {/* Verdict buttons */}
            <div style={{ display: "flex", gap: 8 }}>
              <button
                onClick={() => handleStep("good")}
                disabled={loading}
                style={{ ...btnStyle, flex: 1, background: "rgba(166,227,161,0.15)", color: "var(--text-success)", fontWeight: 700 }}
              >
                Good
              </button>
              <button
                onClick={() => handleStep("bad")}
                disabled={loading}
                style={{ ...btnStyle, flex: 1, background: "rgba(243,139,168,0.15)", color: "var(--text-danger)", fontWeight: 700 }}
              >
                Bad
              </button>
              <button
                onClick={() => handleStep("skip")}
                disabled={loading}
                style={{ ...btnStyle, flex: 1 }}
              >
                Skip
              </button>
            </div>

            {/* Log + AI */}
            <div style={{ display: "flex", gap: 6 }}>
              <button onClick={handleLog} style={btnStyle}>Show Log</button>
              <button onClick={handleAnalyze} disabled={loading} style={{ ...btnStyle, color: "var(--text-info)" }}>
                {loading ? "Analyzing..." : "AI Analyze"}
              </button>
            </div>

            {showLog && bisectLog && (
              <pre style={{
                padding: 8, fontSize: 10, fontFamily: "var(--font-mono)", borderRadius: 4,
                background: "var(--bg-primary)", maxHeight: 150, overflowY: "auto",
                whiteSpace: "pre-wrap", wordBreak: "break-all",
              }}>
                {bisectLog}
              </pre>
            )}

            {analysis && (
              <div style={{
                padding: 8, fontSize: 11, borderRadius: 4, lineHeight: 1.5,
                background: "rgba(137,180,250,0.05)", border: "1px solid rgba(137,180,250,0.2)",
                whiteSpace: "pre-wrap",
              }}>
                {analysis}
              </div>
            )}
          </div>
        )}

        {/* Done view */}
        {view === "done" && current && (
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            <div style={{
              padding: "12px 14px", borderRadius: 6, textAlign: "center",
              background: "rgba(243,139,168,0.1)", border: "1px solid rgba(243,139,168,0.3)",
            }}>
              <div style={{ fontSize: 13, fontWeight: 700, color: "var(--text-danger)" }}>Culprit Found!</div>
              <div style={{ fontFamily: "var(--font-mono)", fontSize: 14, color: "var(--text-warning)", marginTop: 6 }}>
                {current.culprit_commit || current.current_commit}
              </div>
              <div style={{ fontSize: 11, marginTop: 4, opacity: 0.7 }}>
                {current.commit_message}
              </div>
            </div>

            <button onClick={handleAnalyze} disabled={loading} style={{ ...btnStyle, color: "var(--text-info)" }}>
              {loading ? "Analyzing..." : "AI Root Cause Analysis"}
            </button>

            {analysis && (
              <div style={{
                padding: 8, fontSize: 11, borderRadius: 4, lineHeight: 1.5,
                background: "rgba(137,180,250,0.05)", border: "1px solid rgba(137,180,250,0.2)",
                whiteSpace: "pre-wrap",
              }}>
                {analysis}
              </div>
            )}

            <button onClick={handleReset} style={{ ...btnStyle, color: "var(--text-danger)" }}>Reset Bisect</button>
          </div>
        )}
      </div>
    </div>
  );
}

const btnStyle: React.CSSProperties = {
  padding: "4px 10px", fontSize: 11, fontWeight: 600,
  border: "1px solid var(--border-color)", borderRadius: 4,
  background: "var(--bg-secondary)", color: "var(--text-primary)",
  cursor: "pointer",
};

const inputStyle: React.CSSProperties = {
  padding: "6px 8px", fontSize: 12, borderRadius: 4,
  border: "1px solid var(--border-color)",
  background: "var(--bg-primary)", color: "var(--text-primary)",
  outline: "none", fontFamily: "var(--font-mono)",
};
