import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface BridgeStatus {
  connected: boolean;
  socket_path: string;
  ide_name: string | null;
  ide_version: string | null;
  pid: number | null;
}

interface BridgeContext {
  open_files: string[];
  active_file: string | null;
  active_selection: { start_line: number; end_line: number; text: string } | null;
  test_results: { passed: number; failed: number; skipped: number; last_run: string } | null;
  workspace_root: string | null;
}

interface SyncInfo {
  last_sync_at: string | null;
  pending_changes: number;
  sync_status: string;
}

export function IdeBridgePanel() {
  const [tab, setTab] = useState("status");
  const [bridgeStatus, setBridgeStatus] = useState<BridgeStatus | null>(null);
  const [bridgeContext, setBridgeContext] = useState<BridgeContext | null>(null);
  const [syncInfo, setSyncInfo] = useState<SyncInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [connecting, setConnecting] = useState(false);
  const [syncing, setSyncing] = useState(false);

  useEffect(() => {
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const [statusRes, contextRes, syncRes] = await Promise.all([
          invoke<BridgeStatus>("ide_bridge_status"),
          invoke<BridgeContext>("ide_bridge_context"),
          invoke<SyncInfo>("ide_bridge_sync"),
        ]);
        setBridgeStatus(statusRes ?? null);
        setBridgeContext(contextRes ?? null);
        setSyncInfo(syncRes ?? null);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  async function connect() {
    setConnecting(true);
    try {
      const res = await invoke<BridgeStatus>("ide_bridge_status", { action: "connect" });
      setBridgeStatus(res ?? null);
    } catch (e) {
      setError(String(e));
    } finally {
      setConnecting(false);
    }
  }

  async function forceSync() {
    setSyncing(true);
    try {
      const res = await invoke<SyncInfo>("ide_bridge_sync", { force: true });
      setSyncInfo(res ?? null);
    } catch (e) {
      setError(String(e));
    } finally {
      setSyncing(false);
    }
  }

  return (
    <div style={{ padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono)", height: "100%", overflowY: "auto" }}>
      <div style={{ fontSize: 15, fontWeight: 700, marginBottom: 12 }}>IDE Bridge</div>

      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        {["status", "context", "sync"].map(t => (
          <button key={t} onClick={() => setTab(t)} style={{ padding: "4px 12px", borderRadius: 6, cursor: "pointer", background: tab === t ? "var(--accent-color)" : "var(--bg-secondary)", color: tab === t ? "#fff" : "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: 12 }}>{t}</button>
        ))}
      </div>

      {loading && <div style={{ color: "var(--text-muted)" }}>Loading...</div>}
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8 }}>{error}</div>}

      {!loading && tab === "status" && (
        <div style={{ maxWidth: 480 }}>
          <div style={{ background: "var(--bg-secondary)", borderRadius: 10, border: "1px solid var(--border-color)", padding: 18, marginBottom: 16 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 14 }}>
              <div style={{ width: 12, height: 12, borderRadius: "50%", background: bridgeStatus?.connected ? "var(--success-color)" : "var(--error-color)" }} />
              <span style={{ fontSize: 14, fontWeight: 700, color: bridgeStatus?.connected ? "var(--success-color)" : "var(--error-color)" }}>
                {bridgeStatus?.connected ? "Connected" : "Disconnected"}
              </span>
              {bridgeStatus?.ide_name && (
                <span style={{ fontSize: 12, color: "var(--text-muted)", marginLeft: "auto" }}>
                  {bridgeStatus.ide_name} {bridgeStatus.ide_version}
                </span>
              )}
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "110px 1fr", rowGap: 8, fontSize: 12 }}>
              <span style={{ color: "var(--text-muted)" }}>Socket Path</span>
              <code style={{ color: "var(--text-primary)", wordBreak: "break-all" }}>{bridgeStatus?.socket_path ?? "—"}</code>
              {bridgeStatus?.pid && (
                <>
                  <span style={{ color: "var(--text-muted)" }}>PID</span>
                  <span>{bridgeStatus.pid}</span>
                </>
              )}
            </div>
          </div>
          <button onClick={connect} disabled={connecting || bridgeStatus?.connected}
            style={{ padding: "8px 24px", borderRadius: 6, cursor: connecting || bridgeStatus?.connected ? "not-allowed" : "pointer", background: bridgeStatus?.connected ? "var(--bg-secondary)" : "var(--accent-color)", color: bridgeStatus?.connected ? "var(--text-muted)" : "#fff", border: "1px solid var(--border-color)", fontSize: 13, fontWeight: 600, opacity: connecting ? 0.6 : 1 }}>
            {connecting ? "Connecting…" : bridgeStatus?.connected ? "Already Connected" : "Connect"}
          </button>
        </div>
      )}

      {!loading && tab === "context" && (
        <div>
          {!bridgeContext && <div style={{ color: "var(--text-muted)" }}>No context available. Connect to an IDE first.</div>}
          {bridgeContext && (
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              {bridgeContext.workspace_root && (
                <div style={{ background: "var(--bg-secondary)", borderRadius: 8, border: "1px solid var(--border-color)", padding: 12 }}>
                  <div style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 4 }}>Workspace Root</div>
                  <code style={{ fontSize: 12, color: "var(--text-primary)" }}>{bridgeContext.workspace_root}</code>
                </div>
              )}
              <div style={{ background: "var(--bg-secondary)", borderRadius: 8, border: "1px solid var(--border-color)", padding: 12 }}>
                <div style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 8 }}>Open Files ({bridgeContext.open_files.length})</div>
                {bridgeContext.open_files.length === 0 && <div style={{ color: "var(--text-muted)", fontSize: 12 }}>None</div>}
                <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
                  {bridgeContext.open_files.map((f, i) => (
                    <div key={i} style={{ display: "flex", alignItems: "center", gap: 8 }}>
                      <code style={{ fontSize: 11, color: f === bridgeContext.active_file ? "var(--accent-color)" : "var(--text-primary)" }}>{f}</code>
                      {f === bridgeContext.active_file && <span style={{ fontSize: 10, padding: "1px 6px", borderRadius: 6, background: "var(--accent-color)22", color: "var(--accent-color)" }}>active</span>}
                    </div>
                  ))}
                </div>
              </div>
              {bridgeContext.active_selection && (
                <div style={{ background: "var(--bg-secondary)", borderRadius: 8, border: "1px solid var(--border-color)", padding: 12 }}>
                  <div style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 6 }}>
                    Active Selection (lines {bridgeContext.active_selection.start_line}–{bridgeContext.active_selection.end_line})
                  </div>
                  <pre style={{ margin: 0, fontSize: 11, color: "var(--text-primary)", background: "var(--bg-primary)", borderRadius: 6, padding: "8px 10px", overflow: "auto", maxHeight: 120 }}>
                    {bridgeContext.active_selection.text}
                  </pre>
                </div>
              )}
              {bridgeContext.test_results && (
                <div style={{ background: "var(--bg-secondary)", borderRadius: 8, border: "1px solid var(--border-color)", padding: 12 }}>
                  <div style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 8 }}>
                    Test Results <span style={{ color: "var(--text-muted)", fontSize: 10 }}>({bridgeContext.test_results.last_run})</span>
                  </div>
                  <div style={{ display: "flex", gap: 16 }}>
                    <span style={{ fontSize: 13, color: "var(--success-color)" }}>{bridgeContext.test_results.passed} passed</span>
                    <span style={{ fontSize: 13, color: "var(--error-color)" }}>{bridgeContext.test_results.failed} failed</span>
                    <span style={{ fontSize: 13, color: "var(--text-muted)" }}>{bridgeContext.test_results.skipped} skipped</span>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
      )}

      {!loading && tab === "sync" && (
        <div style={{ maxWidth: 480 }}>
          <div style={{ background: "var(--bg-secondary)", borderRadius: 10, border: "1px solid var(--border-color)", padding: 18, marginBottom: 16 }}>
            <div style={{ display: "grid", gridTemplateColumns: "140px 1fr", rowGap: 10, fontSize: 12 }}>
              <span style={{ color: "var(--text-muted)" }}>Last Sync</span>
              <span>{syncInfo?.last_sync_at ?? "Never"}</span>
              <span style={{ color: "var(--text-muted)" }}>Sync Status</span>
              <span style={{ color: syncInfo?.sync_status === "synced" ? "var(--success-color)" : syncInfo?.sync_status === "pending" ? "var(--warning-color)" : "var(--error-color)" }}>
                {syncInfo?.sync_status ?? "unknown"}
              </span>
              <span style={{ color: "var(--text-muted)" }}>Pending Changes</span>
              <span>{syncInfo?.pending_changes ?? 0}</span>
            </div>
          </div>
          <button onClick={forceSync} disabled={syncing}
            style={{ padding: "8px 24px", borderRadius: 6, cursor: syncing ? "not-allowed" : "pointer", background: "var(--accent-color)", color: "#fff", border: "none", fontSize: 13, fontWeight: 600, opacity: syncing ? 0.6 : 1 }}>
            {syncing ? "Syncing…" : "Force Sync"}
          </button>
        </div>
      )}
    </div>
  );
}
