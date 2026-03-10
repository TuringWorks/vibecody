import React, { useState } from "react";

interface ModeInfo {
  id: string;
  name: string;
  description: string;
  icon: string;
  traits: string[];
}

interface ModeStats {
  modeId: string;
  invocations: number;
  avgTokens: number;
  lastUsed: string;
}

interface Profile {
  id: string;
  name: string;
  baseMode: string;
  maxTokens: number;
  temperature: number;
}

const AgentModesPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("modes");
  const [activeMode, setActiveMode] = useState<string>("smart");

  const modes: ModeInfo[] = [
    { id: "smart", name: "Smart", description: "Balanced mode with context-aware tool selection and moderate token usage.", icon: "S", traits: ["Context-aware", "Balanced cost", "Auto tool selection"] },
    { id: "rush", name: "Rush", description: "Fast execution with minimal deliberation. Best for simple, well-defined tasks.", icon: "R", traits: ["Low latency", "Minimal reasoning", "Direct answers"] },
    { id: "deep", name: "Deep", description: "Thorough analysis with extended reasoning chains and comprehensive exploration.", icon: "D", traits: ["Extended thinking", "Multi-file analysis", "High accuracy"] },
  ];

  const [stats] = useState<ModeStats[]>([
    { modeId: "smart", invocations: 247, avgTokens: 1840, lastUsed: "2 min ago" },
    { modeId: "rush", invocations: 89, avgTokens: 620, lastUsed: "1 hr ago" },
    { modeId: "deep", invocations: 34, avgTokens: 4200, lastUsed: "3 hrs ago" },
  ]);

  const [profiles, setProfiles] = useState<Profile[]>([
    { id: "1", name: "Code Review", baseMode: "deep", maxTokens: 8000, temperature: 0.3 },
    { id: "2", name: "Quick Fix", baseMode: "rush", maxTokens: 2000, temperature: 0.1 },
  ]);

  const [newName, setNewName] = useState("");
  const [newBase, setNewBase] = useState("smart");
  const [newMaxTokens, setNewMaxTokens] = useState("4096");
  const [newTemp, setNewTemp] = useState("0.5");

  const containerStyle: React.CSSProperties = {
    padding: "16px",
    color: "var(--vscode-foreground)",
    backgroundColor: "var(--vscode-editor-background)",
    fontFamily: "var(--vscode-font-family)",
    fontSize: "var(--vscode-font-size)",
    height: "100%",
    overflow: "auto",
  };

  const tabBarStyle: React.CSSProperties = {
    display: "flex",
    gap: "4px",
    borderBottom: "1px solid var(--vscode-panel-border)",
    marginBottom: "12px",
  };

  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: "8px 16px",
    cursor: "pointer",
    border: "none",
    background: active ? "var(--vscode-tab-activeBackground)" : "transparent",
    color: active ? "var(--vscode-tab-activeForeground)" : "var(--vscode-tab-inactiveForeground)",
    borderBottom: active ? "2px solid var(--vscode-focusBorder)" : "2px solid transparent",
    fontFamily: "inherit",
    fontSize: "inherit",
  });

  const cardStyle: React.CSSProperties = {
    padding: "12px",
    marginBottom: "8px",
    borderRadius: "4px",
    backgroundColor: "var(--vscode-editorWidget-background)",
    border: "1px solid var(--vscode-editorWidget-border)",
  };

  const btnStyle: React.CSSProperties = {
    padding: "4px 10px",
    border: "1px solid var(--vscode-button-border, var(--vscode-focusBorder))",
    background: "var(--vscode-button-background)",
    color: "var(--vscode-button-foreground)",
    borderRadius: "3px",
    cursor: "pointer",
    fontFamily: "inherit",
    fontSize: "12px",
  };

  const inputStyle: React.CSSProperties = {
    padding: "4px 8px",
    background: "var(--vscode-input-background)",
    color: "var(--vscode-input-foreground)",
    border: "1px solid var(--vscode-input-border)",
    borderRadius: "3px",
    fontFamily: "inherit",
    fontSize: "inherit",
  };

  const addProfile = () => {
    if (!newName) return;
    const p: Profile = { id: String(Date.now()), name: newName, baseMode: newBase, maxTokens: parseInt(newMaxTokens, 10), temperature: parseFloat(newTemp) };
    setProfiles((prev) => [...prev, p]);
    setNewName("");
    setNewMaxTokens("4096");
    setNewTemp("0.5");
  };

  const tabs = ["modes", "stats", "profiles"];

  return (
    <div style={containerStyle}>
      <h3 style={{ margin: "0 0 12px" }}>Agent Modes</h3>
      <div style={tabBarStyle}>
        {tabs.map((t) => (
          <button key={t} style={tabStyle(activeTab === t)} onClick={() => setActiveTab(t)}>
            {t === "modes" ? "Mode Select" : t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {activeTab === "modes" && (
        <div>
          {modes.map((m) => (
            <div
              key={m.id}
              style={{
                ...cardStyle,
                border: activeMode === m.id ? "2px solid var(--vscode-focusBorder)" : cardStyle.border,
                cursor: "pointer",
              }}
              onClick={() => setActiveMode(m.id)}
            >
              <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "6px" }}>
                <div style={{ width: "32px", height: "32px", borderRadius: "50%", backgroundColor: "var(--vscode-button-background)", color: "var(--vscode-button-foreground)", display: "flex", alignItems: "center", justifyContent: "center", fontWeight: 700 }}>
                  {m.icon}
                </div>
                <div>
                  <strong>{m.name}</strong>
                  {activeMode === m.id && <span style={{ marginLeft: "8px", fontSize: "11px", color: "var(--vscode-testing-iconPassed)" }}>Active</span>}
                </div>
              </div>
              <p style={{ margin: "4px 0 8px", fontSize: "12px", opacity: 0.8 }}>{m.description}</p>
              <div style={{ display: "flex", gap: "6px", flexWrap: "wrap" }}>
                {m.traits.map((t) => (
                  <span key={t} style={{ padding: "2px 8px", borderRadius: "10px", fontSize: "11px", backgroundColor: "var(--vscode-badge-background)", color: "var(--vscode-badge-foreground)" }}>{t}</span>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}

      {activeTab === "stats" && (
        <div>
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <thead>
              <tr style={{ borderBottom: "1px solid var(--vscode-panel-border)" }}>
                {["Mode", "Invocations", "Avg Tokens", "Last Used"].map((h) => (
                  <th key={h} style={{ padding: "6px 8px", textAlign: "left", fontSize: "12px", opacity: 0.7 }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {stats.map((s) => (
                <tr key={s.modeId} style={{ borderBottom: "1px solid var(--vscode-panel-border)" }}>
                  <td style={{ padding: "8px" }}><strong>{modes.find((m) => m.id === s.modeId)?.name}</strong></td>
                  <td style={{ padding: "8px" }}>{s.invocations}</td>
                  <td style={{ padding: "8px" }}>{s.avgTokens.toLocaleString()}</td>
                  <td style={{ padding: "8px", fontSize: "12px", opacity: 0.7 }}>{s.lastUsed}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <div style={{ marginTop: "12px", fontSize: "12px", opacity: 0.6 }}>
            Total invocations: {stats.reduce((a, s) => a + s.invocations, 0)}
          </div>
        </div>
      )}

      {activeTab === "profiles" && (
        <div>
          {profiles.map((p) => (
            <div key={p.id} style={cardStyle}>
              <div style={{ display: "flex", justifyContent: "space-between" }}>
                <strong>{p.name}</strong>
                <span style={{ fontSize: "12px", opacity: 0.7 }}>Base: {p.baseMode}</span>
              </div>
              <div style={{ fontSize: "12px", marginTop: "4px", opacity: 0.7 }}>
                Max tokens: {p.maxTokens} | Temperature: {p.temperature}
              </div>
            </div>
          ))}
          <div style={{ ...cardStyle, marginTop: "12px" }}>
            <div style={{ fontWeight: 600, marginBottom: "8px" }}>New Profile</div>
            <div style={{ display: "flex", gap: "8px", flexWrap: "wrap", alignItems: "center" }}>
              <input style={{ ...inputStyle, width: "120px" }} placeholder="Name" value={newName} onChange={(e) => setNewName(e.target.value)} />
              <select style={inputStyle} value={newBase} onChange={(e) => setNewBase(e.target.value)}>
                <option value="smart">Smart</option>
                <option value="rush">Rush</option>
                <option value="deep">Deep</option>
              </select>
              <input style={{ ...inputStyle, width: "80px" }} placeholder="Max tokens" value={newMaxTokens} onChange={(e) => setNewMaxTokens(e.target.value)} />
              <input style={{ ...inputStyle, width: "60px" }} placeholder="Temp" value={newTemp} onChange={(e) => setNewTemp(e.target.value)} />
              <button style={btnStyle} onClick={addProfile}>Create</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default AgentModesPanel;
