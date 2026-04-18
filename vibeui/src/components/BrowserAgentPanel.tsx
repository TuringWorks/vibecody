/* eslint-disable @typescript-eslint/no-explicit-any */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

type SubTab = "browse" | "sessions" | "config";


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
  const [sessions, setSessions] = useState<BrowseSession[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchSessions = useCallback(async () => {
    try {
      const data = await invoke<unknown>("browser_list_sessions");
      const list = Array.isArray(data) ? data : [];
      setSessions(list.map((s: any) => ({
        id: String(s.id),
        url: s.url || "",
        task: s.task || "",
        status: s.status || "unknown",
        actions: s.actions ?? 0,
        screenshots: s.screenshots ?? 0,
      })));
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    setLoading(true);
    setError(null);
    fetchSessions().finally(() => setLoading(false));
    const id = setInterval(fetchSessions, 5_000);
    return () => clearInterval(id);
  }, [fetchSessions]);

  const handleLaunch = useCallback(async () => {
    if (!url || !task) return;
    try {
      await invoke("browser_create_session", { url, task, headless });
      setUrl("https://");
      setTask("");
      await fetchSessions();
    } catch (e) {
      console.error("browser_create_session failed:", e);
    }
  }, [url, task, headless, fetchSessions]);

  if (loading) return <div className="panel-loading">Loading browser sessions...</div>;
  if (error) return <div className="panel-error">Error: {error}</div>;

  return (
    <div className="panel-container">
      <div className="panel-tab-bar" style={{ padding: "0 16px", flexShrink: 0 }}>
        {(["browse", "sessions", "config"] as const).map(t => (
          <button key={t} onClick={() => setTab(t)} className={`panel-tab ${tab === t ? "active" : ""}`}>
            {t === "browse" ? "New Task" : t === "sessions" ? "Sessions" : "Config"}
          </button>
        ))}
      </div>

      {tab === "browse" && (
        <div>
          <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, marginBottom: 12 }}>Browser Agent Task</div>
          <div className="panel-card">
            <label className="panel-label">Target URL</label>
            <input className="panel-input panel-input-full" value={url} onChange={e => setUrl(e.target.value)} placeholder="https://example.com" />
            <div style={{ marginTop: 8 }}>
              <label className="panel-label">Task Description</label>
              <textarea className="panel-input panel-input-full" style={{ height: 60, resize: "vertical" }} value={task} onChange={e => setTask(e.target.value)} placeholder="Extract all product names and prices from the page..." />
            </div>
            <div style={{ marginTop: 8, display: "flex", alignItems: "center", gap: 8 }}>
              <label style={{ fontSize: "var(--font-size-base)", display: "flex", alignItems: "center", gap: 4 }}>
                <input type="checkbox" checked={headless} onChange={e => setHeadless(e.target.checked)} /> Headless mode
              </label>
            </div>
            <div style={{ marginTop: 12 }}>
              <button className="panel-btn panel-btn-primary" style={{ opacity: !url || !task ? 0.5 : 1 }} disabled={!url || !task} onClick={handleLaunch}>Launch Browser Agent</button>
            </div>
          </div>
          <div className="panel-card" style={{ marginTop: 12 }}>
            <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600, marginBottom: 8 }}>Capabilities</div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", lineHeight: 1.6 }}>
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
          <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, marginBottom: 12 }}>Browse Sessions</div>
          {sessions.length === 0 && <div className="panel-empty">No sessions yet. Launch a browser agent task to get started.</div>}
          {sessions.map(s => (
            <div key={s.id} className="panel-card" style={{ marginBottom: 8 }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <div>
                  <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{s.task}</div>
                  <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{s.url}</div>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                  <span style={{ fontSize: "var(--font-size-sm)", padding: "2px 8px", borderRadius: "var(--radius-xs-plus)", background: s.status === "completed" ? "var(--success-bg)" : s.status === "running" ? "color-mix(in srgb, var(--accent-blue) 13%, transparent)" : "color-mix(in srgb, var(--accent-gold) 13%, transparent)", color: s.status === "completed" ? "var(--accent-green)" : s.status === "running" ? "var(--accent-blue)" : "var(--accent-gold)" }}>{s.status}</span>
                  {s.status === "running" && (
                    <button className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-sm)", padding: "2px 8px" }}
                      onClick={() => invoke("browser_close_session", { sessionId: s.id }).then(fetchSessions).catch(() => {})}>
                      Stop
                    </button>
                  )}
                </div>
              </div>
              <div style={{ marginTop: 6, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
                {s.actions} actions | {s.screenshots} screenshots
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "config" && (
        <div>
          <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, marginBottom: 12 }}>Browser Agent Configuration</div>
          <div className="panel-card">
            <table style={{ width: "100%", fontSize: "var(--font-size-base)", borderCollapse: "collapse" }}>
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
