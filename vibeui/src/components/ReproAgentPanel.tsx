import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AgentSnapshot {
  session_id: string;
  fingerprint: string;
  created_at: string;
  label: string | null;
  tool_count: number;
  message_count: number;
  status: string;
}

interface DiffResult {
  session_id: string;
  reference_session_id: string | null;
  diff_lines: string[];
  summary: string;
  identical: boolean;
}

interface VerifyResult {
  trace_id: string;
  reference_hash: string;
  computed_hash: string;
  match: boolean;
  details: string;
}

type TagIntent = "info" | "success" | "warning" | "danger" | "neutral";

function statusIntent(s: string): TagIntent {
  switch (s) {
    case "completed": return "success";
    case "failed": return "danger";
    case "running": return "info";
    default: return "neutral";
  }
}

export function ReproAgentPanel() {
  const [tab, setTab] = useState<"sessions" | "replay" | "verify">("sessions");
  const [snapshots, setSnapshots] = useState<AgentSnapshot[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [replaySession, setReplaySession] = useState("");
  const [diffResult, setDiffResult] = useState<DiffResult | null>(null);
  const [replaying, setReplaying] = useState(false);
  const [traceId, setTraceId] = useState("");
  const [refHash, setRefHash] = useState("");
  const [verifyResult, setVerifyResult] = useState<VerifyResult | null>(null);
  const [verifying, setVerifying] = useState(false);

  useEffect(() => {
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const res = await invoke<AgentSnapshot[]>("repro_agent_snapshots");
        const snaps = Array.isArray(res) ? res : [];
        setSnapshots(snaps);
        if (snaps.length > 0) setReplaySession(snaps[0].session_id);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  async function runReplay() {
    if (!replaySession) return;
    setReplaying(true);
    setDiffResult(null);
    try {
      const res = await invoke<DiffResult>("repro_agent_diff", { sessionId: replaySession });
      setDiffResult(res ?? null);
    } catch (e) {
      setError(String(e));
    } finally {
      setReplaying(false);
    }
  }

  async function verify() {
    if (!traceId.trim() || !refHash.trim()) return;
    setVerifying(true);
    setVerifyResult(null);
    try {
      const res = await invoke<VerifyResult>("repro_agent_verify", { traceId: traceId.trim(), referenceHash: refHash.trim() });
      setVerifyResult(res ?? null);
    } catch (e) {
      setVerifyResult({ trace_id: traceId, reference_hash: refHash, computed_hash: "", match: false, details: String(e) });
    } finally {
      setVerifying(false);
    }
  }

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>Repro Agent</h3>
      </div>

      <div className="panel-body">
        <div className="panel-tab-bar" style={{ marginBottom: 12 }}>
          {(["sessions", "replay", "verify"] as const).map(t => (
            <button
              key={t}
              className={`panel-tab${tab === t ? " active" : ""}`}
              onClick={() => setTab(t)}
            >
              {t}
            </button>
          ))}
        </div>

        {loading && <div className="panel-loading">Loading…</div>}
        {error && (
          <div className="panel-error">
            <span>{error}</span>
            <button onClick={() => setError(null)} aria-label="dismiss">✕</button>
          </div>
        )}

        {!loading && tab === "sessions" && (
          snapshots.length === 0 ? (
            <div className="panel-empty">No snapshots found.</div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {snapshots.map(snap => (
                <div key={snap.session_id} className="panel-card">
                  <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                    <span style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>
                      {snap.label ?? snap.session_id.slice(0, 12) + "…"}
                    </span>
                    <span
                      className={`panel-tag panel-tag-${statusIntent(snap.status)}`}
                      style={{ marginLeft: "auto" }}
                    >
                      {snap.status}
                    </span>
                  </div>
                  <div style={{ display: "flex", gap: 14, fontSize: "var(--font-size-sm)", color: "var(--text-muted)", marginBottom: 6 }}>
                    <span>{snap.message_count} messages</span>
                    <span>{snap.tool_count} tool calls</span>
                    <span>{snap.created_at}</span>
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                    <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-muted)" }}>Fingerprint:</span>
                    <code style={{ fontSize: "var(--font-size-sm)", background: "var(--bg-primary)", padding: "1px 8px", borderRadius: "var(--radius-xs-plus)", color: "var(--accent-color)" }}>
                      {snap.fingerprint}
                    </code>
                  </div>
                </div>
              ))}
            </div>
          )
        )}

        {!loading && tab === "replay" && (
          <div style={{ maxWidth: 600 }}>
            <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 16 }}>
              <select
                className="panel-select"
                value={replaySession}
                onChange={e => setReplaySession(e.target.value)}
                style={{ flex: 1 }}
              >
                {snapshots.map(snap => (
                  <option key={snap.session_id} value={snap.session_id}>
                    {snap.label ?? snap.session_id.slice(0, 16) + "…"} ({snap.created_at})
                  </option>
                ))}
              </select>
              <button
                className="panel-btn panel-btn-primary"
                onClick={runReplay}
                disabled={replaying || !replaySession}
              >
                {replaying ? "Replaying…" : "Replay"}
              </button>
            </div>
            {diffResult && (
              <div className="panel-card">
                <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
                  <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>Diff Result</span>
                  <span className={`panel-tag panel-tag-${diffResult.identical ? "success" : "warning"}`}>
                    {diffResult.identical ? "Identical" : "Differences found"}
                  </span>
                </div>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 10 }}>
                  {diffResult.summary}
                </div>
                {diffResult.diff_lines.length > 0 && (
                  <pre style={{ background: "var(--bg-primary)", borderRadius: "var(--radius-sm)", padding: 12, fontSize: "var(--font-size-sm)", overflow: "auto", maxHeight: 300, margin: 0, color: "var(--text-primary)" }}>
                    {diffResult.diff_lines.map((line, i) => {
                      const color = line.startsWith("+")
                        ? "var(--success-color)"
                        : line.startsWith("-")
                          ? "var(--error-color)"
                          : "var(--text-primary)";
                      return <span key={i} style={{ display: "block", color }}>{line}</span>;
                    })}
                  </pre>
                )}
              </div>
            )}
          </div>
        )}

        {!loading && tab === "verify" && (
          <div style={{ maxWidth: 520 }}>
            <div style={{ marginBottom: 14 }}>
              <label className="panel-label" style={{ display: "block" }}>Trace ID</label>
              <input
                className="panel-input panel-input-full"
                value={traceId}
                onChange={e => setTraceId(e.target.value)}
                placeholder="trace_abc123..."
                style={{ fontFamily: "var(--font-mono)" }}
              />
            </div>
            <div style={{ marginBottom: 16 }}>
              <label className="panel-label" style={{ display: "block" }}>Reference Hash</label>
              <input
                className="panel-input panel-input-full"
                value={refHash}
                onChange={e => setRefHash(e.target.value)}
                placeholder="sha256:..."
                style={{ fontFamily: "var(--font-mono)" }}
              />
            </div>
            <button
              className="panel-btn panel-btn-primary"
              onClick={verify}
              disabled={verifying || !traceId.trim() || !refHash.trim()}
              style={{ marginBottom: 20 }}
            >
              {verifying ? "Verifying…" : "Verify"}
            </button>
            {verifyResult && (
              <div
                className="panel-card"
                style={{ borderColor: verifyResult.match ? "var(--success-color)" : "var(--error-color)" }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
                  <span className={`panel-tag panel-tag-${verifyResult.match ? "success" : "danger"}`}>
                    {verifyResult.match ? "Hash Match" : "Hash Mismatch"}
                  </span>
                </div>
                <div style={{ display: "grid", gridTemplateColumns: "140px 1fr", rowGap: 8, fontSize: "var(--font-size-base)" }}>
                  <span style={{ color: "var(--text-muted)" }}>Trace ID</span>
                  <code style={{ color: "var(--text-primary)", wordBreak: "break-all" }}>{verifyResult.trace_id}</code>
                  <span style={{ color: "var(--text-muted)" }}>Reference</span>
                  <code style={{ color: "var(--text-primary)", wordBreak: "break-all" }}>{verifyResult.reference_hash}</code>
                  <span style={{ color: "var(--text-muted)" }}>Computed</span>
                  <code style={{ color: verifyResult.match ? "var(--success-color)" : "var(--error-color)", wordBreak: "break-all" }}>
                    {verifyResult.computed_hash || "—"}
                  </code>
                </div>
                <div style={{ marginTop: 10, fontSize: "var(--font-size-base)", color: "var(--text-muted)" }}>
                  {verifyResult.details}
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
