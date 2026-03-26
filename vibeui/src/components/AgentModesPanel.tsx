import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

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

interface ModesResponse {
  modes: ModeInfo[];
  activeMode: string;
}

const AgentModesPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("modes");
  const [activeMode, setActiveMode] = useState<string>("smart");
  const [modes, setModes] = useState<ModeInfo[]>([]);
  const [stats, setStats] = useState<ModeStats[]>([]);
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [newName, setNewName] = useState("");
  const [newBase, setNewBase] = useState("smart");
  const [newMaxTokens, setNewMaxTokens] = useState("4096");
  const [newTemp, setNewTemp] = useState("0.5");

  const loadModes = useCallback(async () => {
    try {
      const resp = await invoke<ModesResponse>("get_agent_modes");
      setModes(resp.modes);
      setActiveMode(resp.activeMode);
    } catch (err) {
      setError(String(err));
    }
  }, []);

  const loadStats = useCallback(async () => {
    try {
      const resp = await invoke<ModeStats[]>("get_agent_mode_stats");
      setStats(resp);
    } catch (err) {
      setError(String(err));
    }
  }, []);

  const loadProfiles = useCallback(async () => {
    try {
      const resp = await invoke<Profile[]>("get_agent_mode_profiles");
      setProfiles(resp);
    } catch (err) {
      setError(String(err));
    }
  }, []);

  useEffect(() => {
    const init = async () => {
      setLoading(true);
      await Promise.all([loadModes(), loadStats(), loadProfiles()]);
      setLoading(false);
    };
    init();
  }, [loadModes, loadStats, loadProfiles]);

  const handleSetMode = async (modeId: string) => {
    try {
      setError(null);
      const newActive = await invoke<string>("set_active_agent_mode", { modeId });
      setActiveMode(newActive);
      await loadStats();
    } catch (err) {
      setError(String(err));
    }
  };

  const addProfile = async () => {
    if (!newName) return;
    try {
      setError(null);
      const profile = await invoke<Profile>("create_agent_mode_profile", {
        name: newName,
        baseMode: newBase,
        maxTokens: parseInt(newMaxTokens, 10),
        temperature: parseFloat(newTemp),
      });
      setProfiles((prev) => [...prev, profile]);
      setNewName("");
      setNewMaxTokens("4096");
      setNewTemp("0.5");
    } catch (err) {
      setError(String(err));
    }
  };

  const containerStyle: React.CSSProperties = {
    padding: "16px",
    color: "var(--text-primary)",
    backgroundColor: "var(--bg-primary)",
    fontFamily: "inherit",
    fontSize: "13px",
    height: "100%",
    overflow: "auto",
  };

  const tabBarStyle: React.CSSProperties = {
    display: "flex",
    gap: "4px",
    borderBottom: "1px solid var(--border-color)",
    marginBottom: "12px",
  };

  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: "8px 16px",
    cursor: "pointer",
    border: "none",
    background: active ? "var(--bg-secondary)" : "transparent",
    color: active ? "var(--text-primary)" : "var(--text-secondary)",
    borderBottom: active ? "2px solid var(--accent-blue)" : "2px solid transparent",
    fontFamily: "inherit",
    fontSize: "inherit",
  });

  const cardStyle: React.CSSProperties = {
    padding: "12px",
    marginBottom: "8px",
    borderRadius: "4px",
    backgroundColor: "var(--bg-secondary)",
    border: "1px solid var(--border-color)",
  };

  const btnStyle: React.CSSProperties = {
    padding: "4px 10px",
    border: "1px solid var(--accent-color)",
    background: "var(--accent-color)",
    color: "var(--btn-primary-fg)",
    borderRadius: "3px",
    cursor: "pointer",
    fontFamily: "inherit",
    fontSize: "12px",
  };

  const inputStyle: React.CSSProperties = {
    padding: "4px 8px",
    background: "var(--bg-secondary)",
    color: "var(--text-primary)",
    border: "1px solid var(--border-color)",
    borderRadius: "3px",
    fontFamily: "inherit",
    fontSize: "inherit",
  };

  const tabs = ["modes", "stats", "profiles"];

  return (
    <div style={containerStyle}>
      <h3 style={{ margin: "0 0 12px" }}>Agent Modes</h3>
      {error && (
        <div style={{ padding: "8px", marginBottom: "8px", borderRadius: "4px", backgroundColor: "var(--error-bg)", color: "var(--error-color)", fontSize: "12px" }}>
          {error}
        </div>
      )}
      <div style={tabBarStyle}>
        {tabs.map((t) => (
          <button key={t} style={tabStyle(activeTab === t)} onClick={() => setActiveTab(t)}>
            {t === "modes" ? "Mode Select" : t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {loading && <div style={{ padding: "12px", opacity: 0.6 }}>Loading...</div>}

      {!loading && activeTab === "modes" && (
        <div>
          {modes.map((m) => (
            <div
              key={m.id}
              style={{
                ...cardStyle,
                border: activeMode === m.id ? "2px solid var(--accent-color)" : cardStyle.border,
                cursor: "pointer",
              }}
              onClick={() => handleSetMode(m.id)}
            >
              <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "6px" }}>
                <div style={{ width: "32px", height: "32px", borderRadius: "50%", backgroundColor: "var(--accent-color)", color: "var(--btn-primary-fg)", display: "flex", alignItems: "center", justifyContent: "center", fontWeight: 700 }}>
                  {m.icon}
                </div>
                <div>
                  <strong>{m.name}</strong>
                  {activeMode === m.id && <span style={{ marginLeft: "8px", fontSize: "11px", color: "var(--success-color)" }}>Active</span>}
                </div>
              </div>
              <p style={{ margin: "4px 0 8px", fontSize: "12px", opacity: 0.8 }}>{m.description}</p>
              <div style={{ display: "flex", gap: "6px", flexWrap: "wrap" }}>
                {m.traits.map((t) => (
                  <span key={t} style={{ padding: "2px 8px", borderRadius: "10px", fontSize: "11px", backgroundColor: "var(--bg-tertiary)", color: "var(--btn-primary-fg)" }}>{t}</span>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}

      {!loading && activeTab === "stats" && (
        <div>
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <thead>
              <tr style={{ borderBottom: "1px solid var(--border-color)" }}>
                {["Mode", "Invocations", "Avg Tokens", "Last Used"].map((h) => (
                  <th key={h} style={{ padding: "6px 8px", textAlign: "left", fontSize: "12px", opacity: 0.7 }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {stats.map((s) => (
                <tr key={s.modeId} style={{ borderBottom: "1px solid var(--border-color)" }}>
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
          <button style={{ ...btnStyle, marginTop: "8px" }} onClick={loadStats}>Refresh</button>
        </div>
      )}

      {!loading && activeTab === "profiles" && (
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
