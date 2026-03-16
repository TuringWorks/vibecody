import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// -- Types --------------------------------------------------------------------

type InstanceState = "Creating" | "Running" | "Stopped" | "Expired";
type TabName = "Instances" | "Templates" | "Create";

interface SandboxInstance {
  id: string;
  name: string;
  template: string;
  state: InstanceState;
  url: string;
  owner: string;
  cpu: number;
  memoryGb: number;
  diskGb: number;
  createdAt: string;
  expiresAt: string;
  logs: string[];
}

interface SandboxTemplate {
  id: string;
  name: string;
  language: string;
  description: string;
  preinstalled: string[];
  defaultCpu: number;
  defaultMemoryGb: number;
  defaultDiskGb: number;
}

interface CreateForm {
  name: string;
  template: string;
  cpu: number;
  memory: number;
  disk: number;
}

// -- Helpers ------------------------------------------------------------------

const stateColor = (s: InstanceState): string => {
  switch (s) {
    case "Creating": return "var(--warning-color)";
    case "Running": return "var(--success-color)";
    case "Stopped": return "var(--text-muted)";
    case "Expired": return "var(--error-color)";
  }
};

// -- Component ----------------------------------------------------------------

const CloudSandboxPanel: React.FC = () => {
  const [tab, setTab] = useState<TabName>("Instances");
  const [instances, setInstances] = useState<SandboxInstance[]>([]);
  const [templates, setTemplates] = useState<SandboxTemplate[]>([]);
  const [form, setForm] = useState<CreateForm>({ name: "", template: "Rust", cpu: 2, memory: 4, disk: 20 });
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const tabs: TabName[] = ["Instances", "Templates", "Create"];

  const loadInstances = useCallback(async () => {
    try {
      const data = await invoke<SandboxInstance[]>("list_cloud_sandboxes");
      setInstances(data);
    } catch (e) {
      setError(`Failed to load instances: ${e}`);
    }
  }, []);

  const loadTemplates = useCallback(async () => {
    try {
      const data = await invoke<SandboxTemplate[]>("get_cloud_sandbox_templates");
      setTemplates(data);
    } catch (e) {
      setError(`Failed to load templates: ${e}`);
    }
  }, []);

  useEffect(() => {
    loadInstances();
    loadTemplates();
  }, [loadInstances, loadTemplates]);

  const handleCreate = async () => {
    if (!form.name.trim()) return;
    setLoading(true);
    setError(null);
    try {
      await invoke<SandboxInstance>("create_cloud_sandbox", {
        name: form.name,
        template: form.template,
        cpu: form.cpu,
        memoryGb: form.memory,
        diskGb: form.disk,
      });
      setForm({ name: "", template: "Rust", cpu: 2, memory: 4, disk: 20 });
      await loadInstances();
      setTab("Instances");
    } catch (e) {
      setError(`Failed to create sandbox: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleStop = async (id: string) => {
    setError(null);
    try {
      await invoke("stop_cloud_sandbox", { id });
      await loadInstances();
    } catch (e) {
      setError(`Failed to stop sandbox: ${e}`);
    }
  };

  const handleDelete = async (id: string) => {
    setError(null);
    try {
      await invoke("delete_cloud_sandbox", { id });
      await loadInstances();
    } catch (e) {
      setError(`Failed to delete sandbox: ${e}`);
    }
  };

  const handleTemplateSelect = (tpl: SandboxTemplate) => {
    setForm({ ...form, template: tpl.name, cpu: tpl.defaultCpu, memory: tpl.defaultMemoryGb, disk: tpl.defaultDiskGb });
    setTab("Create");
  };

  return (
    <div style={{ padding: 12, fontFamily: "var(--font-family, sans-serif)", fontSize: 13, height: "100%", overflowY: "auto", color: "var(--text-primary)", background: "var(--bg-primary)" }}>
      <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 12 }}>Cloud Sandbox</div>

      {error && <div className="panel-error"><span>{error}</span><button onClick={() => setError(null)}>&#x2715;</button></div>}

      {/* Tab bar */}
      <div style={{ display: "flex", gap: 0, marginBottom: 12, borderBottom: "1px solid var(--border-color)" }}>
        {tabs.map((t) => (
          <button key={t} onClick={() => setTab(t)} style={{ padding: "6px 16px", fontSize: 12, background: "none", border: "none", borderBottom: tab === t ? "2px solid var(--accent-color)" : "2px solid transparent", color: tab === t ? "var(--text-primary)" : "var(--text-muted)", cursor: "pointer", fontWeight: tab === t ? 600 : 400 }}>
            {t}
          </button>
        ))}
      </div>

      {/* Instances Tab */}
      {tab === "Instances" && (
        <div>
          {instances.map((inst) => (
            <div key={inst.id} style={{ padding: "8px 10px", marginBottom: 6, borderRadius: 4, background: "var(--bg-secondary)", borderLeft: `3px solid ${stateColor(inst.state)}` }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span style={{ fontWeight: 600, fontSize: 12 }}>{inst.name}</span>
                <span style={{ fontSize: 10, padding: "2px 8px", borderRadius: 10, background: stateColor(inst.state), color: "white", fontWeight: 600 }}>{inst.state}</span>
              </div>
              <div style={{ display: "flex", gap: 10, marginTop: 4, fontSize: 11, color: "var(--text-muted)", flexWrap: "wrap" }}>
                <span>{inst.template}</span>
                <span>{inst.cpu} CPU / {inst.memoryGb}GB RAM / {inst.diskGb}GB disk</span>
                <span>Owner: {inst.owner}</span>
              </div>
              <div style={{ display: "flex", gap: 8, marginTop: 6, alignItems: "center" }}>
                {inst.state === "Running" && (
                  <a href={inst.url} target="_blank" rel="noopener noreferrer" style={{ fontSize: 11, color: "var(--accent-color)", textDecoration: "none" }}>{inst.url}</a>
                )}
                {(inst.state === "Running" || inst.state === "Creating") && (
                  <button onClick={() => handleStop(inst.id)} style={{ fontSize: 10, padding: "2px 8px", borderRadius: 3, border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", cursor: "pointer" }}>Stop</button>
                )}
                {(inst.state === "Stopped" || inst.state === "Expired") && (
                  <button onClick={() => handleDelete(inst.id)} style={{ fontSize: 10, padding: "2px 8px", borderRadius: 3, border: "1px solid var(--error-color)", background: "var(--bg-primary)", color: "var(--error-color)", cursor: "pointer" }}>Delete</button>
                )}
                <span style={{ marginLeft: "auto", fontSize: 10, color: "var(--text-muted)" }}>Created: {inst.createdAt}</span>
              </div>
            </div>
          ))}
          {instances.length === 0 && (
            <div style={{ textAlign: "center", padding: 30, color: "var(--text-muted)" }}>No sandbox instances. Create one from the Create tab.</div>
          )}
        </div>
      )}

      {/* Templates Tab */}
      {tab === "Templates" && (
        <div>
          {templates.map((tpl) => (
            <div key={tpl.id} style={{ padding: "10px 12px", marginBottom: 8, borderRadius: 4, background: "var(--bg-secondary)" }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span style={{ fontWeight: 600, fontSize: 13 }}>{tpl.name}</span>
                <button onClick={() => handleTemplateSelect(tpl)} style={{ padding: "4px 12px", fontSize: 11, borderRadius: 4, border: "none", background: "var(--accent-color)", color: "white", cursor: "pointer" }}>Use Template</button>
              </div>
              <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 4 }}>{tpl.language} - {tpl.description}</div>
              <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginTop: 6 }}>
                {tpl.preinstalled.map((pkg) => (
                  <span key={pkg} style={{ fontSize: 10, padding: "2px 6px", borderRadius: 3, background: "var(--border-color)", color: "var(--text-primary)" }}>{pkg}</span>
                ))}
              </div>
              <div style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 6 }}>
                Defaults: {tpl.defaultCpu} CPU / {tpl.defaultMemoryGb}GB RAM / {tpl.defaultDiskGb}GB disk
              </div>
            </div>
          ))}
          {templates.length === 0 && (
            <div style={{ textAlign: "center", padding: 30, color: "var(--text-muted)" }}>Loading templates...</div>
          )}
        </div>
      )}

      {/* Create Tab */}
      {tab === "Create" && (
        <div>
          <div style={{ marginBottom: 12 }}>
            <label style={{ fontSize: 11, color: "var(--text-muted)", display: "block", marginBottom: 4 }}>Instance Name</label>
            <input value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} placeholder="my-sandbox" style={{ width: "100%", padding: "6px 10px", fontSize: 12, background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 4, boxSizing: "border-box" }} />
          </div>
          <div style={{ marginBottom: 12 }}>
            <label style={{ fontSize: 11, color: "var(--text-muted)", display: "block", marginBottom: 4 }}>Template</label>
            <select value={form.template} onChange={(e) => { const tpl = templates.find((t) => t.name === e.target.value); if (tpl) { setForm({ ...form, template: tpl.name, cpu: tpl.defaultCpu, memory: tpl.defaultMemoryGb, disk: tpl.defaultDiskGb }); } }} style={{ width: "100%", padding: "6px 10px", fontSize: 12, background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 4 }}>
              {templates.map((t) => (
                <option key={t.id} value={t.name}>{t.name} ({t.language})</option>
              ))}
            </select>
          </div>
          <div style={{ display: "flex", gap: 12, marginBottom: 12 }}>
            {([["CPU Cores", "cpu", 1, 16], ["Memory (GB)", "memory", 1, 32], ["Disk (GB)", "disk", 5, 100]] as const).map(([label, key, min, max]) => (
              <div key={key} style={{ flex: 1 }}>
                <label style={{ fontSize: 11, color: "var(--text-muted)", display: "block", marginBottom: 4 }}>{label}</label>
                <input type="number" min={min} max={max} value={form[key]} onChange={(e) => setForm({ ...form, [key]: parseInt(e.target.value) || min })} style={{ width: "100%", padding: "6px 10px", fontSize: 12, background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 4, boxSizing: "border-box" }} />
              </div>
            ))}
          </div>
          <div style={{ padding: "8px 12px", borderRadius: 4, background: "var(--bg-secondary)", marginBottom: 12, fontSize: 11, color: "var(--text-muted)" }}>
            Configuration: {form.template} template with {form.cpu} CPU, {form.memory}GB RAM, {form.disk}GB disk. Instance expires in 24 hours.
          </div>
          <button onClick={handleCreate} disabled={!form.name.trim() || loading} style={{ width: "100%", padding: "8px 16px", fontSize: 13, borderRadius: 4, border: "none", background: form.name.trim() && !loading ? "var(--accent-color)" : "var(--text-muted)", color: "white", cursor: form.name.trim() && !loading ? "pointer" : "not-allowed", fontWeight: 600 }}>
            {loading ? "Creating..." : "Create Sandbox"}
          </button>
        </div>
      )}
    </div>
  );
};

export default CloudSandboxPanel;
