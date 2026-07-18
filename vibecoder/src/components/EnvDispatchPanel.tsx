import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface EnvCard {
  id: string;
  name: string;
  env_type: string;
  status: string;
  current_task: string | null;
  cpu_pct: number;
  mem_pct: number;
}

interface DispatchedTask {
  task_id: string;
  env_id: string;
  description: string;
  status: string;
  started_at: string;
  finished_at: string | null;
}

interface DispatchStatus {
  total_envs: number;
  active_tasks: number;
  queued_tasks: number;
  failed_tasks: number;
}

type TagIntent = "info" | "success" | "warning" | "danger" | "neutral";

function statusIntent(s: string): TagIntent {
  switch (s) {
    case "running":
    case "completed":
      return "success";
    case "queued":
    case "starting":
      return "info";
    case "failed":
      return "danger";
    case "idle":
    case "cancelled":
    case "unconfigured":
      return "neutral";
    default:
      return "warning";
  }
}

function progressColor(pct: number, dangerAt = 80): "accent" | "warning" | "danger" {
  if (pct >= dangerAt) return "danger";
  if (pct >= 60) return "warning";
  return "accent";
}

export function EnvDispatchPanel() {
  const [tab, setTab] = useState<"environments" | "tasks" | "config">("environments");
  const [envs, setEnvs] = useState<EnvCard[]>([]);
  const [tasks, setTasks] = useState<DispatchedTask[]>([]);
  const [status, setStatus] = useState<DispatchStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [maxParallel, setMaxParallel] = useState(4);
  const [costBudget, setCostBudget] = useState(50);

  async function load() {
    setLoading(true);
    setError(null);
    try {
      const [envsRes, tasksRes, statusRes] = await Promise.all([
        invoke<EnvCard[]>("env_dispatch_list"),
        invoke<DispatchedTask[]>("env_dispatch_task"),
        invoke<DispatchStatus>("env_dispatch_status"),
      ]);
      setEnvs(Array.isArray(envsRes) ? envsRes : []);
      setTasks(Array.isArray(tasksRes) ? tasksRes : []);
      setStatus(statusRes ?? null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => { load(); }, []);

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>Env Dispatch</h3>
        <button
          className="panel-btn panel-btn-secondary panel-btn-sm"
          style={{ marginLeft: "auto" }}
          onClick={load}
          disabled={loading}
        >
          Refresh
        </button>
      </div>

      <div className="panel-body">
        {status && (
          <div className="panel-stats" style={{ marginBottom: 12 }}>
            <div className="panel-stat">
              <div className="panel-stat-value">{status.total_envs}</div>
              <div className="panel-stat-label">Envs</div>
            </div>
            <div className="panel-stat">
              <div className="panel-stat-value" style={{ color: "var(--success-color)" }}>{status.active_tasks}</div>
              <div className="panel-stat-label">Active</div>
            </div>
            <div className="panel-stat">
              <div className="panel-stat-value" style={{ color: "var(--warning-color)" }}>{status.queued_tasks}</div>
              <div className="panel-stat-label">Queued</div>
            </div>
            <div className="panel-stat">
              <div className="panel-stat-value" style={{ color: "var(--error-color)" }}>{status.failed_tasks}</div>
              <div className="panel-stat-label">Failed</div>
            </div>
          </div>
        )}

        <div className="panel-tab-bar" style={{ marginBottom: 12 }}>
          {(["environments", "tasks", "config"] as const).map(t => (
            <button
              key={t}
              className={`panel-tab${tab === t ? " active" : ""}`}
              onClick={() => setTab(t)}
            >
              {t}
            </button>
          ))}
        </div>

        {loading && <div className="panel-loading">Loading…</div>}
        {error && (
          <div className="panel-error">
            <span>{error}</span>
            <button onClick={() => setError(null)} aria-label="dismiss">✕</button>
          </div>
        )}

        {!loading && tab === "environments" && (
          envs.length === 0 ? (
            <div className="panel-empty">No environments found.</div>
          ) : (
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))", gap: 12 }}>
              {envs.map(env => (
                <div key={env.id} className="panel-card">
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                    <span style={{ fontWeight: 600 }}>{env.name}</span>
                    <span className={`panel-tag panel-tag-${statusIntent(env.status)}`}>{env.status}</span>
                  </div>
                  <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)", marginBottom: 6 }}>
                    Type: {env.env_type}
                  </div>
                  {env.current_task && (
                    <div style={{ fontSize: "var(--font-size-sm)", marginBottom: 8, padding: "3px 8px", background: "var(--bg-primary)", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)" }}>
                      Task: {env.current_task}
                    </div>
                  )}
                  <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-xs)", color: "var(--text-muted)", marginBottom: 2 }}>
                    <span>CPU</span><span>{env.cpu_pct}%</span>
                  </div>
                  <div className="progress-bar progress-bar-sm" style={{ marginBottom: 6 }}>
                    <div className={`progress-bar-fill progress-bar-${progressColor(env.cpu_pct)}`} style={{ width: `${env.cpu_pct}%` }} />
                  </div>
                  <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-xs)", color: "var(--text-muted)", marginBottom: 2 }}>
                    <span>MEM</span><span>{env.mem_pct}%</span>
                  </div>
                  <div className="progress-bar progress-bar-sm">
                    <div className={`progress-bar-fill progress-bar-${progressColor(env.mem_pct)}`} style={{ width: `${env.mem_pct}%` }} />
                  </div>
                </div>
              ))}
            </div>
          )
        )}

        {!loading && tab === "tasks" && (
          tasks.length === 0 ? (
            <div className="panel-empty">No tasks dispatched.</div>
          ) : (
            <div style={{ overflowX: "auto" }}>
              <table className="panel-table">
                <thead>
                  <tr>
                    {["Task ID", "Env", "Description", "Status", "Started", "Finished"].map(h => (
                      <th key={h}>{h}</th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {tasks.map(task => (
                    <tr key={task.task_id}>
                      <td className="panel-mono" style={{ fontSize: "var(--font-size-sm)" }}>{task.task_id.slice(0, 8)}…</td>
                      <td>{task.env_id}</td>
                      <td style={{ maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{task.description}</td>
                      <td><span className={`panel-tag panel-tag-${statusIntent(task.status)}`}>{task.status}</span></td>
                      <td style={{ color: "var(--text-muted)" }}>{task.started_at}</td>
                      <td style={{ color: "var(--text-muted)" }}>{task.finished_at ?? "—"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )
        )}

        {!loading && tab === "config" && (
          <div style={{ maxWidth: 400 }}>
            <div style={{ marginBottom: 20 }}>
              <label className="panel-label" style={{ display: "block" }}>
                Max Parallel Environments: <strong style={{ color: "var(--text-primary)" }}>{maxParallel}</strong>
              </label>
              <input
                type="range"
                min={1}
                max={16}
                value={maxParallel}
                onChange={e => setMaxParallel(Number(e.target.value))}
                style={{ width: "100%", accentColor: "var(--accent-color)" }}
              />
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-xs)", color: "var(--text-muted)" }}>
                <span>1</span><span>16</span>
              </div>
            </div>
            <div style={{ marginBottom: 20 }}>
              <label className="panel-label" style={{ display: "block" }}>
                Cost Budget (USD/hr): <strong style={{ color: "var(--text-primary)" }}>${costBudget}</strong>
              </label>
              <input
                type="range"
                min={1}
                max={500}
                value={costBudget}
                onChange={e => setCostBudget(Number(e.target.value))}
                style={{ width: "100%", accentColor: "var(--accent-color)" }}
              />
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-xs)", color: "var(--text-muted)" }}>
                <span>$1</span><span>$500</span>
              </div>
            </div>
            <button className="panel-btn panel-btn-primary">Apply Config</button>
          </div>
        )}
      </div>
    </div>
  );
}
