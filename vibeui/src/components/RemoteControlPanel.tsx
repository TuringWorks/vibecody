/**
 * RemoteControlPanel — Mobile/Web Remote Control.
 *
 * Tabs: Server (start/stop, QR code, pairing token),
 * Clients (connected devices), Events (scrollable event log).
 * Pure TypeScript — no Tauri commands.
 */
import { useState } from "react";

type Tab = "server" | "clients" | "events";

interface ConnectedClient {
  id: string;
  name: string;
  type: "mobile" | "tablet" | "desktop" | "web";
  permissions: string[];
  lastSeen: string;
  connected: boolean;
}

interface RemoteEvent {
  id: string;
  timestamp: string;
  clientId: string;
  action: string;
  detail: string;
}

const MOCK_CLIENTS: ConnectedClient[] = [
  { id: "c1", name: "iPhone 15 Pro", type: "mobile", permissions: ["navigate", "execute", "view"], lastSeen: "2s ago", connected: true },
  { id: "c2", name: "iPad Air", type: "tablet", permissions: ["navigate", "view"], lastSeen: "15s ago", connected: true },
  { id: "c3", name: "Chrome Web", type: "web", permissions: ["view"], lastSeen: "2m ago", connected: false },
  { id: "c4", name: "MacBook Remote", type: "desktop", permissions: ["navigate", "execute", "view", "edit"], lastSeen: "1h ago", connected: false },
];

const MOCK_EVENTS: RemoteEvent[] = [
  { id: "e1", timestamp: "14:32:01", clientId: "c1", action: "file.open", detail: "src/main.rs" },
  { id: "e2", timestamp: "14:31:45", clientId: "c1", action: "command.run", detail: "cargo test" },
  { id: "e3", timestamp: "14:31:20", clientId: "c2", action: "navigate", detail: "Switched to Tests tab" },
  { id: "e4", timestamp: "14:30:58", clientId: "c1", action: "file.save", detail: "src/lib.rs" },
  { id: "e5", timestamp: "14:30:12", clientId: "c3", action: "connect", detail: "Web client connected" },
  { id: "e6", timestamp: "14:29:44", clientId: "c2", action: "command.run", detail: "git status" },
  { id: "e7", timestamp: "14:28:30", clientId: "c1", action: "file.open", detail: "Cargo.toml" },
  { id: "e8", timestamp: "14:27:15", clientId: "c3", action: "disconnect", detail: "Web client disconnected" },
];

const tabBtn = (active: boolean): React.CSSProperties => ({
  padding: "6px 14px",
  fontSize: 11,
  fontWeight: active ? 600 : 400,
  background: active ? "var(--accent-bg, rgba(99,102,241,0.15))" : "transparent",
  border: "1px solid " + (active ? "var(--accent-primary, #6366f1)" : "var(--border-color)"),
  borderRadius: 4,
  color: active ? "var(--text-info, #89b4fa)" : "var(--text-muted)",
  cursor: "pointer",
});

const typeIcon: Record<string, string> = { mobile: "phone", tablet: "tablet", desktop: "monitor", web: "globe" };

