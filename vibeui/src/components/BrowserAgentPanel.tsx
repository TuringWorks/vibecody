import { useState } from "react";

type SubTab = "browse" | "sessions" | "config";

const card: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, border: "1px solid var(--border-color)" };
const label: React.CSSProperties = { fontSize: 12, color: "var(--text-secondary)", marginBottom: 4, display: "block" };
const input: React.CSSProperties = { width: "100%", padding: "6px 10px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary, #222)", color: "var(--text-primary)", fontSize: 12, fontFamily: "var(--font-mono)", boxSizing: "border-box" as const };
const btn: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "none", background: "var(--accent-color)", color: "#fff", cursor: "pointer", fontSize: 12, fontWeight: 600 };

interface BrowseSession {
  id: string;
  url: string;
  task: string;
  status: string;
  actions: number;
  screenshots: number;
}

export function BrowserAgentPanel() {
  const [tab, setTab] = useState<SubTab>("browse");
  const [url, setUrl] = useState("https://");
  const [task, setTask] = useState("");
  const [headless, setHeadless] = useState(true);
  const [sessions] = useState<BrowseSession[]>([
    { id: "demo-1", url: "https://example.com", task: "Extract pricing", status: "completed", actions: 12, screenshots: 5 },
  ]);

  return (
    <div style={{ padding: 16, display: "flex", flexDirection: "column", gap: 16, fontSize: 13, color: "var(--text-primary)" }}>
      <div style={{ display: "flex", gap: 2, borderBottom: "1px solid var(--border-color)", marginBottom: 4 }}>
        {(["browse", "sessions", "config"] as const).map(t => (
          <button key={t} onClick={() => setTab(t)} style={{
            padding: "6px 12px", border: "none", background: "transparent", cursor: "pointer",
            borderBottom: tab === t ? "2px solid var(--accent-color)" : "2px solid transparent",
            color: tab === t ? "var(--accent-color)" : "var(--text-secondary)", fontSize: 12, fontFamily: "inherit", textTransform: "capitalize",
          }}>{t === "browse" ? "New Task" : t === "sessions" ? "Sessions" : "Config"}</button>
        ))}
      </div>

      {tab === "browse" && (
        <div>
          <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>Browser Agent Task</div>
          <div style={card}>
            <label style={label}>Target URL</label>
            <input style={input} value={url} onChange={e => setUrl(e.target.value)} placeholder="https://example.com" />
            <div style={{ marginTop: 8 }}>
              <label style={label}>Task Description</label>
              <textarea style={{ ...input, height: 60, resize: "vertical" as const }} value={task} onChange={e => setTask(e.target.value)} placeholder="Extract all product names and prices from the page..." />
            </div>
            <div style={{ marginTop: 8, display: "flex", alignItems: "center", gap: 8 }}>
              <label style={{ fontSize: 12, display: "flex", alignItems: "center", gap: 4 }}>
                <input type="checkbox" checked={headless} onChange={e => setHeadless(e.target.checked)} /> Headless mode
              </label>
            </div>
            <div style={{ marginTop: 12 }}>
              <button style={{ ...btn, opacity: !url || !task ? 0.5 : 1 }} disabled={!url || !task}>Launch Browser Agent</button>
            </div>
          </div>
          <div style={{ ...card, marginTop: 12 }}>
            <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>Capabilities</div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.6 }}>
              <div>Navigate to URLs, click elements, fill forms, scroll pages</div>
              <div>Extract text and structured data from any web page</div>
              <div>Execute JavaScript for dynamic content interaction</div>
              <div>Capture screenshots at each step for visual verification</div>
              <div>Wait for elements to appear (SPA/dynamic content support)</div>
            </div>
          </div>
        </div>
      )}

      {tab === "sessions" && (
        <div>
          <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>Browse Sessions</div>
          {sessions.map(s => (
            <div key={s.id} style={{ ...card, marginBottom: 8 }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div>
                  <div style={{ fontWeight: 600, fontSize: 13 }}>{s.task}</div>
                  <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>{s.url}</div>
                </div>
                <span style={{ fontSize: 11, padding: "2px 8px", borderRadius: 4, background: s.status === "completed" ? "#4caf5022" : "#ff980022", color: s.status === "completed" ? "#4caf50" : "#ff9800" }}>{s.status}</span>
              </div>
              <div style={{ marginTop: 6, fontSize: 11, color: "var(--text-secondary)" }}>
                {s.actions} actions | {s.screenshots} screenshots
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "config" && (
        <div>
          <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>Browser Agent Configuration</div>
          <div style={card}>
            <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
              <tbody>
                <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Chrome Debug Port</td><td style={{ padding: "4px 0" }}>9222</td></tr>
                <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Default Viewport</td><td style={{ padding: "4px 0" }}>1280 x 720</td></tr>
                <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Request Timeout</td><td style={{ padding: "4px 0" }}>30s</td></tr>
                <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Vision Provider</td><td style={{ padding: "4px 0" }}>Claude (multimodal)</td></tr>
                <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Max Actions/Step</td><td style={{ padding: "4px 0" }}>5</td></tr>
                <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Verify After Action</td><td style={{ padding: "4px 0" }}>Enabled</td></tr>
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
