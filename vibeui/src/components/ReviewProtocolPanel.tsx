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

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12, marginRight: 8 };
const inputStyle: React.CSSProperties = { width: "100%", padding: "6px 8px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: 12, boxSizing: "border-box" };
const tabRow: React.CSSProperties = { display: "flex", gap: 4, marginBottom: 12 };
const metricBox: React.CSSProperties = { textAlign: "center", padding: 12, borderRadius: 6, background: "var(--bg-tertiary)", flex: 1 };

type Tab = "start" | "stats";

export default function ReviewProtocolPanel() {
  const [tab, setTab] = useState<Tab>("start");
  const [title, setTitle] = useState("");
  const [filesInput, setFilesInput] = useState("");
  const [sessionId, setSessionId] = useState("");
  const [stats, setStats] = useState<ReviewStats | null>(null);
  const [loading, setLoading] = useState(false);

  const doStart = useCallback(async () => {
    setLoading(true);
    try {
      const files = filesInput.split(",").map(f => f.trim()).filter(Boolean);
      const res = await invoke<{ sessionId: string; title: string }>("creview_start", { title, files: files.length ? files : ["."] });
      setSessionId(res.sessionId);
    } catch (e) { console.error(e); }
    setLoading(false);
  }, [title, filesInput]);

  const loadStats = useCallback(async () => {
    setLoading(true);
    try {
      const res = await invoke<ReviewStats>("creview_stats");
      setStats(res);
    } catch (e) { console.error(e); }
    setLoading(false);
  }, []);

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Collaborative Review Protocol</h2>
      <div style={tabRow}>
        {(["start", "stats"] as Tab[]).map(t => (
          <button key={t} style={{ ...btnStyle, background: tab === t ? "var(--accent-color)" : "var(--bg-tertiary)", color: tab === t ? "#fff" : "var(--text-primary)" }} onClick={() => { setTab(t); if (t === "stats") loadStats(); }}>
            {t === "start" ? "New Review" : "Quality Stats"}
          </button>
        ))}
      </div>

      {tab === "start" && (
        <>
          <div style={cardStyle}>
            <div style={labelStyle}>Review Title</div>
            <input value={title} onChange={e => setTitle(e.target.value)} style={{ ...inputStyle, marginBottom: 8 }} placeholder="Review: auth refactor" />
            <div style={labelStyle}>Files (comma-separated, leave empty for all)</div>
            <input value={filesInput} onChange={e => setFilesInput(e.target.value)} style={{ ...inputStyle, marginBottom: 8 }} placeholder="src/auth.rs, src/session.rs" />
            <button style={btnStyle} onClick={doStart} disabled={loading || !title}>
              {loading ? "..." : "Start Review"}
            </button>
          </div>

          {sessionId && (
            <div style={{ ...cardStyle, borderLeft: "3px solid #4caf50" }}>
              <div style={{ fontWeight: 600 }}>Review Started</div>
              <div style={labelStyle}>Session ID: {sessionId}</div>
              <div style={{ marginTop: 4, fontSize: 12 }}>
                Use <code>/creview comment file:line msg</code> in the terminal to add comments.
              </div>
            </div>
          )}
        </>
      )}

      {tab === "stats" && stats && (
        <>
          <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
            <div style={metricBox}>
              <div style={{ fontSize: 24, fontWeight: 700, color: "var(--text-primary)" }}>{stats.totalComments}</div>
              <div style={labelStyle}>Total Comments</div>
            </div>
            <div style={metricBox}>
              <div style={{ fontSize: 24, fontWeight: 700, color: "#4caf50" }}>{stats.resolved}</div>
              <div style={labelStyle}>Resolved</div>
            </div>
            <div style={metricBox}>
              <div style={{ fontSize: 24, fontWeight: 700, color: "#2196f3" }}>{stats.realIssues}</div>
              <div style={labelStyle}>Real Issues</div>
            </div>
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            <div style={metricBox}>
              <div style={{ fontSize: 24, fontWeight: 700, color: "#f44336" }}>{stats.falsePositives}</div>
              <div style={labelStyle}>False Positives</div>
            </div>
            <div style={metricBox}>
              <div style={{ fontSize: 24, fontWeight: 700, color: stats.precision >= 0.8 ? "#4caf50" : "#ff9800" }}>
                {(stats.precision * 100).toFixed(0)}%
              </div>
              <div style={labelStyle}>Precision</div>
            </div>
          </div>
        </>
      )}

      {tab === "stats" && !stats && !loading && <div style={labelStyle}>Loading quality metrics...</div>}
    </div>
  );
}