export default function RemoteControlPanel() {
  const [tab, setTab] = useState<Tab>("server");
  const [serverRunning, setServerRunning] = useState(false);
  const [port, setPort] = useState(9090);
  const [token] = useState("vbc-" + Math.random().toString(36).slice(2, 10));
  const [clients, setClients] = useState(MOCK_CLIENTS);
  const [events] = useState(MOCK_EVENTS);

  const toggleClient = (id: string) => {
    setClients(cs => cs.map(c => c.id === id ? { ...c, connected: !c.connected } : c));
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Tab bar */}
      <div style={{ display: "flex", gap: 6, padding: "8px 10px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" }}>
        {(["server", "clients", "events"] as Tab[]).map(t => (
          <button key={t} onClick={() => setTab(t)} style={tabBtn(tab === t)}>
            {t[0].toUpperCase() + t.slice(1)}
          </button>
        ))}
        <span style={{ marginLeft: "auto", fontSize: 10, color: serverRunning ? "var(--text-success, #a6e3a1)" : "var(--text-muted)", alignSelf: "center" }}>
          {serverRunning ? "Listening" : "Stopped"}
        </span>
      </div>

      <div style={{ flex: 1, overflowY: "auto", padding: 12, display: "flex", flexDirection: "column", gap: 12 }}>
        {/* Server tab */}
        {tab === "server" && (
          <>
            <div style={{ padding: 14, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
              <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 10 }}>Remote Server</div>
              <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 10 }}>
                <label style={{ fontSize: 11, color: "var(--text-muted)" }}>Port:</label>
                <input type="number" value={port} onChange={e => setPort(Number(e.target.value))}
                  style={{ width: 80, padding: "4px 8px", fontSize: 12, fontFamily: "monospace", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)" }} />
                <button onClick={() => setServerRunning(!serverRunning)}
                  style={{ padding: "6px 16px", fontSize: 11, fontWeight: 600, borderRadius: 4, border: "none", cursor: "pointer",
                    background: serverRunning ? "var(--text-danger, #f38ba8)" : "var(--text-success, #a6e3a1)",
                    color: "#1e1e2e" }}>
                  {serverRunning ? "Stop Server" : "Start Server"}
                </button>
              </div>
              {serverRunning && <div style={{ fontSize: 11, color: "var(--text-muted)" }}>Listening on 0.0.0.0:{port}</div>}
            </div>

            <div style={{ padding: 14, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)", textAlign: "center" }}>
              <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 10 }}>QR Code</div>
              <div style={{ width: 120, height: 120, margin: "0 auto", border: "2px dashed var(--border-color)", borderRadius: 8, display: "flex", alignItems: "center", justifyContent: "center", color: "var(--text-muted)", fontSize: 10 }}>
                {serverRunning ? "[QR Code]" : "Start server to generate"}
              </div>
            </div>

            <div style={{ padding: 14, background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
              <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 8 }}>Pairing Token</div>
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                <code style={{ flex: 1, padding: "6px 10px", background: "var(--bg-primary)", borderRadius: 4, fontSize: 13, fontFamily: "monospace", color: "var(--accent-primary, #6366f1)", letterSpacing: 1 }}>{token}</code>
                <button onClick={() => navigator.clipboard.writeText(token)}
                  style={{ padding: "5px 12px", fontSize: 10, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer" }}>Copy</button>
              </div>
            </div>
          </>
        )}

        {/* Clients tab */}
        {tab === "clients" && clients.map(c => (
          <div key={c.id} style={{ padding: 10, background: "var(--bg-secondary)", borderRadius: 6, border: `1px solid ${c.connected ? "var(--accent-primary, #6366f1)" : "var(--border-color)"}`, display: "flex", gap: 10, alignItems: "center" }}>
            <span style={{ fontSize: 14 }}>[{typeIcon[c.type]}]</span>
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: 12, fontWeight: 600, color: "var(--text-primary)" }}>{c.name}</div>
              <div style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 2 }}>
                {c.permissions.join(", ")} | {c.lastSeen}
              </div>
            </div>
            <span style={{ fontSize: 10, padding: "2px 8px", borderRadius: 10, background: c.connected ? "rgba(166,227,161,0.15)" : "rgba(243,139,168,0.15)", color: c.connected ? "var(--text-success, #a6e3a1)" : "var(--text-danger, #f38ba8)" }}>
              {c.connected ? "Online" : "Offline"}
            </span>
            <button onClick={() => toggleClient(c.id)}
              style={{ padding: "4px 10px", fontSize: 10, borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-muted)", cursor: "pointer" }}>
              {c.connected ? "Disconnect" : "Reconnect"}
            </button>
          </div>
        ))}

        {/* Events tab */}
        {tab === "events" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            {events.map(e => (
              <div key={e.id} style={{ display: "flex", gap: 10, padding: "6px 10px", background: "var(--bg-secondary)", borderRadius: 4, border: "1px solid var(--border-color)", fontSize: 11, fontFamily: "monospace" }}>
                <span style={{ color: "var(--text-muted)", minWidth: 60 }}>{e.timestamp}</span>
                <span style={{ color: "var(--accent-primary, #6366f1)", minWidth: 80 }}>{e.action}</span>
                <span style={{ color: "var(--text-primary)", flex: 1 }}>{e.detail}</span>
                <span style={{ color: "var(--text-muted)", fontSize: 10 }}>{e.clientId}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
