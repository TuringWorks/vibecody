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

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 };
const tabBtnStyle = (active: boolean): React.CSSProperties => ({ ...btnStyle, background: active ? "var(--accent-primary)" : "var(--bg-tertiary)", color: active ? "var(--btn-primary-fg)" : "var(--text-primary)", marginRight: 4 });

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
    return <div style={panelStyle}><h2 style={headingStyle}>Agent Client Protocol (ACP)</h2><div>Loading...</div></div>;
  }

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Agent Client Protocol (ACP)</h2>

      {error && (
        <div style={{ ...cardStyle, borderColor: "var(--error-color)", color: "var(--error-color)", marginBottom: 12 }}>
          {error}
          <button style={{ ...btnStyle, marginLeft: 8, fontSize: 10 }} onClick={() => setError(null)}>Dismiss</button>
        </div>
      )}

      <div style={{ marginBottom: 12 }}>
        <button style={tabBtnStyle(tab === "server")} onClick={() => setTab("server")}>Server</button>
        <button style={tabBtnStyle(tab === "client")} onClick={() => setTab("client")}>Client</button>
        <button style={tabBtnStyle(tab === "protocol")} onClick={() => setTab("protocol")}>Protocol</button>
      </div>

      {tab === "server" && (
        <div>
          <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <div>
              <div style={{ fontWeight: 600 }}>ACP Server</div>
              <div style={{ fontSize: 11, color: serverRunning ? "var(--success-color)" : "var(--text-secondary)" }}>
                {serverRunning ? "Running on localhost:7878/acp" : "Stopped"}
              </div>
              {status && <div style={{ fontSize: 10, color: "var(--text-secondary)" }}>v{status.version} | {status.connected_clients} client(s)</div>}
            </div>
            <button
              style={{ ...btnStyle, background: serverRunning ? "var(--error-color)" : "var(--success-color)", color: "var(--btn-primary-fg)" }}
              onClick={handleToggleServer}
            >
              {serverRunning ? "Stop Server" : "Start Server"}
            </button>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10, marginBottom: 12 }}>
            <div style={cardStyle}>
              <div style={labelStyle}>Tools</div>
              <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{toolCount}</div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>Resources</div>
              <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{resourceCount}</div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>Prompts</div>
              <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{promptCount}</div>
            </div>
          </div>

          <div style={labelStyle}>Registered Capabilities</div>
          {capabilities.map((cap) => (
            <div key={cap.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <span style={{ fontWeight: 600 }}>{cap.name}</span>{" "}
                <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>v{cap.version}</span>
                <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 2 }}>{cap.description}</div>
              </div>
              <span style={badgeStyle(cap.type)}>{cap.type}</span>
            </div>
          ))}

          {/* Register new capability form */}
          <div style={{ ...cardStyle, marginTop: 12 }}>
            <div style={{ ...labelStyle, fontWeight: 600, marginBottom: 8 }}>Register New Capability</div>
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
              <button style={{ ...btnStyle, background: "var(--accent-primary)", color: "var(--btn-primary-fg)" }} onClick={handleRegisterCapability}>Register</button>
            </div>
          </div>
        </div>
      )}

      {tab === "client" && (
        <div>
          <div style={cardStyle}>
            <div style={labelStyle}>External ACP Server URL</div>
            <div style={{ display: "flex", gap: 8 }}>
              <input style={{ ...inputStyle, flex: 1 }} value={clientUrl} onChange={(e) => setClientUrl(e.target.value)} placeholder="http://localhost:3001/acp" />
              {!clientConnected ? (
                <button style={{ ...btnStyle, background: "var(--accent-primary)", color: "var(--btn-primary-fg)" }} onClick={connectClient}>Connect</button>
              ) : (
                <button style={{ ...btnStyle, background: "var(--error-color)", color: "var(--btn-primary-fg)" }} onClick={disconnectClient}>Disconnect</button>
              )}
            </div>
          </div>

          <div style={cardStyle}>
            <div style={labelStyle}>Negotiation Status</div>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 4 }}>
              <span style={badgeStyle(negotiationStatus === "connected" ? "ok" : negotiationStatus === "failed" ? "error" : "pending")}>
                {negotiationStatus}
              </span>
              {negotiationStatus === "connected" && <span style={{ fontSize: 11 }}>Protocol v1.0 negotiated</span>}
            </div>
          </div>

          {clientConnected && (
            <div style={cardStyle}>
              <div style={labelStyle}>Remote Server Capabilities</div>
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
          <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between" }}>
            <div>
              <div style={{ fontWeight: 600 }}>ACP Protocol</div>
              <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>Based on Model Context Protocol (MCP) specification</div>
            </div>
            <div style={{ textAlign: "right" }}>
              <div style={labelStyle}>Version</div>
              <div style={{ fontWeight: 600 }}>{status?.version ?? "1.0.0"}</div>
            </div>
          </div>

          {/* Quick-send message controls */}
          <div style={{ ...cardStyle, display: "flex", gap: 6, alignItems: "center" }}>
            <span style={labelStyle}>Quick Send:</span>
            <button style={btnStyle} onClick={() => handleSendMessage("initialize", "{}")}>initialize</button>
            <button style={btnStyle} onClick={() => handleSendMessage("tools/list", "{}")}>tools/list</button>
            <button style={btnStyle} onClick={() => handleSendMessage("resources/list", "{}")}>resources/list</button>
            <button style={{ ...btnStyle, marginLeft: "auto" }} onClick={fetchMessages}>Refresh</button>
          </div>

          <div style={labelStyle}>Message Log ({messages.length} messages)</div>
          {messages.map((msg) => (
            <div key={msg.id} style={{ ...cardStyle, padding: 8 }}>
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
          {messages.length === 0 && <div style={{ ...cardStyle, textAlign: "center", color: "var(--text-secondary)" }}>No messages yet. Use Quick Send or toggle the server to generate protocol messages.</div>}
        </div>
      )}
    </div>
  );
}
