import { useState, useCallback } from "react";

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

export function ConnectorsPanel() {
  const [tab, setTab] = useState("connected");
  const [connected] = useState([
    { name: "GitHub", status: "green", type: "SCM" },
    { name: "Jira", status: "green", type: "Project" },
    { name: "Slack", status: "yellow", type: "Chat" },
    { name: "PostgreSQL", status: "green", type: "Database" },
    { name: "Datadog", status: "red", type: "Monitoring" },
    { name: "AWS S3", status: "green", type: "Storage" },
  ]);
  const [available] = useState([
    "GitHub", "GitLab", "Bitbucket", "Jira", "Linear", "Asana", "Slack", "Discord", "Teams",
    "PostgreSQL", "MySQL", "MongoDB", "Redis", "Datadog", "Grafana", "PagerDuty",
    "AWS S3", "GCS", "Confluence", "Notion",
  ]);
  const [webhookUrl] = useState("https://api.vibecody.dev/hooks/proj_abc123");
  const [events] = useState([
    { source: "GitHub", event: "push", time: "2 min ago", status: 200 },
    { source: "Jira", event: "issue.updated", time: "8 min ago", status: 200 },
    { source: "Slack", event: "message", time: "15 min ago", status: 200 },
    { source: "GitHub", event: "pull_request.opened", time: "1 hr ago", status: 200 },
    { source: "Datadog", event: "alert.triggered", time: "2 hr ago", status: 500 },
  ]);
  const [discovered, setDiscovered] = useState<string[]>([]);
  const [scanning, setScanning] = useState(false);

  const runDiscovery = useCallback(() => {
    setScanning(true);
    setDiscovered([]);
    setTimeout(() => {
      setDiscovered(["Redis on localhost:6379", "PostgreSQL on localhost:5432", "Elasticsearch on localhost:9200"]);
      setScanning(false);
    }, 1500);
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
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
          {connected.map((c) => (
            <div key={c.name} style={{ ...cardStyle, display: "flex", alignItems: "center", gap: 8 }}>
              <span style={dotStyle(statusColor(c.status))} />
              <div>
                <div style={{ fontWeight: 600, fontSize: 13 }}>{c.name}</div>
                <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>{c.type}</div>
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "available" && (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
          {available.map((name) => {
            const isConnected = connected.some((c) => c.name === name);
            return (
              <div key={name} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span style={{ fontSize: 13 }}>{name}</span>
                <button style={{ ...btnStyle, background: isConnected ? "var(--success-color)" : "var(--accent-color)", fontSize: 11, padding: "4px 10px" }}>
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
            <div style={{ fontSize: 13, fontFamily: "monospace", wordBreak: "break-all" }}>{webhookUrl}</div>
          </div>
          <div style={{ fontWeight: 600, fontSize: 13, margin: "12px 0 8px" }}>Recent Events</div>
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
            {discovered.map((s, i) => (
              <div key={i} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={dotStyle("var(--success-color)")} />
                  <span style={{ fontSize: 13 }}>{s}</span>
                </div>
                <button style={{ ...btnStyle, fontSize: 11, padding: "4px 10px" }}>Connect</button>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
