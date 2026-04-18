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

export function EnvDispatchPanel() {
  const [tab, setTab] = useState("environments");
  const [envs, setEnvs] = useState<EnvCard[]>([]);
  const [tasks, setTasks] = useState<DispatchedTask[]>([]);
  const [status, setStatus] = useState<DispatchStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [maxParallel, setMaxParallel] = useState(4);
  const [costBudget, setCostBudget] = useState(50);

  useEffect(() => {
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
    load();
  }, []);

  const statusColor = (s: string) => {
    if (s === "running") return "var(--success-color)";
    if (s === "idle") return "var(--text-muted)";
    if (s === "failed") return "var(--error-color)";
    return "var(--warning-color)";
  };

  return (
    <div className="panel-container" style={{ padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono)", flex: 1, minHeight: 0, overflowY: "auto" }}>
      <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 700, marginBottom: 12 }}>Env Dispatch</div>

      {status && (
        <div style={{ display: "flex", gap: 12, marginBottom: 16, flexWrap: "wrap" }}>
          {[
            { label: "Envs", value: status.total_envs },
            { label: "Active", value: status.active_tasks, color: "var(--success-color)" },
            { label: "Queued", value: status.queued_tasks, color: "var(--warning-color)" },
            { label: "Failed", value: status.failed_tasks, color: "var(--error-color)" },
          ].map(s => (
            <div key={s.label} style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", padding: "8px 16px", border: "1px solid var(--border-color)", minWidth: 80, textAlign: "center" }}>
              <div style={{ fontSize: 20, fontWeight: 700, color: s.color ?? "var(--text-primary)" }}>{s.value}</div>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)" }}>{s.label}</div>
            </div>
          ))}
        </div>
      )}

      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        {["environments", "tasks", "config"].map(t => (
          <button className="panel-tab" key={t} onClick={() => setTab(t)} style={{ padding: "4px 12px", borderRadius: "var(--radius-sm)", cursor: "pointer", background: tab === t ? "var(--accent-color)" : "var(--bg-secondary)", color: tab === t ? "var(--btn-primary-fg)" : "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)" }}>{t}</button>
        ))}
      </div>

      {loading && <div className="panel-loading" style={{ color: "var(--text-muted)" }}>Loading...</div>}
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8 }}>{error}</div>}

      {!loading && tab === "environments" && (
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))", gap: 12 }}>
          {envs.length === 0 && <div style={{ color: "var(--text-muted)" }}>No environments found.</div>}
          {envs.map(env => (
            <div key={env.id} style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm-alt)", padding: 12 }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{env.name}</span>
                <span style={{ fontSize: "var(--font-size-sm)", padding: "2px 8px", borderRadius: "var(--radius-md)", background: statusColor(env.status) + "22", color: statusColor(env.status), border: `1px solid ${statusColor(env.status)}` }}>{env.status}</span>
              </div>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)", marginBottom: 4 }}>Type: {env.env_type}</div>
              {env.current_task && (
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-primary)", marginBottom: 6, padding: "3px 8px", background: "var(--bg-primary)", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)" }}>
                  Task: {env.current_task}
                </div>
              )}
              <div style={{ marginTop: 6 }}>
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-xs)", color: "var(--text-muted)", marginBottom: 2 }}>
                  <span>CPU</span><span>{env.cpu_pct}%</span>
                </div>
                <div style={{ height: 4, background: "var(--bg-primary)", borderRadius: 2, marginBottom: 4 }}>
                  <div style={{ height: "100%", width: `${env.cpu_pct}%`, background: env.cpu_pct > 80 ? "var(--error-color)" : "var(--accent-color)", borderRadius: 2 }} />
                </div>
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-xs)", color: "var(--text-muted)", marginBottom: 2 }}>
                  <span>MEM</span><span>{env.mem_pct}%</span>
                </div>
                <div style={{ height: 4, background: "var(--bg-primary)", borderRadius: 2 }}>
                  <div style={{ height: "100%", width: `${env.mem_pct}%`, background: env.mem_pct > 80 ? "var(--warning-color)" : "var(--success-color)", borderRadius: 2 }} />
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {!loading && tab === "tasks" && (
        <div style={{ overflowX: "auto" }}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" }}>
            <thead>
              <tr style={{ background: "var(--bg-secondary)" }}>
                {["Task ID", "Env", "Description", "Status", "Started", "Finished"].map(h => (
                  <th key={h} style={{ padding: "8px 12px", textAlign: "left", borderBottom: "1px solid var(--border-color)", color: "var(--text-muted)", fontWeight: 600 }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {tasks.length === 0 && (
                <tr><td colSpan={6} style={{ padding: 16, color: "var(--text-muted)", textAlign: "center" }}>No tasks dispatched.</td></tr>
              )}
              {tasks.map(task => (
                <tr key={task.task_id} style={{ borderBottom: "1px solid var(--border-color)" }}>
                  <td style={{ padding: "8px 12px", fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)" }}>{task.task_id.slice(0, 8)}…</td>
                  <td style={{ padding: "8px 12px" }}>{task.env_id}</td>
                  <td style={{ padding: "8px 12px", maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{task.description}</td>
                  <td style={{ padding: "8px 12px" }}>
                    <span style={{ padding: "2px 8px", borderRadius: "var(--radius-md)", fontSize: "var(--font-size-sm)", background: statusColor(task.status) + "22", color: statusColor(task.status) }}>{task.status}</span>
                  </td>
                  <td style={{ padding: "8px 12px", color: "var(--text-muted)" }}>{task.started_at}</td>
                  <td style={{ padding: "8px 12px", color: "var(--text-muted)" }}>{task.finished_at ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {!loading && tab === "config" && (
        <div style={{ maxWidth: 400 }}>
          <div style={{ marginBottom: 20 }}>
            <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 6 }}>Max Parallel Environments: <strong style={{ color: "var(--text-primary)" }}>{maxParallel}</strong></label>
            <input type="range" min={1} max={16} value={maxParallel} onChange={e => setMaxParallel(Number(e.target.value))}
              style={{ width: "100%", accentColor: "var(--accent-color)" }} />
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-xs)", color: "var(--text-muted)" }}>
              <span>1</span><span>16</span>
            </div>
          </div>
          <div style={{ marginBottom: 20 }}>
            <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 6 }}>Cost Budget (USD/hr): <strong style={{ color: "var(--text-primary)" }}>${costBudget}</strong></label>
            <input type="range" min={1} max={500} value={costBudget} onChange={e => setCostBudget(Number(e.target.value))}
              style={{ width: "100%", accentColor: "var(--accent-color)" }} />
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-xs)", color: "var(--text-muted)" }}>
              <span>$1</span><span>$500</span>
            </div>
          </div>
          <button className="panel-btn" style={{ padding: "8px 20px", borderRadius: "var(--radius-sm)", cursor: "pointer", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontSize: "var(--font-size-md)", fontWeight: 600 }}>
            Apply Config
          </button>
        </div>
      )}
    </div>
  );
}
