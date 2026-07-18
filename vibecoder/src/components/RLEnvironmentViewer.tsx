/**
 * RLEnvironmentViewer — Environment management panel.
 *
 * Slice 3 — productionized. Reads from the daemon's `EnvStore` (the
 * `rl_environments` table in workspace.db). On first load, the daemon
 * seeds a minimal Gymnasium classic-control bundle so the panel is
 * never empty even before `vibe-rl-py` is installed.
 *
 * Affordances:
 * - Source filter (Gymnasium / PettingZoo / Custom)
 * - Search box
 * - Refresh: invokes `python -m vibe_rl probe-envs` via the daemon and
 *   upserts every discovered env. Surfaces a structured error when the
 *   sidecar isn't installed yet.
 * - Register custom Python env: dialog accepts a workspace-relative
 *   `.py` path that defines a `gymnasium.Env` subclass.
 * - Delete: only allowed for `custom_*` sources.
 */
import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";

interface EnvWire {
  id: string;
  name: string;
  version: string;
  source: string;
  observationSpace: string;
  actionSpace: string;
  rewardComponents: string[] | unknown[];
  backend: string;
  entryPoint?: string | null;
  filePath?: string | null;
  parentEnvId?: string | null;
  spec?: unknown;
}

interface RefreshReport {
  source: string;
  added: number;
  updated: number;
  total: number;
  sidecar_invoked: boolean;
  error?: string | null;
}

const badgeStyle: React.CSSProperties = {
  fontSize: "var(--font-size-xs)",
  padding: "2px 6px",
  borderRadius: 3,
  background: "var(--accent-blue)",
  color: "var(--btn-primary-fg, #fff)",
  marginLeft: 6,
};

const sourceOptions = [
  { value: "", label: "All sources" },
  { value: "gymnasium", label: "Gymnasium" },
  { value: "pettingzoo", label: "PettingZoo" },
  { value: "custom_python", label: "Custom (Python)" },
];

