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

  const statusColor = (s: string) => s === "active" ? "var(--success-color)" : s === "draft" ? "var(--warning-color)" : "var(--text-secondary)";

  return (
    <div className="panel-container">
      <h2 style={{ margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" }}>RL Environments</h2>

      <div className="panel-card">
        <div className="panel-label">Environments</div>
        {envs.map(e => (
          <div key={e.id} style={{ padding: "6px 0", borderBottom: "1px solid var(--border-color)", cursor: "pointer", display: "flex", justifyContent: "space-between", alignItems: "center" }} onClick={() => selectEnv(e.id)}>
            <span>
              <span style={{ fontWeight: 600 }}>{e.name}</span>
              <span style={badgeStyle}>v{e.version}</span>
              <span style={{ ...badgeStyle, background: statusColor(e.status) }}>{e.status}</span>
            </span>
            <span className="panel-label">obs:{e.obsSpaceDims} act:{e.actionSpaceDims}</span>
          </div>
        ))}
        {envs.length === 0 && <div className="panel-empty">No environments found.</div>}
      </div>

      {loading && <div className="panel-loading">Loading environment details...</div>}
      {detail && !loading && (
        <>
          <div className="panel-card">
            <div style={{ fontWeight: 600, marginBottom: 8 }}>{detail.name} <span style={badgeStyle}>v{detail.version}</span></div>
            <div className="panel-label">Observation Schema</div>
            {Object.entries(detail.observationSchema).map(([k, v]) => (
              <div key={k} style={{ display: "flex", gap: 8, fontSize: 12, padding: "2px 0" }}><span style={{ fontWeight: 600 }}>{k}:</span><span>{v}</span></div>
            ))}
            <div className="panel-label" style={{ marginTop: 8 }}>Action Schema</div>
            {Object.entries(detail.actionSchema).map(([k, v]) => (
              <div key={k} style={{ display: "flex", gap: 8, fontSize: 12, padding: "2px 0" }}><span style={{ fontWeight: 600 }}>{k}:</span><span>{v}</span></div>
            ))}
            <div className="panel-label" style={{ marginTop: 8 }}>Reward Components</div>
            <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
              {detail.rewardComponents.map(c => <span key={c} style={{ ...badgeStyle, background: "var(--bg-tertiary)", color: "var(--text-primary)" }}>{c}</span>)}
            </div>
            <div className="panel-label" style={{ marginTop: 8 }}>Connectors</div>
            <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
              {detail.connectors.map(c => <span key={c} style={{ ...badgeStyle, background: "var(--bg-tertiary)", color: "var(--text-primary)" }}>{c}</span>)}
            </div>
          </div>

          <div className="panel-card">
            <div className="panel-label">Version History</div>
            {detail.versionHistory.map(v => (
              <div key={v.version} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "4px 0", borderBottom: "1px solid var(--border-color)" }}>
                <div><span style={{ fontWeight: 600 }}>v{v.version}</span> <span className="panel-label">{v.author} — {v.note}</span></div>
                <button className="panel-btn panel-btn-secondary" onClick={() => deploy(detail.id, v.version)}>Deploy</button>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
