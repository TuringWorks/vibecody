import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

const panelStyle: React.CSSProperties = {
  padding: 16,
  height: "100%",
  overflow: "auto",
  color: "var(--text-primary)",
  background: "var(--bg-primary)",
};

const headingStyle: React.CSSProperties = {
  fontSize: 18,
  fontWeight: 600,
  marginBottom: 12,
  color: "var(--text-primary)",
};

const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  borderRadius: 8,
  padding: 12,
  marginBottom: 8,
  border: "1px solid var(--border-color)",
};

const btnStyle: React.CSSProperties = {
  padding: "6px 14px",
  borderRadius: 6,
  border: "1px solid var(--border-color)",
  background: "var(--accent-color)",
  color: "var(--btn-primary-fg, #fff)",
  cursor: "pointer",
  fontSize: 13,
  marginRight: 8,
};

const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 16px",
  cursor: "pointer",
  borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
  color: active ? "var(--accent-color)" : "var(--text-secondary)",
  background: "transparent",
  border: "none",
  fontSize: 13,
  fontWeight: active ? 600 : 400,
});

const dotStyle = (color: string): React.CSSProperties => ({
  width: 8, height: 8, borderRadius: "50%", background: color, display: "inline-block",
});

interface ConnectorEntry {
  id: string;
  type: string;
  status: string;
  key_len: number;
  connected_at: string;
}

interface DiscoveredEntry {
  type: string;
  status: string;
}

interface WebhookEvent {
  source: string;
  event: string;
  time: string;
  status: number;
}

export function ConnectorsPanel() {
  const [tab, setTab] = useState("connected");
  const [connected, setConnected] = useState<ConnectorEntry[]>([]);
  const [available, setAvailable] = useState<string[]>([]);
  const [webhookUrl] = useState("");
  const [events] = useState<WebhookEvent[]>([]);
  const [discovered, setDiscovered] = useState<DiscoveredEntry[]>([]);
  const [scanning, setScanning] = useState(false);

  const fetchData = useCallback(() => {
    invoke<ConnectorEntry[]>("connectors_list").then(setConnected).catch(console.error);
    invoke<string[]>("connectors_available").then(setAvailable).catch(console.error);
  }, []);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  const handleSetup = useCallback((name: string) => {
    invoke("connectors_add", { connectorType: name, apiKey: "" })
      .then(() => fetchData())
      .catch(console.error);
  }, [fetchData]);

  const runDiscovery = useCallback(() => {
    setScanning(true);
    setDiscovered([]);
    invoke<{ discovered: DiscoveredEntry[] }>("connectors_discover")
      .then((result) => {
        setDiscovered(result.discovered);
      })
      .catch(console.error)
      .finally(() => setScanning(false));
  }, []);

  const statusColor = (s: string) => s === "green" ? "var(--success-color)" : s === "yellow" ? "var(--warning-color)" : "var(--error-color)";

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Native Integration Connectors</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        {["connected", "available", "webhooks", "discovery"].map((t) => (
          <button key={t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {tab === "connected" && (
        <div>
          <div style={{ marginBottom: 8 }}>
            <button style={btnStyle} onClick={fetchData}>Refresh</button>
          </div>
          {connected.length === 0 && (
            <div style={{ ...cardStyle, color: "var(--text-secondary)", fontSize: 13 }}>No connectors configured yet. Go to Available to set one up.</div>
          )}
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
            {connected.map((c) => (
              <div key={c.id} style={{ ...cardStyle, display: "flex", alignItems: "center", gap: 8 }}>
                <span style={dotStyle(statusColor(c.status))} />
                <div>
                  <div style={{ fontWeight: 600, fontSize: 13 }}>{c.type}</div>
                  <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>
                    Key: {c.key_len} chars &middot; {c.connected_at}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {tab === "available" && (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
          {available.map((name) => {
            const isConnected = connected.some((c) => c.type === name);
            return (
              <div key={name} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span style={{ fontSize: 13 }}>{name}</span>
                <button
                  style={{ ...btnStyle, background: isConnected ? "var(--success-color)" : "var(--accent-color)", fontSize: 11, padding: "4px 10px" }}
                  onClick={() => !isConnected && handleSetup(name)}
                  disabled={isConnected}
                >
                  {isConnected ? "Connected" : "Setup"}
                </button>
              </div>
            );
          })}
        </div>
      )}

      {tab === "webhooks" && (
        <div>
          <div style={cardStyle}>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>Webhook Endpoint</div>
            <div style={{ fontSize: 13, fontFamily: "monospace", wordBreak: "break-all" }}>
              {webhookUrl || "No webhook URL configured"}
            </div>
          </div>
          <div style={{ fontWeight: 600, fontSize: 13, margin: "12px 0 8px" }}>Recent Events</div>
          {events.length === 0 && (
            <div style={{ ...cardStyle, color: "var(--text-secondary)", fontSize: 13 }}>No webhook events yet.</div>
          )}
          {events.map((e, i) => (
            <div key={i} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <span style={{ fontWeight: 600, fontSize: 13 }}>{e.source}</span>
                <span style={{ fontSize: 12, color: "var(--text-secondary)", marginLeft: 8 }}>{e.event}</span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>{e.time}</span>
                <span style={{ fontSize: 11, fontWeight: 600, color: e.status === 200 ? "var(--success-color)" : "var(--error-color)" }}>{e.status}</span>
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "discovery" && (
        <div>
          <button style={btnStyle} onClick={runDiscovery} disabled={scanning}>
            {scanning ? "Scanning..." : "Auto-Detect Services"}
          </button>
          <div style={{ marginTop: 12 }}>
            {discovered.length === 0 && !scanning && (
              <div style={{ ...cardStyle, color: "var(--text-secondary)", fontSize: 13 }}>Click auto-detect to scan for local services.</div>
            )}
            {discovered.map((d, i) => (
              <div key={i} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={dotStyle(statusColor(d.status))} />
                  <span style={{ fontSize: 13 }}>{d.type}</span>
                </div>
                <button
                  style={{ ...btnStyle, fontSize: 11, padding: "4px 10px" }}
                  onClick={() => handleSetup(d.type)}
                >
                  Connect
                </button>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