export function RLEnvironmentViewer(props: { workspacePath?: string | null }) {
  const workspacePath = props.workspacePath ?? null;
  const [envs, setEnvs] = useState<EnvWire[]>([]);
  const [detail, setDetail] = useState<EnvWire | null>(null);
  const [loadState, setLoadState] = useState<"idle" | "loading" | "loaded" | "error">("idle");
  const [error, setError] = useState<string | null>(null);
  const [sourceFilter, setSourceFilter] = useState<string>("");
  const [search, setSearch] = useState<string>("");
  const [refreshing, setRefreshing] = useState(false);
  const [refreshReport, setRefreshReport] = useState<RefreshReport | null>(null);
  const [registerOpen, setRegisterOpen] = useState(false);
  const [regName, setRegName] = useState("");
  const [regVersion, setRegVersion] = useState("0.1.0");
  const [regFilePath, setRegFilePath] = useState("");
  const [regError, setRegError] = useState<string | null>(null);

  const fetchEnvs = useCallback(async () => {
    if (!workspacePath) {
      setLoadState("error");
      setError("Open a workspace before browsing RL environments.");
      setEnvs([]);
      return;
    }
    setLoadState("loading");
    setError(null);
    try {
      const res = await invoke<EnvWire[]>("rl_list_environments", { workspacePath });
      setEnvs(res);
      setLoadState("loaded");
    } catch (e) {
      setError(String(e));
      setLoadState("error");
    }
  }, [workspacePath]);

  useEffect(() => { fetchEnvs(); }, [fetchEnvs]);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    return envs.filter(e => {
      if (sourceFilter && e.source !== sourceFilter) return false;
      if (q && !e.name.toLowerCase().includes(q)) return false;
      return true;
    });
  }, [envs, sourceFilter, search]);

  const selectEnv = useCallback(async (id: string) => {
    if (!workspacePath) return;
    try {
      const res = await invoke<EnvWire>("rl_get_environment", { workspacePath, envId: id });
      setDetail(res);
    } catch (e) {
      setError(String(e));
    }
  }, [workspacePath]);

  const refresh = useCallback(async () => {
    if (!workspacePath) return;
    setRefreshing(true);
    setRefreshReport(null);
    try {
      const r = await invoke<RefreshReport>("rl_refresh_environments", { workspacePath });
      setRefreshReport(r);
      fetchEnvs();
    } catch (e) {
      setRefreshReport({ source: "gymnasium", added: 0, updated: 0, total: 0, sidecar_invoked: false, error: String(e) });
    }
    setRefreshing(false);
  }, [workspacePath, fetchEnvs]);

  const submitRegister = useCallback(async () => {
    if (!workspacePath) return;
    setRegError(null);
    if (!regName.trim()) { setRegError("Name is required."); return; }
    if (!regFilePath.trim()) { setRegError("File path is required."); return; }
    try {
      await invoke("rl_register_custom_environment", {
        workspacePath,
        config: JSON.stringify({ name: regName.trim(), version: regVersion.trim() || "0.1.0", filePath: regFilePath.trim() }),
      });
      setRegisterOpen(false);
      setRegName("");
      setRegFilePath("");
      setRegVersion("0.1.0");
      fetchEnvs();
    } catch (e) {
      setRegError(String(e));
    }
  }, [workspacePath, regName, regVersion, regFilePath, fetchEnvs]);

  const deleteEnv = useCallback(async (envId: string) => {
    if (!workspacePath) return;
    try {
      await invoke("rl_delete_environment", { workspacePath, envId });
      if (detail?.id === envId) setDetail(null);
      fetchEnvs();
    } catch (e) {
      setError(String(e));
    }
  }, [workspacePath, detail, fetchEnvs]);

  const sourceColor = (s: string) =>
    s === "gymnasium" ? "var(--accent-blue)" :
    s === "pettingzoo" ? "var(--accent-purple, #8b5cf6)" :
    s === "custom_python" ? "var(--success-color)" :
    "var(--text-secondary)";

  return (
    <div className="panel-container">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <h2 style={{ margin: 0, fontSize: "var(--font-size-xl)", fontWeight: 600, color: "var(--text-primary)" }}>RL Environments</h2>
        <div style={{ display: "flex", gap: 6 }}>
          <button className="panel-btn panel-btn-secondary" onClick={() => setRegisterOpen(true)} disabled={!workspacePath}>+ Register Custom</button>
          <button className="panel-btn panel-btn-secondary" onClick={refresh} disabled={!workspacePath || refreshing}>
            {refreshing ? "Refreshing…" : "Refresh"}
          </button>
        </div>
      </div>

      <div className="panel-card" style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap", marginBottom: 8 }}>
        <select className="panel-select" value={sourceFilter} onChange={e => setSourceFilter(e.target.value)} style={{ minWidth: 140 }}>
          {sourceOptions.map(s => <option key={s.value} value={s.value}>{s.label}</option>)}
        </select>
        <input
          className="panel-input"
          placeholder="Search name…"
          value={search}
          onChange={e => setSearch(e.target.value)}
          style={{ flex: 1, minWidth: 160 }}
        />
        <span className="panel-label">{filtered.length} of {envs.length}</span>
      </div>

      {refreshReport && (
        <div className="panel-card" style={{ borderColor: refreshReport.error ? "var(--warning-color)" : "var(--success-color)", marginBottom: 8 }}>
          {refreshReport.error ? (
            <>
              <div className="panel-label" style={{ color: "var(--warning-color)" }}>Sidecar refresh unavailable</div>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 4 }}>
                Install the sidecar with <code>cd vibe-rl-py &amp;&amp; uv sync</code>, then retry.
              </div>
              <pre style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", whiteSpace: "pre-wrap", margin: 0 }}>{refreshReport.error}</pre>
            </>
          ) : (
            <>
              <div className="panel-label" style={{ color: "var(--success-color)" }}>Refresh complete</div>
              <div style={{ fontSize: "var(--font-size-sm)" }}>
                {refreshReport.source}: <strong>{refreshReport.added}</strong> added, <strong>{refreshReport.updated}</strong> updated, <strong>{refreshReport.total}</strong> total.
              </div>
            </>
          )}
        </div>
      )}

      {error && (
        <div className="panel-card" style={{ borderColor: "var(--error-color)", marginBottom: 8 }}>
          <div className="panel-label" style={{ color: "var(--error-color)" }}>{error}</div>
        </div>
      )}

      {registerOpen && (
        <div className="panel-card" style={{ borderColor: "var(--accent-blue)", marginBottom: 8 }}>
          <div className="panel-label" style={{ marginBottom: 8 }}>Register Custom Python Environment</div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 8 }}>
            <div>
              <span className="panel-label">Name</span>
              <input className="panel-input" value={regName} onChange={e => setRegName(e.target.value)} placeholder="my-trading-env" />
            </div>
            <div>
              <span className="panel-label">Version</span>
              <input className="panel-input" value={regVersion} onChange={e => setRegVersion(e.target.value)} placeholder="0.1.0" />
            </div>
          </div>
          <div style={{ marginBottom: 8 }}>
            <span className="panel-label">File path (workspace-relative)</span>
            <input className="panel-input" value={regFilePath} onChange={e => setRegFilePath(e.target.value)} placeholder="envs/my_env.py" />
            <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginTop: 4 }}>
              Must be inside the workspace and define a <code>gymnasium.Env</code> subclass. Slice 3.5 will introspect the spec automatically.
            </div>
          </div>
          {regError && <div style={{ color: "var(--error-color)", fontSize: "var(--font-size-sm)", marginBottom: 4 }}>{regError}</div>}
          <div style={{ display: "flex", gap: 6 }}>
            <button className="panel-btn panel-btn-primary" onClick={submitRegister}>Register</button>
            <button className="panel-btn panel-btn-secondary" onClick={() => { setRegisterOpen(false); setRegError(null); }}>Cancel</button>
          </div>
        </div>
      )}

      <div className="panel-card">
        <div className="panel-label">Environments</div>
        {loadState === "loading" && <div className="panel-loading">Loading environments…</div>}
        {loadState !== "loading" && filtered.map(e => (
          <div
            key={e.id}
            style={{ padding: "6px 0", borderBottom: "1px solid var(--border-color)", cursor: "pointer", display: "flex", justifyContent: "space-between", alignItems: "center" }}
            onClick={() => selectEnv(e.id)}
          >
            <span>
              <span style={{ fontWeight: 600 }}>{e.name}</span>
              <span style={badgeStyle}>v{e.version}</span>
              <span style={{ ...badgeStyle, background: sourceColor(e.source) }}>{e.source}</span>
            </span>
            <span className="panel-label">{e.observationSpace} → {e.actionSpace}</span>
          </div>
        ))}
        {loadState === "loaded" && filtered.length === 0 && (
          <div className="panel-empty">
            {envs.length === 0
              ? "No environments yet. Click Refresh to probe Gymnasium, or register a custom env."
              : "No environments match the current filter."}
          </div>
        )}
      </div>

      {detail && (
        <div className="panel-card">
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
            <div style={{ fontWeight: 600 }}>
              {detail.name} <span style={badgeStyle}>v{detail.version}</span>
              <span style={{ ...badgeStyle, background: sourceColor(detail.source) }}>{detail.source}</span>
            </div>
            <div style={{ display: "flex", gap: 6 }}>
              {detail.source.startsWith("custom") && (
                <button className="panel-btn panel-btn-danger" onClick={() => deleteEnv(detail.id)}>Delete</button>
              )}
              <button className="panel-btn panel-btn-secondary" onClick={() => setDetail(null)}>Close</button>
            </div>
          </div>
          <div className="panel-label">env_id</div>
          <div style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-xs)", marginBottom: 8 }}>{detail.id}</div>
          <div className="panel-label">Observation → Action</div>
          <div style={{ marginBottom: 8 }}>{detail.observationSpace} → {detail.actionSpace}</div>
          {detail.entryPoint && (
            <>
              <div className="panel-label">Entry point</div>
              <div style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-xs)", marginBottom: 8 }}>{detail.entryPoint}</div>
            </>
          )}
          {detail.filePath && (
            <>
              <div className="panel-label">File path</div>
              <div style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-xs)", marginBottom: 8 }}>{detail.filePath}</div>
            </>
          )}
          {Array.isArray(detail.rewardComponents) && detail.rewardComponents.length > 0 && (
            <>
              <div className="panel-label">Reward Components</div>
              <div style={{ display: "flex", gap: 4, flexWrap: "wrap", marginBottom: 8 }}>
                {(detail.rewardComponents as unknown[]).map((c, i) => (
                  <span key={i} style={{ ...badgeStyle, background: "var(--bg-tertiary)", color: "var(--text-primary)" }}>{String(c)}</span>
                ))}
              </div>
            </>
          )}
          <div className="panel-label">Spec JSON</div>
          <pre style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", overflow: "auto", maxHeight: 200, margin: 0 }}>{JSON.stringify(detail.spec, null, 2)}</pre>
        </div>
      )}
    </div>
  );
}
