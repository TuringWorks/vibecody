/**
 * CompanyDashboardPanel — Company management UI.
 *
 * Create, list, switch, and delete companies. Shows active company
 * status with agent/task counts. All actions accessible via UI forms.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Building2, X } from "lucide-react";

interface CompanyDashboardPanelProps {
  workspacePath?: string | null;
}

interface Company {
  id: string;
  name: string;
  status: string;
  description: string;
  mission: string;
  active: boolean;
}

/** Parse text lines from company_list into Company objects */
function parseCompanies(text: string): Company[] {
  // Format: "▶ [active] Acme  Build great stuff" or "  [active] Acme  desc"
  const lines = text.split("\n").filter(Boolean);
  return lines.map((line) => {
    const active = line.startsWith("▶");
    const m = line.match(/\[(\w+)\]\s+(.*)/);
    if (!m) return null;
    const status = m[1];
    const rest = m[2].trim();
    const spaceIdx = rest.indexOf("  ");
    const name = spaceIdx > -1 ? rest.slice(0, spaceIdx).trim() : rest;
    const description = spaceIdx > -1 ? rest.slice(spaceIdx).trim() : "";
    return { id: name, name, status, description, mission: "", active };
  }).filter(Boolean) as Company[];
}

export function CompanyDashboardPanel({ workspacePath: _wp }: CompanyDashboardPanelProps) {
  const [companies, setCompanies] = useState<Company[]>([]);
  const [rawStatus, setRawStatus] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Create form
  const [showCreate, setShowCreate] = useState(false);
  const [newName, setNewName] = useState("");
  const [newDesc, setNewDesc] = useState("");
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  // Action feedback
  const [actionMsg, setActionMsg] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [listOut, statusOut] = await Promise.all([
        invoke<string>("company_list"),
        invoke<string>("company_status"),
      ]);
      setCompanies(parseCompanies(listOut));
      setRawStatus(statusOut);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const createCompany = async () => {
    if (!newName.trim()) return;
    setCreating(true);
    setCreateError(null);
    try {
      await invoke<string>("company_create", { name: newName.trim(), description: newDesc.trim() || null });
      setNewName("");
      setNewDesc("");
      setShowCreate(false);
      load();
    } catch (e) {
      setCreateError(String(e));
    } finally {
      setCreating(false);
    }
  };

  const switchCompany = async (name: string) => {
    try {
      const out = await invoke<string>("company_switch", { nameOrId: name });
      setActionMsg(out);
      load();
    } catch (e) {
      setActionMsg(`Error: ${e}`);
    }
  };

  const deleteCompany = async (name: string) => {
    if (!confirm(`Archive company "${name}"?`)) return;
    try {
      const out = await invoke<string>("company_delete", { nameOrId: name });
      setActionMsg(out);
      load();
    } catch (e) {
      setActionMsg(`Error: ${e}`);
    }
  };

  return (
    <div className="panel-container">
      {/* Header */}
      <div className="panel-header">
        <h3>Companies</h3>
        <div style={{ display: "flex", gap: 6, marginLeft: "auto" }}>
          <button onClick={() => { setShowCreate(!showCreate); setCreateError(null); }} className="panel-btn panel-btn-secondary">
            {showCreate ? "Cancel" : "+ New Company"}
          </button>
          <button onClick={load} className="panel-btn panel-btn-secondary">Refresh</button>
        </div>
      </div>
      <div className="panel-body">

      {/* Create form */}
      {showCreate && (
        <div className="panel-card" style={{ marginBottom: 16 }}>
          <div style={{ fontWeight: 600, marginBottom: 10, fontSize: 13 }}>Create Company</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <input
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && createCompany()}
              placeholder="Company name *"
              autoFocus
              className="panel-input panel-input-full"
            />
            <input
              value={newDesc}
              onChange={(e) => setNewDesc(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && createCompany()}
              placeholder="Description (optional)"
              className="panel-input panel-input-full"
            />
            {createError && <div className="panel-error" style={{ fontSize: 12 }}>{createError}</div>}
            <div style={{ display: "flex", gap: 8 }}>
              <button
                onClick={createCompany}
                disabled={creating || !newName.trim()}
                className="panel-btn panel-btn-primary"
                style={{ opacity: creating ? 0.6 : 1 }}
              >
                {creating ? "Creating…" : "Create"}
              </button>
              <button onClick={() => setShowCreate(false)} className="panel-btn panel-btn-secondary">Cancel</button>
            </div>
          </div>
        </div>
      )}

      {/* Action feedback */}
      {actionMsg && (
        <div className="panel-card" style={{ marginBottom: 12, fontSize: 12 }}>
          {actionMsg}
          <button onClick={() => setActionMsg(null)} style={{ marginLeft: 8, cursor: "pointer", background: "none", border: "none", color: "var(--text-secondary)", display: "inline-flex" }}><X size={12} /></button>
        </div>
      )}

      {loading && <div className="panel-loading">Loading…</div>}
      {error && <div className="panel-error" style={{ marginBottom: 12 }}>{error}</div>}

      {/* Company list */}
      {!loading && companies.length === 0 && !error && (
        <div className="panel-empty" style={{ padding: 24 }}>
          <div style={{ marginBottom: 8, display: "flex", justifyContent: "center", color: "var(--accent, #4a9eff)" }}><Building2 size={32} strokeWidth={1.5} /></div>
          <div style={{ fontWeight: 600, marginBottom: 4 }}>No companies yet</div>
          <div style={{ color: "var(--text-secondary)", fontSize: 12, marginBottom: 16 }}>
            Create your first company to get started
          </div>
          <button onClick={() => setShowCreate(true)} className="panel-btn panel-btn-primary" style={{ fontSize: 12 }}>
            + Create Company
          </button>
        </div>
      )}

      {companies.map((c) => (
        <div
          key={c.name}
          className="panel-card"
          style={{
            background: c.active ? "var(--selection-bg, rgba(99,179,237,0.1))" : undefined,
            border: `1px solid ${c.active ? "var(--accent, #4a9eff)" : "var(--border-color)"}`,
            display: "flex", alignItems: "center", gap: 10,
          }}
        >
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 2 }}>
              {c.active && <span style={{ fontSize: 10, color: "var(--accent, #4a9eff)", fontWeight: 700 }}>ACTIVE</span>}
              <span style={{ fontWeight: 600, fontSize: 13 }}>{c.name}</span>
              <span style={{
                fontSize: 10, padding: "1px 5px", borderRadius: 3,
                background: "var(--bg-tertiary)", color: "var(--text-secondary)",
              }}>{c.status}</span>
            </div>
            {c.description && (
              <div style={{ fontSize: 11, color: "var(--text-secondary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                {c.description}
              </div>
            )}
          </div>
          <div style={{ display: "flex", gap: 6, flexShrink: 0 }}>
            {!c.active && (
              <button onClick={() => switchCompany(c.name)} className="panel-btn panel-btn-secondary" style={{ fontSize: 10, padding: "2px 8px" }}>
                Switch
              </button>
            )}
            <button onClick={() => deleteCompany(c.name)} className="panel-btn panel-btn-danger" style={{ fontSize: 10, padding: "2px 8px" }}>
              Archive
            </button>
          </div>
        </div>
      ))}

      {/* Active company status */}
      {rawStatus && rawStatus !== "No companies yet.\nUse: /company create <name>" && (
        <div style={{ marginTop: 16 }}>
          <div className="panel-label" style={{ marginBottom: 6, fontWeight: 600 }}>STATUS</div>
          <div className="panel-card">
            <pre style={{ margin: 0, fontSize: 11, whiteSpace: "pre-wrap", lineHeight: 1.5 }}>
              {rawStatus}
            </pre>
          </div>
        </div>
      )}
      </div>
    </div>
  );
}
