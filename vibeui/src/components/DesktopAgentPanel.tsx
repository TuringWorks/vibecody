import { useState } from "react";

type SubTab = "actions" | "windows" | "macros" | "config";

const card: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, border: "1px solid var(--border-color)" };
const btn: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "none", background: "var(--accent-color)", color: "#fff", cursor: "pointer", fontSize: 12, fontWeight: 600 };
const mono: React.CSSProperties = { fontFamily: "var(--font-mono)", fontSize: 11 };

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
  const [windows] = useState<WindowInfo[]>([
    { id: "1", title: "VibeCody — main.rs", app: "VibeCody", focused: true },
    { id: "2", title: "Terminal — zsh", app: "Terminal", focused: false },
    { id: "3", title: "Chrome — GitHub", app: "Chrome", focused: false },
  ]);

  const platform = navigator.platform.includes("Mac") ? "macOS" : navigator.platform.includes("Linux") ? "Linux" : "Windows";

  return (
    <div style={{ padding: 16, display: "flex", flexDirection: "column", gap: 16, fontSize: 13, color: "var(--text-primary)" }}>
      <div style={{ display: "flex", gap: 2, borderBottom: "1px solid var(--border-color)", marginBottom: 4 }}>
        {(["actions", "windows", "macros", "config"] as const).map(t => (
          <button key={t} onClick={() => setTab(t)} style={{
            padding: "6px 12px", border: "none", background: "transparent", cursor: "pointer",
            borderBottom: tab === t ? "2px solid var(--accent-color)" : "2px solid transparent",
            color: tab === t ? "var(--accent-color)" : "var(--text-secondary)", fontSize: 12, fontFamily: "inherit", textTransform: "capitalize",
          }}>{t}</button>
        ))}
      </div>

      {tab === "actions" && (
        <div>
          <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>Desktop Actions</div>
          <p style={{ fontSize: 12, color: "var(--text-secondary)", margin: "0 0 12px" }}>
            Cross-platform GUI automation: mouse, keyboard, and window control. Detected platform: <strong>{platform}</strong>
          </p>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
            <div style={card}>
              <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>Mouse</div>
              <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
                <div>
                  <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>X</span>
                  <input type="number" value={mouseX} onChange={e => setMouseX(+e.target.value)} style={{ ...mono, width: 60, padding: "3px 6px", border: "1px solid var(--border-color)", borderRadius: 3, background: "var(--bg-tertiary)", color: "var(--text-primary)", marginLeft: 4 }} />
                </div>
                <div>
                  <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>Y</span>
                  <input type="number" value={mouseY} onChange={e => setMouseY(+e.target.value)} style={{ ...mono, width: 60, padding: "3px 6px", border: "1px solid var(--border-color)", borderRadius: 3, background: "var(--bg-tertiary)", color: "var(--text-primary)", marginLeft: 4 }} />
                </div>
              </div>
              <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                <button style={{ ...btn, fontSize: 11, padding: "4px 10px" }}>Move</button>
                <button style={{ ...btn, fontSize: 11, padding: "4px 10px" }}>Click</button>
                <button style={{ ...btn, fontSize: 11, padding: "4px 10px" }}>Double-Click</button>
                <button style={{ ...btn, fontSize: 11, padding: "4px 10px" }}>Right-Click</button>
              </div>
            </div>

            <div style={card}>
              <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>Keyboard</div>
              <div style={{ marginBottom: 8 }}>
                <input style={{ ...mono, width: "100%", padding: "4px 8px", border: "1px solid var(--border-color)", borderRadius: 3, background: "var(--bg-tertiary)", color: "var(--text-primary)" }} value={typeText} onChange={e => setTypeText(e.target.value)} placeholder="Text to type..." />
              </div>
              <div style={{ display: "flex", gap: 4, marginBottom: 8 }}>
                <button style={{ ...btn, fontSize: 11, padding: "4px 10px" }}>Type Text</button>
              </div>
              <div style={{ marginBottom: 4 }}>
                <input style={{ ...mono, width: "100%", padding: "4px 8px", border: "1px solid var(--border-color)", borderRadius: 3, background: "var(--bg-tertiary)", color: "var(--text-primary)" }} value={keyCombo} onChange={e => setKeyCombo(e.target.value)} placeholder="e.g., ctrl+shift+p" />
              </div>
              <button style={{ ...btn, fontSize: 11, padding: "4px 10px" }}>Press Key Combo</button>
            </div>
          </div>

          <div style={{ ...card, marginTop: 12 }}>
            <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>Quick Actions</div>
            <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
              {["Screenshot", "Get Mouse Position", "Get Screen Size", "Get Active Window"].map(a => (
                <button key={a} style={{ ...btn, fontSize: 11, padding: "4px 10px", background: "var(--bg-tertiary)", color: "var(--text-primary)", border: "1px solid var(--border-color)" }}>{a}</button>
              ))}
            </div>
          </div>
        </div>
      )}

      {tab === "windows" && (
        <div>
          <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>Window Management</div>
          <button style={{ ...btn, marginBottom: 12, fontSize: 11 }}>Refresh Windows</button>
          {windows.map(w => (
            <div key={w.id} style={{ ...card, marginBottom: 6, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontWeight: 600, fontSize: 12 }}>{w.title}</div>
                <div style={{ fontSize: 10, color: "var(--text-secondary)" }}>{w.app}</div>
              </div>
              <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
                {w.focused && <span style={{ fontSize: 10, padding: "1px 6px", borderRadius: 3, background: "#4caf5022", color: "var(--accent-green)" }}>Focused</span>}
                <button style={{ ...btn, fontSize: 10, padding: "3px 8px" }}>Focus</button>
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "macros" && (
        <div>
          <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>Action Macros</div>
          <p style={{ fontSize: 12, color: "var(--text-secondary)", margin: "0 0 12px" }}>Record and replay sequences of desktop actions.</p>
          <div style={card}>
            <div style={{ display: "flex", gap: 8 }}>
              <button style={btn}>Record Macro</button>
              <button style={{ ...btn, background: "var(--bg-tertiary)", color: "var(--text-primary)", border: "1px solid var(--border-color)" }}>Import JSON</button>
            </div>
          </div>
          <div style={{ ...card, marginTop: 8, color: "var(--text-secondary)", fontSize: 12, fontStyle: "italic" }}>No saved macros yet.</div>
        </div>
      )}

      {tab === "config" && (
        <div>
          <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>Desktop Agent Configuration</div>
          <div style={card}>
            <table style={{ width: "100%", fontSize: 12, borderCollapse: "collapse" }}>
              <tbody>
                <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Platform</td><td style={{ padding: "4px 0" }}>{platform}</td></tr>
                <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Action Delay</td><td style={{ padding: "4px 0" }}>100ms</td></tr>
                <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Tools (macOS)</td><td style={{ padding: "4px 0" }}>osascript, screencapture, cliclick (optional)</td></tr>
                <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Tools (Linux)</td><td style={{ padding: "4px 0" }}>xdotool, scrot, wmctrl</td></tr>
                <tr><td style={{ padding: "4px 0", color: "var(--text-secondary)" }}>Tools (Windows)</td><td style={{ padding: "4px 0" }}>PowerShell (built-in)</td></tr>
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
