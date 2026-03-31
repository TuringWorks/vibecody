/**
 * RLEnvironmentViewer — Environment management panel.
 *
 * List RL environments with version badges, view observation/action space schemas,
 * reward components, connectors, version history, and deploy/rollback controls.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface EnvSummary {
  id: string;
  name: string;
  version: string;
  status: string;
  obsSpaceDims: number;
  actionSpaceDims: number;
}

interface EnvDetail {
  id: string;
  name: string;
  version: string;
  observationSchema: Record<string, string>;
  actionSchema: Record<string, string>;
  rewardComponents: string[];
  connectors: string[];
  versionHistory: VersionEntry[];
}

interface VersionEntry {
  version: string;
  timestamp: number;
  author: string;
  note: string;
}

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12, marginRight: 8 };
const badgeStyle: React.CSSProperties = { fontSize: 10, padding: "2px 6px", borderRadius: 3, background: "var(--accent-blue)", color: "#fff", marginLeft: 6 };

export function RLEnvironmentViewer() {
  const [envs, setEnvs] = useState<EnvSummary[]>([]);
  const [detail, setDetail] = useState<EnvDetail | null>(null);
  const [loading, setLoading] = useState(false);

  const fetchEnvs = useCallback(async () => {
    try {
      const res = await invoke<EnvSummary[]>("rl_list_environments");
      setEnvs(res);
    } catch (e) { console.error(e); }
  }, []);

  useEffect(() => { fetchEnvs(); }, [fetchEnvs]);

  const selectEnv = useCallback(async (id: string) => {
    setLoading(true);
    try {
      const res = await invoke<EnvDetail>("rl_get_environment", { envId: id });
      setDetail(res);
    } catch (e) { console.error(e); }
    setLoading(false);
  }, []);

  const deploy = useCallback(async (envId: string, version: string) => {
    try {
      await invoke("rl_deploy_environment", { envId, version });
      fetchEnvs();
      if (detail?.id === envId) selectEnv(envId);
    } catch (e) { console.error(e); }
  }, [fetchEnvs, detail, selectEnv]);

  const statusColor = (s: string) => s === "active" ? "#4caf50" : s === "draft" ? "#ff9800" : "var(--text-secondary)";

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>RL Environments</h2>

      <div style={cardStyle}>
        <div style={labelStyle}>Environments</div>
        {envs.map(e => (
          <div key={e.id} style={{ padding: "6px 0", borderBottom: "1px solid var(--border-color)", cursor: "pointer", display: "flex", justifyContent: "space-between", alignItems: "center" }} onClick={() => selectEnv(e.id)}>
            <span>
              <span style={{ fontWeight: 600 }}>{e.name}</span>
              <span style={badgeStyle}>v{e.version}</span>
              <span style={{ ...badgeStyle, background: statusColor(e.status) }}>{e.status}</span>
            </span>
            <span style={labelStyle}>obs:{e.obsSpaceDims} act:{e.actionSpaceDims}</span>
          </div>
        ))}
        {envs.length === 0 && <div style={labelStyle}>No environments found.</div>}
      </div>

      {loading && <div style={labelStyle}>Loading environment details...</div>}
      {detail && !loading && (
        <>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, marginBottom: 8 }}>{detail.name} <span style={badgeStyle}>v{detail.version}</span></div>
            <div style={labelStyle}>Observation Schema</div>
            {Object.entries(detail.observationSchema).map(([k, v]) => (
              <div key={k} style={{ display: "flex", gap: 8, fontSize: 12, padding: "2px 0" }}><span style={{ fontWeight: 600 }}>{k}:</span><span>{v}</span></div>
            ))}
            <div style={{ ...labelStyle, marginTop: 8 }}>Action Schema</div>
            {Object.entries(detail.actionSchema).map(([k, v]) => (
              <div key={k} style={{ display: "flex", gap: 8, fontSize: 12, padding: "2px 0" }}><span style={{ fontWeight: 600 }}>{k}:</span><span>{v}</span></div>
            ))}
            <div style={{ ...labelStyle, marginTop: 8 }}>Reward Components</div>
            <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
              {detail.rewardComponents.map(c => <span key={c} style={{ ...badgeStyle, background: "var(--bg-tertiary)", color: "var(--text-primary)" }}>{c}</span>)}
            </div>
            <div style={{ ...labelStyle, marginTop: 8 }}>Connectors</div>
            <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
              {detail.connectors.map(c => <span key={c} style={{ ...badgeStyle, background: "var(--bg-tertiary)", color: "var(--text-primary)" }}>{c}</span>)}
            </div>
          </div>

          <div style={cardStyle}>
            <div style={labelStyle}>Version History</div>
            {detail.versionHistory.map(v => (
              <div key={v.version} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "4px 0", borderBottom: "1px solid var(--border-color)" }}>
                <div><span style={{ fontWeight: 600 }}>v{v.version}</span> <span style={labelStyle}>{v.author} — {v.note}</span></div>
                <button style={btnStyle} onClick={() => deploy(detail.id, v.version)}>Deploy</button>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
