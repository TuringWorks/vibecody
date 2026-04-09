/**
 * ReviewProtocolPanel — Collaborative Code Review dashboard.
 *
 * Start review sessions, view quality metrics (precision, false positive rate),
 * and manage multi-round collaborative reviews with comment tracking.
 */
import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ReviewStats {
  totalComments: number;
  resolved: number;
  realIssues: number;
  falsePositives: number;
  precision: number;
}

type Tab = "start" | "stats";

export default function ReviewProtocolPanel() {
  const [tab, setTab] = useState<Tab>("start");
  const [title, setTitle] = useState("");
  const [filesInput, setFilesInput] = useState("");
  const [sessionId, setSessionId] = useState("");
  const [stats, setStats] = useState<ReviewStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [cliOutput, setCliOutput] = useState("");

  const runCreview = useCallback(async (args: string) => {
    setCliOutput("");
    try {
      const res = await invoke<string>("handle_creview_command", { args });
      setCliOutput(res);
    } catch (e) { setCliOutput(`Error: ${e}`); }
  }, []);

  const doStart = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const files = filesInput.split(",").map(f => f.trim()).filter(Boolean);
      const res = await invoke<{ sessionId: string; title: string }>(
        "creview_start",
        { title, files: files.length ? files : ["."] }
      );
      setSessionId(res.sessionId);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [title, filesInput]);

  const loadStats = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const res = await invoke<ReviewStats>("creview_stats");
      setStats(res);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const precisionColor = (p: number) =>
    p >= 0.8 ? "var(--success-color)" : p >= 0.6 ? "var(--warning-color)" : "var(--error-color)";

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>Collaborative Review Protocol</h3>
        <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
          {(["start", "stats"] as Tab[]).map(t => (
            <button
              key={t}
              className={`panel-btn ${tab === t ? "panel-btn-primary" : "panel-btn-secondary"}`}
              onClick={() => {
                setTab(t);
                if (t === "stats" && !stats) loadStats();
              }}
            >
              {t === "start" ? "New Review" : "Quality Stats"}
            </button>
          ))}
        </div>
      </div>

      <div className="panel-body">
        {error && (
          <div className="panel-error" style={{ marginBottom: 10 }}>
            {error}
            <button onClick={() => setError("")}>✕</button>
          </div>
        )}

        {tab === "start" && (
          <>
            <div className="panel-card" style={{ marginBottom: 10 }}>
              <div className="panel-label">Review Title</div>
              <input
                className="panel-input panel-input-full"
                value={title}
                onChange={e => setTitle(e.target.value)}
                placeholder="Review: auth refactor"
                style={{ marginBottom: 8 }}
              />
              <div className="panel-label">Files (comma-separated, leave empty for all)</div>
              <input
                className="panel-input panel-input-full"
                value={filesInput}
                onChange={e => setFilesInput(e.target.value)}
                placeholder="src/auth.rs, src/session.rs"
                style={{ marginBottom: 8 }}
              />
              <button
                className="panel-btn panel-btn-primary"
                onClick={doStart}
                disabled={loading || !title}
              >
                {loading ? "Starting…" : "Start Review"}
              </button>
            </div>

            {sessionId && (
              <div className="panel-card" style={{ borderLeft: "3px solid var(--success-color)" }}>
                <div style={{ fontWeight: "var(--font-semibold)", marginBottom: 4 }}>Review Started</div>
                <div className="panel-label" style={{ marginBottom: 6 }}>Session: {sessionId}</div>
                <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginTop: 4 }}>
                  <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => runCreview(`list ${sessionId}`)} title='vibecli --cmd "/creview list"'>▶ List Comments</button>
                  <button className="panel-btn panel-btn-secondary panel-btn-sm" onClick={() => runCreview(`summary ${sessionId}`)} title='vibecli --cmd "/creview summary"'>▶ Summary</button>
                </div>
                {cliOutput && <pre style={{ whiteSpace: "pre-wrap", marginTop: 8, fontSize: 11 }}>{cliOutput}</pre>}
              </div>
            )}

            {!sessionId && !loading && !error && (
              <div className="panel-empty">Enter a review title above and click Start Review.</div>
            )}
          </>
        )}

        {tab === "stats" && loading && <div className="panel-loading">Loading quality metrics…</div>}

        {tab === "stats" && stats && !loading && (
          <>
            <div className="panel-stats" style={{ marginBottom: 8 }}>
              <div className="panel-stat">
                <div className="panel-stat-value">{stats.totalComments}</div>
                <div className="panel-stat-label">Total</div>
              </div>
              <div className="panel-stat">
                <div className="panel-stat-value" style={{ color: "var(--success-color)" }}>{stats.resolved}</div>
                <div className="panel-stat-label">Resolved</div>
              </div>
              <div className="panel-stat">
                <div className="panel-stat-value" style={{ color: "var(--info-color)" }}>{stats.realIssues}</div>
                <div className="panel-stat-label">Real Issues</div>
              </div>
            </div>
            <div className="panel-stats">
              <div className="panel-stat">
                <div className="panel-stat-value" style={{ color: "var(--error-color)" }}>{stats.falsePositives}</div>
                <div className="panel-stat-label">False +</div>
              </div>
              <div className="panel-stat">
                <div className="panel-stat-value" style={{ color: precisionColor(stats.precision) }}>
                  {(stats.precision * 100).toFixed(0)}%
                </div>
                <div className="panel-stat-label">Precision</div>
              </div>
            </div>

            <div className="panel-card" style={{ marginTop: 10 }}>
              <div className="panel-label" style={{ marginBottom: 6 }}>Precision trend</div>
              <div className="progress-bar progress-bar-lg">
                <div
                  className="progress-bar-fill"
                  style={{ width: `${stats.precision * 100}%`, background: precisionColor(stats.precision) }}
                />
              </div>
            </div>
          </>
        )}

        {tab === "stats" && !stats && !loading && !error && (
          <div className="panel-empty">Click Quality Stats to load review metrics.</div>
        )}
      </div>
    </div>
  );
}
