import React, { useState } from "react";

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

// -- Mock Data ----------------------------------------------------------------

const MOCK_INSTANCES: SandboxInstance[] = [
  { id: "sb-001", name: "api-prototype", template: "Rust", state: "Running", url: "https://sb-001.sandbox.vibe.dev", owner: "alice", cpu: 2, memoryGb: 4, diskGb: 20, createdAt: "2026-03-09 08:00", expiresAt: "2026-03-10 08:00" },
  { id: "sb-002", name: "ml-experiment", template: "Python", state: "Running", url: "https://sb-002.sandbox.vibe.dev", owner: "bob", cpu: 4, memoryGb: 8, diskGb: 50, createdAt: "2026-03-09 10:30", expiresAt: "2026-03-10 10:30" },
  { id: "sb-003", name: "frontend-spike", template: "Node", state: "Stopped", url: "https://sb-003.sandbox.vibe.dev", owner: "alice", cpu: 1, memoryGb: 2, diskGb: 10, createdAt: "2026-03-08 14:00", expiresAt: "2026-03-09 14:00" },
  { id: "sb-004", name: "data-pipeline", template: "Python", state: "Creating", url: "https://sb-004.sandbox.vibe.dev", owner: "carol", cpu: 2, memoryGb: 4, diskGb: 30, createdAt: "2026-03-09 15:00", expiresAt: "2026-03-10 15:00" },
  { id: "sb-005", name: "old-demo", template: "Node", state: "Expired", url: "https://sb-005.sandbox.vibe.dev", owner: "bob", cpu: 1, memoryGb: 2, diskGb: 10, createdAt: "2026-03-05 09:00", expiresAt: "2026-03-06 09:00" },
];

const MOCK_TEMPLATES: SandboxTemplate[] = [
  { id: "tpl-rust", name: "Rust", language: "Rust", description: "Rust development environment with cargo, clippy, and rust-analyzer", preinstalled: ["cargo", "clippy", "rust-analyzer", "rustfmt", "cargo-watch"], defaultCpu: 2, defaultMemoryGb: 4, defaultDiskGb: 20 },
  { id: "tpl-node", name: "Node", language: "TypeScript/JavaScript", description: "Node.js 22 with npm, pnpm, and common dev tools", preinstalled: ["node 22", "npm", "pnpm", "typescript", "eslint", "prettier"], defaultCpu: 1, defaultMemoryGb: 2, defaultDiskGb: 10 },
  { id: "tpl-python", name: "Python", language: "Python", description: "Python 3.12 with pip, poetry, and scientific computing libraries", preinstalled: ["python 3.12", "pip", "poetry", "numpy", "pandas", "pytest"], defaultCpu: 2, defaultMemoryGb: 4, defaultDiskGb: 30 },
];

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
  const [instances] = useState<SandboxInstance[]>(MOCK_INSTANCES);
  const [form, setForm] = useState<CreateForm>({ name: "", template: "Rust", cpu: 2, memory: 4, disk: 20 });
  const [error, setError] = useState<string | null>(null);

  const tabs: TabName[] = ["Instances", "Templates", "Create"];

  const handleCreate = () => {
    if (!form.name.trim()) return;
    setError(`Creating sandbox "${form.name}" with template ${form.template} (${form.cpu} CPU, ${form.memory}GB RAM, ${form.disk}GB disk)`);
    setForm({ name: "", template: "Rust", cpu: 2, memory: 4, disk: 20 });
  };

  const handleTemplateSelect = (tpl: SandboxTemplate) => {
    setForm({ ...form, template: tpl.name, cpu: tpl.defaultCpu, memory: tpl.defaultMemoryGb, disk: tpl.defaultDiskGb });
    setTab("Create");
  };

  return (
    <div style={{ padding: 12, fontFamily: "var(--font-family, sans-serif)", fontSize: 13, height: "100%", overflowY: "auto", color: "var(--text-primary)", background: "var(--bg-primary)" }}>
      <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 12 }}>Cloud Sandbox</div>

      {error && <div className="panel-error"><span>{error}</span><button onClick={() => setError(null)}>✕</button></div>}

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
          {MOCK_TEMPLATES.map((tpl) => (
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
            <select value={form.template} onChange={(e) => { const tpl = MOCK_TEMPLATES.find((t) => t.name === e.target.value); if (tpl) { setForm({ ...form, template: tpl.name, cpu: tpl.defaultCpu, memory: tpl.defaultMemoryGb, disk: tpl.defaultDiskGb }); } }} style={{ width: "100%", padding: "6px 10px", fontSize: 12, background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: 4 }}>
              {MOCK_TEMPLATES.map((t) => (
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
          <button onClick={handleCreate} disabled={!form.name.trim()} style={{ width: "100%", padding: "8px 16px", fontSize: 13, borderRadius: 4, border: "none", background: form.name.trim() ? "var(--accent-color)" : "var(--text-muted)", color: "white", cursor: form.name.trim() ? "pointer" : "not-allowed", fontWeight: 600 }}>
            Create Sandbox
          </button>
        </div>
      )}
    </div>
  );
};

export default CloudSandboxPanel;
