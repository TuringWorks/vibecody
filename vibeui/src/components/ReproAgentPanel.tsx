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

export function ReproAgentPanel() {
  const [tab, setTab] = useState("sessions");
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

  const statusColor = (s: string) => {
    if (s === "completed") return "var(--success-color)";
    if (s === "failed") return "var(--error-color)";
    if (s === "running") return "var(--accent-color)";
    return "var(--text-muted)";
  };

  return (
    <div style={{ padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono)", height: "100%", overflowY: "auto" }}>
      <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 700, marginBottom: 12 }}>Repro Agent</div>

      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        {["sessions", "replay", "verify"].map(t => (
          <button key={t} onClick={() => setTab(t)} style={{ padding: "4px 12px", borderRadius: "var(--radius-sm)", cursor: "pointer", background: tab === t ? "var(--accent-color)" : "var(--bg-secondary)", color: tab === t ? "#fff" : "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)" }}>{t}</button>
        ))}
      </div>

      {loading && <div style={{ color: "var(--text-muted)" }}>Loading...</div>}
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8 }}>{error}</div>}

      {!loading && tab === "sessions" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {snapshots.length === 0 && <div style={{ color: "var(--text-muted)" }}>No snapshots found.</div>}
          {snapshots.map(snap => (
            <div key={snap.session_id} style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", border: "1px solid var(--border-color)", padding: "12px 14px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                <span style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>{snap.label ?? snap.session_id.slice(0, 12) + "…"}</span>
                <span style={{ fontSize: "var(--font-size-sm)", padding: "2px 10px", borderRadius: "var(--radius-md)", background: statusColor(snap.status) + "22", color: statusColor(snap.status), marginLeft: "auto" }}>{snap.status}</span>
              </div>
              <div style={{ display: "flex", gap: 14, fontSize: "var(--font-size-sm)", color: "var(--text-muted)", marginBottom: 6 }}>
                <span>{snap.message_count} messages</span>
                <span>{snap.tool_count} tool calls</span>
                <span>{snap.created_at}</span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-muted)" }}>Fingerprint:</span>
                <code style={{ fontSize: "var(--font-size-sm)", background: "var(--bg-primary)", padding: "1px 8px", borderRadius: "var(--radius-xs-plus)", color: "var(--accent-color)" }}>{snap.fingerprint}</code>
              </div>
            </div>
          ))}
        </div>
      )}

      {!loading && tab === "replay" && (
        <div style={{ maxWidth: 600 }}>
          <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 16 }}>
            <select value={replaySession} onChange={e => setReplaySession(e.target.value)}
              style={{ flex: 1, padding: "6px 10px", borderRadius: "var(--radius-sm)", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)" }}>
              {snapshots.map(snap => (
                <option key={snap.session_id} value={snap.session_id}>
                  {snap.label ?? snap.session_id.slice(0, 16) + "…"} ({snap.created_at})
                </option>
              ))}
            </select>
            <button onClick={runReplay} disabled={replaying || !replaySession}
              style={{ padding: "6px 18px", borderRadius: "var(--radius-sm)", cursor: replaying || !replaySession ? "not-allowed" : "pointer", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontSize: "var(--font-size-base)", fontWeight: 600, opacity: replaying || !replaySession ? 0.6 : 1 }}>
              {replaying ? "Replaying…" : "Replay"}
            </button>
          </div>
          {diffResult && (
            <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-md)", border: "1px solid var(--border-color)", padding: 16 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
                <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>Diff Result</span>
                <span style={{ fontSize: "var(--font-size-base)", padding: "2px 10px", borderRadius: "var(--radius-sm-alt)", background: diffResult.identical ? "var(--success-color)22" : "var(--warning-color)22", color: diffResult.identical ? "var(--success-color)" : "var(--warning-color)" }}>
                  {diffResult.identical ? "Identical" : "Differences found"}
                </span>
              </div>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 10 }}>{diffResult.summary}</div>
              {diffResult.diff_lines.length > 0 && (
                <pre style={{ background: "var(--bg-primary)", borderRadius: "var(--radius-sm)", padding: "10px 12px", fontSize: "var(--font-size-sm)", overflow: "auto", maxHeight: 300, margin: 0, color: "var(--text-primary)" }}>
                  {diffResult.diff_lines.map((line, i) => {
                    const color = line.startsWith("+") ? "var(--success-color)" : line.startsWith("-") ? "var(--error-color)" : "var(--text-primary)";
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
            <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 6 }}>Trace ID</label>
            <input value={traceId} onChange={e => setTraceId(e.target.value)}
              placeholder="trace_abc123..."
              style={{ width: "100%", padding: "7px 10px", borderRadius: "var(--radius-sm)", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)", boxSizing: "border-box", fontFamily: "var(--font-mono)" }} />
          </div>
          <div style={{ marginBottom: 16 }}>
            <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 6 }}>Reference Hash</label>
            <input value={refHash} onChange={e => setRefHash(e.target.value)}
              placeholder="sha256:..."
              style={{ width: "100%", padding: "7px 10px", borderRadius: "var(--radius-sm)", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)", boxSizing: "border-box", fontFamily: "var(--font-mono)" }} />
          </div>
          <button onClick={verify} disabled={verifying || !traceId.trim() || !refHash.trim()}
            style={{ padding: "8px 24px", borderRadius: "var(--radius-sm)", cursor: verifying || !traceId.trim() || !refHash.trim() ? "not-allowed" : "pointer", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontSize: "var(--font-size-md)", fontWeight: 600, opacity: verifying || !traceId.trim() || !refHash.trim() ? 0.6 : 1, marginBottom: 20 }}>
            {verifying ? "Verifying…" : "Verify"}
          </button>
          {verifyResult && (
            <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-md)", border: `1px solid ${verifyResult.match ? "var(--success-color)" : "var(--error-color)"}`, padding: 16 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
                <span style={{ fontWeight: 700, fontSize: "var(--font-size-lg)", color: verifyResult.match ? "var(--success-color)" : "var(--error-color)" }}>
                  {verifyResult.match ? "Hash Match" : "Hash Mismatch"}
                </span>
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "140px 1fr", rowGap: 8, fontSize: "var(--font-size-base)" }}>
                <span style={{ color: "var(--text-muted)" }}>Trace ID</span>
                <code style={{ color: "var(--text-primary)", wordBreak: "break-all" }}>{verifyResult.trace_id}</code>
                <span style={{ color: "var(--text-muted)" }}>Reference</span>
                <code style={{ color: "var(--text-primary)", wordBreak: "break-all" }}>{verifyResult.reference_hash}</code>
                <span style={{ color: "var(--text-muted)" }}>Computed</span>
                <code style={{ color: verifyResult.match ? "var(--success-color)" : "var(--error-color)", wordBreak: "break-all" }}>{verifyResult.computed_hash || "—"}</code>
              </div>
              <div style={{ marginTop: 10, fontSize: "var(--font-size-base)", color: "var(--text-muted)" }}>{verifyResult.details}</div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
