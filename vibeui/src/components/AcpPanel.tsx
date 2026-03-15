/**
 * AcpPanel — Agent Client Protocol panel.
 *
 * Manage ACP server/client connections, view registered capabilities
 * and tools, and inspect protocol messages.
 * Pure TypeScript — no Tauri commands needed.
 */
import { useState } from "react";

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

// ── Mock Data ─────────────────────────────────────────────────────────────────

const MOCK_CAPABILITIES: AcpCapability[] = [
  { id: "c1", name: "file_read", type: "tool", description: "Read file contents from the workspace", version: "1.0.0" },
  { id: "c2", name: "file_write", type: "tool", description: "Write content to files", version: "1.0.0" },
  { id: "c3", name: "code_search", type: "tool", description: "Search code with regex patterns", version: "1.1.0" },
  { id: "c4", name: "project_context", type: "resource", description: "Current project structure and metadata", version: "1.0.0" },
  { id: "c5", name: "git_history", type: "resource", description: "Recent git commit history", version: "1.0.0" },
  { id: "c6", name: "code_review", type: "prompt", description: "Review code changes with AI", version: "0.9.0" },
  { id: "c7", name: "refactor", type: "prompt", description: "Suggest refactoring improvements", version: "0.9.0" },
  { id: "c8", name: "bash_exec", type: "tool", description: "Execute shell commands safely", version: "2.0.0" },
];

const MOCK_MESSAGES: AcpMessage[] = [
  { id: "m1", timestamp: "2026-03-13T08:30:01Z", direction: "received", method: "initialize", status: "ok", payload: '{"protocolVersion":"1.0","capabilities":{"tools":true}}' },
  { id: "m2", timestamp: "2026-03-13T08:30:01Z", direction: "sent", method: "initialize/result", status: "ok", payload: '{"protocolVersion":"1.0","serverInfo":{"name":"vibecody","version":"0.5.0"}}' },
  { id: "m3", timestamp: "2026-03-13T08:30:02Z", direction: "received", method: "tools/list", status: "ok", payload: '{}' },
  { id: "m4", timestamp: "2026-03-13T08:30:02Z", direction: "sent", method: "tools/list/result", status: "ok", payload: '{"tools":[{"name":"file_read"},{"name":"file_write"},{"name":"code_search"},{"name":"bash_exec"}]}' },
  { id: "m5", timestamp: "2026-03-13T08:30:05Z", direction: "received", method: "tools/call", status: "ok", payload: '{"name":"file_read","arguments":{"path":"src/main.rs"}}' },
  { id: "m6", timestamp: "2026-03-13T08:30:05Z", direction: "sent", method: "tools/call/result", status: "ok", payload: '{"content":[{"type":"text","text":"fn main() { ... }"}]}' },
  { id: "m7", timestamp: "2026-03-13T08:31:00Z", direction: "received", method: "tools/call", status: "error", payload: '{"name":"file_read","arguments":{"path":"/etc/shadow"}}' },
  { id: "m8", timestamp: "2026-03-13T08:31:00Z", direction: "sent", method: "tools/call/result", status: "error", payload: '{"error":{"code":-1,"message":"Access denied: path outside workspace"}}' },
];

// ── Styles ────────────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono, monospace)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-primary)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 };
const tabBtnStyle = (active: boolean): React.CSSProperties => ({ ...btnStyle, background: active ? "var(--accent-primary)" : "var(--bg-tertiary)", color: active ? "white" : "var(--text-primary)", marginRight: 4 });

