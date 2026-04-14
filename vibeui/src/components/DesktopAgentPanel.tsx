import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

type SubTab = "actions" | "windows" | "macros" | "config";

const mono: React.CSSProperties = { fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)" };

interface WindowInfo {
  id: string;
  title: string;
  app: string;
  focused: boolean;
}

export function DesktopAgentPanel() {
  const [tab, setTab] = useState<SubTab>("actions");
  const [mouseX, setMouseX] = useState(640);
  const [mouseY, setMouseY] = useState(360);
  const [typeText, setTypeText] = useState("");
  const [keyCombo, setKeyCombo] = useState("ctrl+c");
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [actionResult, setActionResult] = useState<string | null>(null);

  const platform = navigator.platform.includes("Mac") ? "macOS" : navigator.platform.includes("Linux") ? "Linux" : "Windows";

  const refreshWindows = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await invoke<{ ok: boolean; windows: WindowInfo[] }>("desktop_run_action", { action: "refresh_windows", params: {} });
      if (res.windows) {
        setWindows(res.windows);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const runAction = useCallback(async (action: string, params: Record<string, unknown>) => {
    setActionResult(null);
    setError(null);
    try {
      const res = await invoke<{ ok: boolean; action: string; timestamp: string }>("desktop_run_action", { action, params });
      setActionResult(`${res.action} executed at ${res.timestamp}`);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  // Fetch windows on mount and when switching to windows tab
  useEffect(() => {
    if (tab === "windows") {
      refreshWindows();
    }
  }, [tab, refreshWindows]);

  // Also load initial list
  useEffect(() => {
    invoke<{ ok: boolean; windows: WindowInfo[] }>("desktop_run_action", { action: "refresh_windows", params: {} })
      .then(res => { if (res.windows) setWindows(res.windows); })
      .catch(() => {});
  }, []);

  return (
    <div className="panel-container">
      <div className="panel-tab-bar">
        {(["actions", "windows", "macros", "config"] as const).map(t => (
          <button key={t} onClick={() => setTab(t)} className={`panel-tab ${tab === t ? "active" : ""}`} style={{ textTransform: "capitalize" }}>{t}</button>
        ))}
      </div>

      {error && (
        <div className="panel-error">
          {error}
        </div>
      )}

      <div className="panel-body" style={{ display: "flex", flexDirection: "column", gap: 16 }}>
      {actionResult && (
        <div style={{ padding: "8px 12px", background: "var(--success-bg, #00ff0011)", border: "1px solid var(--success-color)", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-base)", color: "var(--success-color)" }}>
          {actionResult}
        </div>
      )}

      {tab === "actions" && (
        <div>
          <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, marginBottom: 12 }}>Desktop Actions</div>
          <p style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", margin: "0 0 12px" }}>
            Cross-platform GUI automation: mouse, keyboard, and window control. Detected platform: <strong>{platform}</strong>
          </p>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
            <div className="panel-card">
              <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600, marginBottom: 8 }}>Mouse</div>
              <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
                <div>
                  <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>X</span>
                  <input type="number" value={mouseX} onChange={e => setMouseX(+e.target.value)} style={{ ...mono, width: 60, padding: "3px 6px", border: "1px solid var(--border-color)", borderRadius: 3, background: "var(--bg-tertiary)", color: "var(--text-primary)", marginLeft: 4 }} />
                </div>
                <div>
                  <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Y</span>
                  <input type="number" value={mouseY} onChange={e => setMouseY(+e.target.value)} style={{ ...mono, width: 60, padding: "3px 6px", border: "1px solid var(--border-color)", borderRadius: 3, background: "var(--bg-tertiary)", color: "var(--text-primary)", marginLeft: 4 }} />
                </div>
              </div>
              <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                <button className="panel-btn panel-btn-primary" style={{ fontSize: "var(--font-size-sm)", padding: "4px 10px" }} onClick={() => runAction("mouse_move", { x: mouseX, y: mouseY })}>Move</button>
                <button className="panel-btn panel-btn-primary" style={{ fontSize: "var(--font-size-sm)", padding: "4px 10px" }} onClick={() => runAction("mouse_click", { x: mouseX, y: mouseY })}>Click</button>
                <button className="panel-btn panel-btn-primary" style={{ fontSize: "var(--font-size-sm)", padding: "4px 10px" }} onClick={() => runAction("mouse_double_click", { x: mouseX, y: mouseY })}>Double-Click</button>
                <button className="panel-btn panel-btn-primary" style={{ fontSize: "var(--font-size-sm)", padding: "4px 10px" }} onClick={() => runAction("mouse_right_click", { x: mouseX, y: mouseY })}>Right-Click</button>
              </div>
            </div>

            <div className="panel-card">
              <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600, marginBottom: 8 }}>Keyboard</div>
              <div style={{ marginBottom: 8 }}>
                <input className="panel-mono" style={{ width: "100%", padding: "4px 8px", border: "1px solid var(--border-color)", borderRadius: 3, background: "var(--bg-tertiary)", color: "var(--text-primary)", boxSizing: "border-box" }} value={typeText} onChange={e => setTypeText(e.target.value)} placeholder="Text to type..." />
              </div>
              <div style={{ display: "flex", gap: 4, marginBottom: 8 }}>
                <button className="panel-btn panel-btn-primary" style={{ fontSize: "var(--font-size-sm)", padding: "4px 10px" }} onClick={() => runAction("type_text", { text: typeText })}>Type Text</button>
              </div>
              <div style={{ marginBottom: 4 }}>
                <input className="panel-mono" style={{ width: "100%", padding: "4px 8px", border: "1px solid var(--border-color)", borderRadius: 3, background: "var(--bg-tertiary)", color: "var(--text-primary)", boxSizing: "border-box" }} value={keyCombo} onChange={e => setKeyCombo(e.target.value)} placeholder="e.g., ctrl+shift+p" />
              </div>
              <button className="panel-btn panel-btn-primary" style={{ fontSize: "var(--font-size-sm)", padding: "4px 10px" }} onClick={() => runAction("key_combo", { combo: keyCombo })}>Press Key Combo</button>
            </div>
          </div>

          <div className="panel-card" style={{ marginTop: 12 }}>
            <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600, marginBottom: 8 }}>Quick Actions</div>
            <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
              {["screenshot", "get_mouse_position", "get_screen_size", "get_active_window"].map(a => (
                <button key={a} onClick={() => runAction(a, {})} className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-sm)", padding: "4px 10px" }}>
                  {a.replace(/_/g, " ").replace(/\b\w/g, c => c.toUpperCase())}
                </button>
              ))}
            </div>
          </div>
        </div>
      )}

      {tab === "windows" && (
        <div>
          <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, marginBottom: 12 }}>Window Management</div>
          <button className="panel-btn panel-btn-primary" style={{ marginBottom: 12, fontSize: "var(--font-size-sm)" }} onClick={refreshWindows} disabled={loading}>
            {loading ? "Refreshing..." : "Refresh Windows"}
          </button>
          {windows.length === 0 && !loading && (
            <div className="panel-empty">No windows detected. Click Refresh to scan.</div>
          )}
          {windows.map(w => (
            <div key={w.id} className="panel-card" style={{ marginBottom: 6, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontWeight: 600, fontSize: "var(--font-size-base)" }}>{w.title}</div>
                <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>{w.app}</div>
              </div>
              <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
                {w.focused && <span style={{ fontSize: "var(--font-size-xs)", padding: "1px 6px", borderRadius: 3, background: "var(--success-bg)", color: "var(--accent-green)" }}>Focused</span>}
                <button className="panel-btn panel-btn-primary" style={{ fontSize: "var(--font-size-xs)", padding: "3px 8px" }} onClick={() => runAction("focus_window", { id: w.id, app: w.app })}>Focus</button>
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "macros" && (
        <div>
          <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, marginBottom: 12 }}>Action Macros</div>
          <p style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", margin: "0 0 12px" }}>Record and replay sequences of desktop actions.</p>
          <div className="panel-card">
            <div style={{ display: "flex", gap: 8 }}>
              <button className="panel-btn panel-btn-primary">Record Macro</button>
              <button className="panel-btn panel-btn-secondary">Import JSON</button>
            </div>
          </div>
          <div className="panel-card" style={{ marginTop: 8, color: "var(--text-secondary)", fontSize: "var(--font-size-base)", fontStyle: "italic" }}>No saved macros yet.</div>
        </div>
      )}

      {tab === "config" && (
        <div>
          <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600, marginBottom: 12 }}>Desktop Agent Configuration</div>
          <div className="panel-card">
            <table style={{ width: "100%", fontSize: "var(--font-size-base)", borderCollapse: "collapse" }}>
              <tbody>
                <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Platform</td><td style={{ padding: "4px 0" }}>{platform}</td></tr>
                <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Action Delay</td><td style={{ padding: "4px 0" }}>100ms</td></tr>
                <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Tools (macOS)</td><td style={{ padding: "4px 0" }}>osascript, screencapture, cliclick (optional)</td></tr>
                <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Tools (Linux)</td><td style={{ padding: "4px 0" }}>xdotool, scrot, wmctrl</td></tr>
                <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Tools (Windows)</td><td style={{ padding: "4px 0" }}>PowerShell (built-in)</td></tr>
                <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Detected Windows</td><td style={{ padding: "4px 0" }}>{windows.length} apps</td></tr>
              </tbody>
            </table>
          </div>
        </div>
      )}
      </div>
    </div>
  );
}
