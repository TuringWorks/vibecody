/**
 * AcpPanel — Agent Client Protocol panel.
 *
 * Manage ACP server/client connections, view registered capabilities
 * and tools, and inspect protocol messages.
 * Wired to Tauri backend commands persisted at ~/.vibeui/acp-state.json.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types ─────────────────────────────────────────────────────────────────────

interface AcpCapability {
  id: string;
  name: string;
  type: "tool" | "resource" | "prompt";
  description: string;
  version: string;
}

interface AcpMessage {
  id: string;
  timestamp: string;
  direction: "sent" | "received";
  method: string;
  status: "ok" | "error" | "pending";
  payload: string;
}

interface AcpStatus {
  running: boolean;
  version: string;
  mode: string;
  connected_clients: number;
  capability_count: number;
  message_count: number;
}

// ── Styles ────────────────────────────────────────────────────────────────────

const inputStyle: React.CSSProperties = { width: "100%", padding: "6px 10px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontSize: 12, boxSizing: "border-box" };
const badgeStyle = (variant: string): React.CSSProperties => {
  const colors: Record<string, string> = { tool: "var(--accent-color)", resource: "var(--accent-purple)", prompt: "var(--warning-color)", ok: "var(--success-color)", error: "var(--error-color)", pending: "var(--text-secondary)", sent: "var(--accent-color)", received: "var(--accent-purple)" };
  return { display: "inline-block", padding: "2px 8px", borderRadius: 10, fontSize: 10, fontWeight: 600, color: "var(--btn-primary-fg)", background: colors[variant] || "var(--text-secondary)" };
};

// ── Component ─────────────────────────────────────────────────────────────────

type Tab = "server" | "client" | "protocol";

export function AcpPanel() {
  const [tab, setTab] = useState<Tab>("server");
  const [status, setStatus] = useState<AcpStatus | null>(null);
  const [capabilities, setCapabilities] = useState<AcpCapability[]>([]);
  const [messages, setMessages] = useState<AcpMessage[]>([]);
  const [clientUrl, setClientUrl] = useState("http://localhost:3001/acp");
  const [clientConnected, setClientConnected] = useState(false);
  const [negotiationStatus, setNegotiationStatus] = useState<"idle" | "connecting" | "connected" | "failed">("idle");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // ── Register capability form state ──────────────────────────────────────
  const [newCapName, setNewCapName] = useState("");
  const [newCapType, setNewCapType] = useState<"tool" | "resource" | "prompt">("tool");
  const [newCapDesc, setNewCapDesc] = useState("");
  const [newCapVersion, setNewCapVersion] = useState("1.0.0");

  const fetchStatus = useCallback(async () => {
    try {
      const res = await invoke<AcpStatus>("get_acp_status");
      setStatus(res);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const fetchCapabilities = useCallback(async () => {
    try {
      const res = await invoke<{ capabilities: AcpCapability[] }>("get_acp_capabilities");
      setCapabilities(res.capabilities);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const fetchMessages = useCallback(async () => {
    try {
      const res = await invoke<{ messages: AcpMessage[] }>("get_acp_messages");
      setMessages(res.messages);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const loadAll = useCallback(async () => {
    setLoading(true);
    setError(null);
    await Promise.all([fetchStatus(), fetchCapabilities(), fetchMessages()]);
    setLoading(false);
  }, [fetchStatus, fetchCapabilities, fetchMessages]);

  useEffect(() => {
    loadAll();
  }, [loadAll]);

  const handleToggleServer = async () => {
    try {
      const res = await invoke<{ running: boolean; action: string }>("toggle_acp_server");
      setStatus((prev) => prev ? { ...prev, running: res.running, connected_clients: res.running ? prev.connected_clients : 0 } : prev);
      await fetchMessages();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleRegisterCapability = async () => {
    if (!newCapName.trim()) return;
    try {
      await invoke("register_acp_capability", {
        name: newCapName.trim(),
        capType: newCapType,
        description: newCapDesc.trim() || newCapName.trim(),
        version: newCapVersion.trim() || "1.0.0",
      });
      setNewCapName("");
      setNewCapDesc("");
      setNewCapVersion("1.0.0");
      await Promise.all([fetchCapabilities(), fetchMessages(), fetchStatus()]);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleSendMessage = async (method: string, payload: string) => {
    try {
      await invoke("send_acp_message", { method, payload });
      await Promise.all([fetchMessages(), fetchStatus()]);
    } catch (e) {
      setError(String(e));
    }
  };

  const connectClient = async () => {
    setNegotiationStatus("connecting");
    try {
      await handleSendMessage("initialize", JSON.stringify({ protocolVersion: "1.0", capabilities: { tools: true }, clientUrl }));
      setNegotiationStatus("connected");
      setClientConnected(true);
    } catch {
      setNegotiationStatus("failed");
    }
  };

  const disconnectClient = () => {
    setClientConnected(false);
    setNegotiationStatus("idle");
  };

  const serverRunning = status?.running ?? false;
  const toolCount = capabilities.filter((c) => c.type === "tool").length;
  const resourceCount = capabilities.filter((c) => c.type === "resource").length;
  const promptCount = capabilities.filter((c) => c.type === "prompt").length;

  if (loading) {
    return (
      <div className="panel-container">
        <h2>Agent Client Protocol (ACP)</h2>
        <div className="panel-loading">Loading...</div>
      </div>
    );
  }

  return (
    <div className="panel-container">
      <h2 style={{ margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" }}>Agent Client Protocol (ACP)</h2>

      {error && (
        <div className="panel-error" style={{ marginBottom: 12 }}>
          {error}
          <button className="panel-btn panel-btn-secondary" style={{ marginLeft: 8, fontSize: 10 }} onClick={() => setError(null)}>Dismiss</button>
        </div>
      )}

      <div className="panel-tab-bar" style={{ marginBottom: 12 }}>
        <button className={`panel-tab ${tab === "server" ? "active" : ""}`} onClick={() => setTab("server")}>Server</button>
        <button className={`panel-tab ${tab === "client" ? "active" : ""}`} onClick={() => setTab("client")}>Client</button>
        <button className={`panel-tab ${tab === "protocol" ? "active" : ""}`} onClick={() => setTab("protocol")}>Protocol</button>
      </div>

      {tab === "server" && (
        <div>
          <div className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <div>
              <div style={{ fontWeight: 600 }}>ACP Server</div>
              <div style={{ fontSize: 11, color: serverRunning ? "var(--success-color)" : "var(--text-secondary)" }}>
                {serverRunning ? "Running on localhost:7878/acp" : "Stopped"}
              </div>
              {status && <div style={{ fontSize: 10, color: "var(--text-secondary)" }}>v{status.version} | {status.connected_clients} client(s)</div>}
            </div>
            <button
              className={serverRunning ? "panel-btn panel-btn-danger" : "panel-btn panel-btn-primary"}
              onClick={handleToggleServer}
            >
              {serverRunning ? "Stop Server" : "Start Server"}
            </button>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10, marginBottom: 12 }}>
            <div className="panel-card">
              <div className="panel-label">Tools</div>
              <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{toolCount}</div>
            </div>
            <div className="panel-card">
              <div className="panel-label">Resources</div>
              <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{resourceCount}</div>
            </div>
            <div className="panel-card">
              <div className="panel-label">Prompts</div>
              <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{promptCount}</div>
            </div>
          </div>

          <div className="panel-label">Registered Capabilities</div>
          {capabilities.map((cap) => (
            <div key={cap.id} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <span style={{ fontWeight: 600 }}>{cap.name}</span>{" "}
                <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>v{cap.version}</span>
                <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 2 }}>{cap.description}</div>
              </div>
              <span style={badgeStyle(cap.type)}>{cap.type}</span>
            </div>
          ))}

          {/* Register new capability form */}
          <div className="panel-card" style={{ marginTop: 12 }}>
            <div className="panel-label" style={{ fontWeight: 600, marginBottom: 8 }}>Register New Capability</div>
            <div style={{ display: "flex", gap: 6, marginBottom: 6 }}>
              <input style={{ ...inputStyle, flex: 2 }} placeholder="Name" value={newCapName} onChange={(e) => setNewCapName(e.target.value)} />
              <select
                style={{ ...inputStyle, flex: 1 }}
                value={newCapType}
                onChange={(e) => setNewCapType(e.target.value as "tool" | "resource" | "prompt")}
              >
                <option value="tool">tool</option>
                <option value="resource">resource</option>
                <option value="prompt">prompt</option>
              </select>
              <input style={{ ...inputStyle, flex: 1 }} placeholder="Version" value={newCapVersion} onChange={(e) => setNewCapVersion(e.target.value)} />
            </div>
            <div style={{ display: "flex", gap: 6 }}>
              <input style={{ ...inputStyle, flex: 1 }} placeholder="Description" value={newCapDesc} onChange={(e) => setNewCapDesc(e.target.value)} />
              <button className="panel-btn panel-btn-primary" onClick={handleRegisterCapability}>Register</button>
            </div>
          </div>
        </div>
      )}

      {tab === "client" && (
        <div>
          <div className="panel-card">
            <div className="panel-label">External ACP Server URL</div>
            <div style={{ display: "flex", gap: 8 }}>
              <input style={{ ...inputStyle, flex: 1 }} value={clientUrl} onChange={(e) => setClientUrl(e.target.value)} placeholder="http://localhost:3001/acp" />
              {!clientConnected ? (
                <button className="panel-btn panel-btn-primary" onClick={connectClient}>Connect</button>
              ) : (
                <button className="panel-btn panel-btn-danger" onClick={disconnectClient}>Disconnect</button>
              )}
            </div>
          </div>

          <div className="panel-card">
            <div className="panel-label">Negotiation Status</div>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 4 }}>
              <span style={badgeStyle(negotiationStatus === "connected" ? "ok" : negotiationStatus === "failed" ? "error" : "pending")}>
                {negotiationStatus}
              </span>
              {negotiationStatus === "connected" && <span style={{ fontSize: 11 }}>Protocol v1.0 negotiated</span>}
            </div>
          </div>

          {clientConnected && (
            <div className="panel-card">
              <div className="panel-label">Remote Server Capabilities</div>
              <div style={{ fontSize: 12, marginTop: 4 }}>
                <div>Tools: {capabilities.filter((c) => c.type === "tool").map((c) => c.name).join(", ")}</div>
                <div>Resources: {capabilities.filter((c) => c.type === "resource").map((c) => c.name).join(", ")}</div>
                <div>Prompts: {capabilities.filter((c) => c.type === "prompt").map((c) => c.name).join(", ")}</div>
              </div>
            </div>
          )}
        </div>
      )}

      {tab === "protocol" && (
        <div>
          <div className="panel-card" style={{ display: "flex", justifyContent: "space-between" }}>
            <div>
              <div style={{ fontWeight: 600 }}>ACP Protocol</div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>Based on Model Context Protocol (MCP) specification</div>
            </div>
            <div style={{ textAlign: "right" }}>
              <div className="panel-label">Version</div>
              <div style={{ fontWeight: 600 }}>{status?.version ?? "1.0.0"}</div>
            </div>
          </div>

          {/* Quick-send message controls */}
          <div className="panel-card" style={{ display: "flex", gap: 6, alignItems: "center" }}>
            <span className="panel-label">Quick Send:</span>
            <button className="panel-btn panel-btn-secondary" onClick={() => handleSendMessage("initialize", "{}")}>initialize</button>
            <button className="panel-btn panel-btn-secondary" onClick={() => handleSendMessage("tools/list", "{}")}>tools/list</button>
            <button className="panel-btn panel-btn-secondary" onClick={() => handleSendMessage("resources/list", "{}")}>resources/list</button>
            <button className="panel-btn panel-btn-secondary" style={{ marginLeft: "auto" }} onClick={fetchMessages}>Refresh</button>
          </div>

          <div className="panel-label">Message Log ({messages.length} messages)</div>
          {messages.map((msg) => (
            <div key={msg.id} className="panel-card" style={{ padding: 8 }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                  <span style={badgeStyle(msg.direction)}>{msg.direction === "sent" ? "OUT" : "IN"}</span>
                  <span style={{ fontWeight: 600, fontSize: 12 }}>{msg.method}</span>
                  <span style={badgeStyle(msg.status)}>{msg.status}</span>
                </div>
                <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>{new Date(msg.timestamp).toLocaleTimeString()}</span>
              </div>
              <div style={{ fontSize: 10, color: "var(--text-secondary)", fontFamily: "var(--font-family)", wordBreak: "break-all" }}>
                {msg.payload}
              </div>
            </div>
          ))}
          {messages.length === 0 && <div className="panel-empty">No messages yet. Use Quick Send or toggle the server to generate protocol messages.</div>}
        </div>
      )}
    </div>
  );
}
