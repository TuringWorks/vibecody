import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface RecordingFrame {
  path: string;
  timestamp: number;
  caption: string;
}

interface Recording {
  session_id: string;
  frames: RecordingFrame[];
  started_at: number;
  finished_at: number | null;
}

const badgeStyle: React.CSSProperties = {
  background: "var(--accent-color)",
  color: "var(--text-primary)",
  borderRadius: "var(--radius-md)",
  padding: "2px 8px",
  fontSize: "var(--font-size-sm)",
  fontWeight: 600,
  marginLeft: 8,
};

const sessionCardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  borderRadius: "var(--radius-sm)",
  padding: 10,
  marginBottom: 8,
  cursor: "pointer",
  border: "1px solid transparent",
  transition: "border-color 0.15s",
};

const frameRowStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 10,
  padding: "6px 0",
  borderBottom: "1px solid var(--border-color)",
};

const thumbStyle: React.CSSProperties = {
  width: 80,
  height: 50,
  objectFit: "cover",
  borderRadius: "var(--radius-xs-plus)",
  border: "1px solid var(--border-color)",
  background: "var(--bg-primary)",
};

function formatTs(ts: number): string {
  return new Date(ts * 1000).toLocaleString();
}

export function AgentRecordingPanel() {
  const [recordings, setRecordings] = useState<Recording[]>([]);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await invoke<Recording[]>("list_agent_recordings");
      setRecordings(list);
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  const toggle = (id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  return (
    <div className="panel-container">
      <div className="panel-header">
        <span style={{ fontWeight: 700, fontSize: "var(--font-size-xl)" }}>Agent Recordings</span>
        <button
          onClick={load}
          disabled={loading}
          className="panel-btn panel-btn-primary"
        >
          {loading ? "Loading..." : "Refresh"}
        </button>
      </div>

      {error && (
        <div className="panel-error">{error}</div>
      )}

      {!loading && recordings.length === 0 && (
        <div className="panel-empty">
          No recordings found. Use <code>--record</code> with the agent to capture sessions.
        </div>
      )}

      {recordings.map((rec) => (
        <div
          key={rec.session_id}
          style={{
            ...sessionCardStyle,
            borderColor: expanded.has(rec.session_id) ? "var(--accent-color)" : "transparent",
          }}
          onClick={() => toggle(rec.session_id)}
        >
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span style={{ fontWeight: 600 }}>{rec.session_id}</span>
            <span style={badgeStyle}>{rec.frames.length} frames</span>
          </div>
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 4 }}>
            {formatTs(rec.started_at)}
            {rec.finished_at && <span> — {formatTs(rec.finished_at)}</span>}
          </div>

          {expanded.has(rec.session_id) && (
            <div style={{ marginTop: 8 }}>
              {rec.frames.length === 0 && (
                <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>No frames captured.</div>
              )}
              {rec.frames.map((frame, i) => (
                <div key={i} style={frameRowStyle}>
                  <img
                    src={`asset://localhost/${frame.path}`}
                    alt={frame.caption}
                    style={thumbStyle}
                    onError={(e) => {
                      (e.target as HTMLImageElement).style.display = "none";
                    }}
                  />
                  <div>
                    <div style={{ fontWeight: 500, fontSize: "var(--font-size-base)" }}>{frame.caption}</div>
                    <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{formatTs(frame.timestamp)}</div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
