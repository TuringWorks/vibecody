import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

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
    <div className="panel-container">
      <h2 style={{ fontSize: 18, fontWeight: 600, marginBottom: 12, color: "var(--text-primary)" }}>Native Integration Connectors</h2>
      <div className="panel-tab-bar">
        {["connected", "available", "webhooks", "discovery"].map((t) => (
          <button key={t} className={`panel-tab ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {tab === "connected" && (
        <div>
          <div style={{ marginBottom: 8 }}>
            <button className="panel-btn panel-btn-primary" onClick={fetchData}>Refresh</button>
          </div>
          {connected.length === 0 && (
            <div className="panel-empty">No connectors configured yet. Go to Available to set one up.</div>
          )}
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
            {connected.map((c) => (
              <div key={c.id} className="panel-card" style={{ display: "flex", alignItems: "center", gap: 8 }}>
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
              <div key={name} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span style={{ fontSize: 13 }}>{name}</span>
                <button
                  className={`panel-btn ${isConnected ? "panel-btn-secondary" : "panel-btn-primary"}`}
                  style={{ fontSize: 11, padding: "4px 10px" }}
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
          <div className="panel-card">
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 4 }}>Webhook Endpoint</div>
            <div className="panel-mono" style={{ wordBreak: "break-all" }}>
              {webhookUrl || "No webhook URL configured"}
            </div>
          </div>
          <div style={{ fontWeight: 600, fontSize: 13, margin: "12px 0 8px" }}>Recent Events</div>
          {events.length === 0 && (
            <div className="panel-empty">No webhook events yet.</div>
          )}
          {events.map((e, i) => (
            <div key={i} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
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
          <button className="panel-btn panel-btn-primary" onClick={runDiscovery} disabled={scanning}>
            {scanning ? "Scanning..." : "Auto-Detect Services"}
          </button>
          <div style={{ marginTop: 12 }}>
            {discovered.length === 0 && !scanning && (
              <div className="panel-empty">Click auto-detect to scan for local services.</div>
            )}
            {discovered.map((d, i) => (
              <div key={i} className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={dotStyle(statusColor(d.status))} />
                  <span style={{ fontSize: 13 }}>{d.type}</span>
                </div>
                <button
                  className="panel-btn panel-btn-primary"
                  style={{ fontSize: 11, padding: "4px 10px" }}
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
