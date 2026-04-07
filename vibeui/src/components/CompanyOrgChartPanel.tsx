/**
 * CompanyOrgChartPanel — Agent roster with hire/fire actions.
 *
 * Lists all agents with role, title, status. Hire new agents via form.
 * Fire agents with confirmation. Shows reporting hierarchy tree.
 */
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Users, X } from "lucide-react";

interface CompanyOrgChartPanelProps {
  workspacePath?: string | null;
}

const ROLES = ["ceo", "cto", "cfo", "engineer", "designer", "analyst", "agent", "manager", "hr"];

const STATUS_COLOR: Record<string, string> = {
  idle: "var(--text-secondary)",
  active: "var(--success, #27ae60)",
  paused: "var(--warning, #f39c12)",
  terminated: "var(--danger, #e74c3c)",
};

const STATUS_BADGE: Record<string, string> = {
  idle: "○", active: "●", paused: "‖", terminated: "×",
};

const inputStyle: React.CSSProperties = {
  fontSize: 12, padding: "4px 8px", background: "var(--bg-primary)",
  border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)",
};

export function CompanyOrgChartPanel({ workspacePath: _wp }: CompanyOrgChartPanelProps) {
  const [agentText, setAgentText] = useState<string>("");
  const [treeText, setTreeText] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useState<"list" | "tree" | "hire">("list");

  // Hire form
  const [hireName, setHireName] = useState("");
  const [hireTitle, setHireTitle] = useState("");
  const [hireRole, setHireRole] = useState("agent");
  const [hiring, setHiring] = useState(false);
  const [hireResult, setHireResult] = useState<string | null>(null);

  // Fire
  const [fireId, setFireId] = useState("");
  const [actionMsg, setActionMsg] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const [listOut, treeOut] = await Promise.all([
        invoke<string>("company_agent_list"),
        invoke<string>("company_cmd", { args: "agent tree" }),
      ]);
      setAgentText(listOut);
      setTreeText(treeOut);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const hireAgent = async () => {
    if (!hireName.trim()) return;
    setHiring(true);
    setHireResult(null);
    try {
      const out = await invoke<string>("company_agent_hire", {
        name: hireName.trim(),
        title: hireTitle.trim() || hireName.trim(),
        role: hireRole,
        reportsTo: null,
      });
      setHireResult(out);
      setHireName("");
      setHireTitle("");
      setHireRole("agent");
      load();
    } catch (e) {
      setHireResult(`Error: ${e}`);
    } finally {
      setHiring(false);
    }
  };

  const fireAgent = async (id: string) => {
    if (!id.trim()) return;
    if (!confirm(`Fire agent "${id}"?`)) return;
    try {
      const out = await invoke<string>("company_agent_fire", { id: id.trim() });
      setActionMsg(out);
      setFireId("");
      load();
    } catch (e) {
      setActionMsg(`Error: ${e}`);
    }
  };

  const isEmpty = !agentText || agentText.includes("No agents");

  return (
    <div className="panel-container">
      {/* Header */}
      <div className="panel-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <span style={{ fontWeight: 600, fontSize: 14 }}>Agents</span>
        <div style={{ display: "flex", gap: 6 }}>
          {(["list", "tree", "hire"] as const).map((v) => (
            <button key={v} onClick={() => setView(v)} className={`panel-btn ${view === v ? "panel-btn-primary" : "panel-btn-secondary"}`} style={{ padding: "2px 8px" }}>
              {v === "hire" ? "+ Hire" : v.charAt(0).toUpperCase() + v.slice(1)}
            </button>
          ))}
          <button onClick={load} className="panel-btn panel-btn-secondary">Refresh</button>
        </div>
      </div>
      <div className="panel-body">

      {loading && <div className="panel-loading">Loading…</div>}
      {error && <div className="panel-error" style={{ fontSize: 12, marginBottom: 8 }}>{error}</div>}
      {actionMsg && (
        <div className="panel-card" style={{ marginBottom: 12, fontSize: 12 }}>
          {actionMsg}
          <button onClick={() => setActionMsg(null)} style={{ marginLeft: 8, cursor: "pointer", background: "none", border: "none", color: "var(--text-secondary)", display: "inline-flex" }}><X size={12} /></button>
        </div>
      )}

      {/* Hire form */}
      {view === "hire" && (
        <div className="panel-card" style={{ marginBottom: 12 }}>
          <div style={{ fontWeight: 600, marginBottom: 10 }}>Hire New Agent</div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 8 }}>
            <input value={hireName} onChange={(e) => setHireName(e.target.value)} placeholder="Name *" style={inputStyle} autoFocus />
            <input value={hireTitle} onChange={(e) => setHireTitle(e.target.value)} placeholder="Title (e.g. Senior Engineer)" style={inputStyle} />
            <select value={hireRole} onChange={(e) => setHireRole(e.target.value)} style={{ ...inputStyle }}>
              {ROLES.map((r) => <option key={r} value={r}>{r}</option>)}
            </select>
            <input placeholder="Reports to (agent name, optional)" style={inputStyle} />
          </div>
          {hireResult && <div style={{ fontSize: 12, marginBottom: 8, color: hireResult.startsWith("Error") ? "var(--danger, #e74c3c)" : "var(--success, #27ae60)" }}>{hireResult}</div>}
          <button onClick={hireAgent} disabled={hiring || !hireName.trim()} className="panel-btn panel-btn-primary" style={{ opacity: hiring ? 0.6 : 1 }}>
            {hiring ? "Hiring…" : "Hire Agent"}
          </button>
        </div>
      )}

      {/* List view */}
      {view === "list" && (
        <>
          {isEmpty && !loading && !error && (
            <div className="panel-empty" style={{ padding: 24 }}>
              <div style={{ marginBottom: 8, display: "flex", justifyContent: "center", color: "var(--accent, #4a9eff)" }}><Users size={32} strokeWidth={1.5} /></div>
              <div style={{ fontWeight: 600, marginBottom: 4 }}>No agents yet</div>
              <div style={{ color: "var(--text-secondary)", fontSize: 12, marginBottom: 16 }}>Hire your first agent to build your team</div>
              <button onClick={() => setView("hire")} className="panel-btn panel-btn-primary" style={{ fontSize: 12 }}>+ Hire Agent</button>
            </div>
          )}
          {!isEmpty && (
            <div className="panel-card" style={{ marginBottom: 12 }}>
              <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap", lineHeight: 1.7, fontFamily: "inherit" }}>
                {agentText.split("\n").filter(Boolean).map((line, i) => {
                  // Parse line: "  [status] name — title (role)"
                  const m = line.match(/\[(\w+)\]\s+(.+?)\s+—\s+(.+?)\s+\((\w+)\)/);
                  if (!m) return <div key={i} style={{ color: "var(--text-secondary)" }}>{line}</div>;
                  const [, status, name, title, role] = m;
                  const color = STATUS_COLOR[status] ?? "var(--text-primary)";
                  const badge = STATUS_BADGE[status] ?? "?";
                  return (
                    <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, padding: "3px 0" }}>
                      <span style={{ color, fontSize: 11 }}>{badge}</span>
                      <span style={{ fontWeight: 500 }}>{name}</span>
                      <span style={{ color: "var(--text-secondary)", fontSize: 11 }}>{title}</span>
                      <span style={{ fontSize: 10, padding: "1px 5px", borderRadius: 3, background: "var(--bg-tertiary)", color: "var(--text-secondary)" }}>{role}</span>
                    </div>
                  );
                })}
              </pre>
            </div>
          )}

          {/* Fire by ID */}
          {!isEmpty && (
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <input
                value={fireId}
                onChange={(e) => setFireId(e.target.value)}
                placeholder="Agent name or ID to fire"
                style={{ ...inputStyle, flex: 1 }}
              />
              <button
                onClick={() => fireAgent(fireId)}
                disabled={!fireId.trim()}
                className="panel-btn panel-btn-danger"
                style={{ opacity: fireId.trim() ? 1 : 0.5 }}
              >
                Fire
              </button>
            </div>
          )}
        </>
      )}

      {/* Tree view */}
      {view === "tree" && (
        <div className="panel-card">
          {treeText ? (
            <pre style={{ margin: 0, fontSize: 12, whiteSpace: "pre-wrap", lineHeight: 1.7 }}>{treeText}</pre>
          ) : (
            <div className="panel-empty">No org chart yet.</div>
          )}
        </div>
      )}
      </div>
    </div>
  );
}