const inputStyle: React.CSSProperties = { width: "100%", padding: "6px 10px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontSize: 12, fontFamily: "var(--font-mono)", boxSizing: "border-box" };
const badgeStyle = (variant: string): React.CSSProperties => {
  const colors: Record<string, string> = { tool: "var(--accent-color)", resource: "var(--accent-purple)", prompt: "var(--warning-color)", ok: "var(--success-color)", error: "var(--error-color)", pending: "var(--text-muted)", sent: "var(--accent-color)", received: "var(--accent-purple)" };
  return { display: "inline-block", padding: "2px 8px", borderRadius: 10, fontSize: 10, fontWeight: 600, color: "white", background: colors[variant] || "var(--text-muted)" };
};

// ── Component ─────────────────────────────────────────────────────────────────

type Tab = "server" | "client" | "protocol";

export function AcpPanel() {
  const [tab, setTab] = useState<Tab>("server");
  const [serverRunning, setServerRunning] = useState(true);
  const [clientUrl, setClientUrl] = useState("http://localhost:3001/acp");
  const [clientConnected, setClientConnected] = useState(false);
  const [negotiationStatus, setNegotiationStatus] = useState<"idle" | "connecting" | "connected" | "failed">("idle");
  const [messages] = useState<AcpMessage[]>(MOCK_MESSAGES);

  const toolCount = MOCK_CAPABILITIES.filter((c) => c.type === "tool").length;
  const resourceCount = MOCK_CAPABILITIES.filter((c) => c.type === "resource").length;
  const promptCount = MOCK_CAPABILITIES.filter((c) => c.type === "prompt").length;

  const connectClient = () => {
    setNegotiationStatus("connecting");
    setTimeout(() => {
      setNegotiationStatus("connected");
      setClientConnected(true);
    }, 800);
  };

  const disconnectClient = () => {
    setClientConnected(false);
    setNegotiationStatus("idle");
  };

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Agent Client Protocol (ACP)</h2>

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
            </div>
            <button
              style={{ ...btnStyle, background: serverRunning ? "var(--error-color)" : "var(--success-color)", color: "white" }}
              onClick={() => setServerRunning(!serverRunning)}
            >
              {serverRunning ? "Stop Server" : "Start Server"}
            </button>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 10, marginBottom: 12 }}>
            <div style={cardStyle}>
              <div style={labelStyle}>Tools</div>
              <div style={{ fontSize: 20, fontWeight: 700 }}>{toolCount}</div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>Resources</div>
              <div style={{ fontSize: 20, fontWeight: 700 }}>{resourceCount}</div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>Prompts</div>
              <div style={{ fontSize: 20, fontWeight: 700 }}>{promptCount}</div>
            </div>
          </div>

          <div style={labelStyle}>Registered Capabilities</div>
          {MOCK_CAPABILITIES.map((cap) => (
            <div key={cap.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <span style={{ fontWeight: 600 }}>{cap.name}</span>{" "}
                <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>v{cap.version}</span>
                <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 2 }}>{cap.description}</div>
              </div>
              <span style={badgeStyle(cap.type)}>{cap.type}</span>
            </div>
          ))}
        </div>
      )}

      {tab === "client" && (
        <div>
          <div style={cardStyle}>
            <div style={labelStyle}>External ACP Server URL</div>
            <div style={{ display: "flex", gap: 8 }}>
              <input style={{ ...inputStyle, flex: 1 }} value={clientUrl} onChange={(e) => setClientUrl(e.target.value)} placeholder="http://localhost:3001/acp" />
              {!clientConnected ? (
                <button style={{ ...btnStyle, background: "var(--accent-primary)", color: "white" }} onClick={connectClient}>Connect</button>
              ) : (
                <button style={{ ...btnStyle, background: "var(--error-color)", color: "white" }} onClick={disconnectClient}>Disconnect</button>
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
                <div>Tools: file_read, file_write, bash_exec</div>
                <div>Resources: project_context</div>
                <div>Prompts: code_review</div>
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
              <div style={{ fontWeight: 600 }}>1.0.0</div>
            </div>
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
              <div style={{ fontSize: 10, color: "var(--text-secondary)", fontFamily: "var(--font-mono, monospace)", wordBreak: "break-all" }}>
                {msg.payload}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
