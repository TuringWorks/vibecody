import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface WorkflowTemplate {
  id: string;
  name: string;
  description: string;
  estimatedMinutes: number;
  category: string;
}

interface SecretEntry {
  name: string;
  description: string;
  required: boolean;
}

interface HistoryEntry {
  id: string;
  name: string;
  triggers: string[];
  jobs: string[];
  generatedAt: string;
  yaml: string;
}

const GhActionsPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("workflows");
  const [workflowName, setWorkflowName] = useState("ci");
  const [triggers, setTriggers] = useState<Record<string, boolean>>({ push: true, pull_request: true, schedule: false, workflow_dispatch: false });
  const [jobs, setJobs] = useState("build, test, lint");
  const [yamlPreview, setYamlPreview] = useState("");
  const [templates, setTemplates] = useState<WorkflowTemplate[]>([]);
  const [secrets, setSecrets] = useState<SecretEntry[]>([]);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [newSecretName, setNewSecretName] = useState("");
  const [newSecretDesc, setNewSecretDesc] = useState("");
  const [saving, setSaving] = useState(false);
  const [saveResult, setSaveResult] = useState("");
  const [error, setError] = useState("");

  const loadTemplates = useCallback(async () => {
    try {
      const result = await invoke<WorkflowTemplate[]>("list_gh_workflow_templates");
      setTemplates(result);
    } catch (e) {
      setError(`Failed to load templates: ${e}`);
    }
  }, []);

  const loadSecrets = useCallback(async () => {
    try {
      const result = await invoke<SecretEntry[]>("list_gh_secrets");
      setSecrets(result);
    } catch (e) {
      setError(`Failed to load secrets: ${e}`);
    }
  }, []);

  const loadHistory = useCallback(async () => {
    try {
      const result = await invoke<HistoryEntry[]>("get_gh_actions_history");
      setHistory(Array.isArray(result) ? result : []);
    } catch (e) {
      setError(`Failed to load history: ${e}`);
    }
  }, []);

  useEffect(() => {
    loadTemplates();
    loadSecrets();
    loadHistory();
  }, [loadTemplates, loadSecrets, loadHistory]);

  const generateYaml = async () => {
    setError("");
    const activeTriggers = Object.entries(triggers).filter(([, v]) => v).map(([k]) => k);
    const jobList = jobs.split(",").map(j => j.trim()).filter(Boolean);
    try {
      const yaml = await invoke<string>("generate_gh_workflow", {
        config: { name: workflowName, triggers: activeTriggers, jobs: jobList },
      });
      setYamlPreview(yaml);
      loadHistory();
    } catch (e) {
      setError(`Failed to generate workflow: ${e}`);
    }
  };

  const saveWorkflow = async () => {
    if (!yamlPreview) return;
    setSaving(true);
    setSaveResult("");
    setError("");
    const filename = `${workflowName.replace(/[^a-zA-Z0-9_-]/g, "_")}.yml`;
    try {
      const path = await invoke<string>("save_gh_workflow", { filename, yaml: yamlPreview });
      setSaveResult(`Saved to ${path}`);
    } catch (e) {
      setError(`Failed to save workflow: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const generateFromTemplate = async (template: WorkflowTemplate) => {
    setError("");
    const defaultJobs: Record<string, string[]> = {
      CodeReview: ["review"],
      AutoFix: ["lint-fix", "type-fix"],
      TestSuite: ["unit-tests", "integration-tests", "e2e-tests"],
      SecurityScan: ["sast", "dependency-audit", "secret-scan"],
      Deploy: ["build", "push", "deploy"],
      Release: ["version", "changelog", "release"],
      Custom: ["build"],
    };
    try {
      const yaml = await invoke<string>("generate_gh_workflow", {
        config: {
          name: template.name.toLowerCase().replace(/\s+/g, "-"),
          triggers: ["push", "pull_request"],
          jobs: defaultJobs[template.name] || ["build"],
        },
      });
      setYamlPreview(yaml);
      setActiveTab("workflows");
      loadHistory();
    } catch (e) {
      setError(`Failed to generate from template: ${e}`);
    }
  };

  const addSecret = () => {
    if (!newSecretName) return;
    setSecrets(prev => [...prev, { name: newSecretName, description: newSecretDesc, required: false }]);
    setNewSecretName("");
    setNewSecretDesc("");
  };

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3 style={{ margin: 0 }}>GitHub Actions</h3>
      </div>
      {error && (
        <div className="panel-error">{error}</div>
      )}
      <div className="panel-tab-bar">
        {["workflows", "templates", "secrets", "history"].map(t => (
          <button key={t} className={`panel-tab${activeTab === t ? " active" : ""}`} onClick={() => setActiveTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      <div className="panel-body">
        {activeTab === "workflows" && (
          <div>
            <div className="panel-card">
              <h4 style={{ margin: "0 0 12px" }}>Workflow Configuration</h4>
              <div style={{ marginBottom: "12px" }}>
                <label className="panel-label">Workflow Name</label>
                <input style={{ padding: "6px 10px", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", backgroundColor: "var(--bg-secondary)", color: "var(--text-primary)", boxSizing: "border-box", width: "100%" }} value={workflowName} onChange={e => setWorkflowName(e.target.value)} />
              </div>
              <div style={{ marginBottom: "12px" }}>
                <label className="panel-label">Triggers</label>
                <div style={{ display: "flex", gap: "16px", flexWrap: "wrap" }}>
                  {Object.entries(triggers).map(([key, val]) => (
                    <label key={key} style={{ display: "flex", alignItems: "center", gap: "4px" }}>
                      <input type="checkbox" checked={val} onChange={e => setTriggers(prev => ({ ...prev, [key]: e.target.checked }))} />
                      {key}
                    </label>
                  ))}
                </div>
              </div>
              <div style={{ marginBottom: "12px" }}>
                <label className="panel-label">Jobs (comma-separated)</label>
                <input style={{ padding: "6px 10px", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", backgroundColor: "var(--bg-secondary)", color: "var(--text-primary)", boxSizing: "border-box", width: "100%" }} value={jobs} onChange={e => setJobs(e.target.value)} />
              </div>
              <div style={{ display: "flex", gap: "8px" }}>
                <button className="panel-btn panel-btn-primary" onClick={generateYaml}>Generate YAML</button>
              </div>
            </div>
            {yamlPreview && (
              <div className="panel-card">
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "8px" }}>
                  <h4 style={{ margin: 0 }}>YAML Output</h4>
                  <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                    {saveResult && <span style={{ fontSize: "var(--font-size-base)", color: "var(--success-color, #4ade80)" }}>{saveResult}</span>}
                    <button className="panel-btn panel-btn-primary" onClick={saveWorkflow} disabled={saving}>
                      {saving ? "Saving..." : "Save to .github/workflows/"}
                    </button>
                  </div>
                </div>
                <pre style={{ margin: 0, padding: "12px", borderRadius: "var(--radius-xs-plus)", backgroundColor: "var(--bg-secondary)", overflow: "auto", fontSize: "var(--font-size-base)", lineHeight: 1.5 }}>
                  {yamlPreview}
                </pre>
              </div>
            )}
          </div>
        )}

        {activeTab === "templates" && (
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(260px, 1fr))", gap: "8px" }}>
            {templates.map(t => (
              <div key={t.id} className="panel-card">
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "8px" }}>
                  <strong>{t.name}</strong>
                  <span style={{ opacity: 0.6, fontSize: "var(--font-size-sm)" }}>{t.category}</span>
                </div>
                <p style={{ margin: "0 0 8px", opacity: 0.8, fontSize: "var(--font-size-md)" }}>{t.description}</p>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <span style={{ opacity: 0.6, fontSize: "var(--font-size-base)" }}>~{t.estimatedMinutes} min</span>
                  <button className="panel-btn panel-btn-primary" onClick={() => generateFromTemplate(t)}>Generate</button>
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === "secrets" && (
          <div>
            <h4 style={{ margin: "0 0 12px" }}>Required Secrets</h4>
            {secrets.map((s, i) => (
              <div key={i} className="panel-card" style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <div>
                  <strong>{s.name}</strong>
                  {s.required && <span style={{ marginLeft: "6px", fontSize: "var(--font-size-sm)", color: "var(--error-color)" }}>required</span>}
                  <div style={{ opacity: 0.7, fontSize: "var(--font-size-base)", marginTop: "2px" }}>{s.description}</div>
                </div>
              </div>
            ))}
            <div className="panel-card" style={{ marginTop: "16px" }}>
              <h4 style={{ margin: "0 0 8px" }}>Add Secret</h4>
              <div style={{ display: "flex", gap: "8px" }}>
                <input style={{ flex: 1, padding: "6px 10px", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", backgroundColor: "var(--bg-secondary)", color: "var(--text-primary)", boxSizing: "border-box" }} placeholder="SECRET_NAME" value={newSecretName} onChange={e => setNewSecretName(e.target.value)} />
                <input style={{ flex: 2, padding: "6px 10px", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", backgroundColor: "var(--bg-secondary)", color: "var(--text-primary)", boxSizing: "border-box" }} placeholder="Description" value={newSecretDesc} onChange={e => setNewSecretDesc(e.target.value)} />
                <button className="panel-btn panel-btn-primary" onClick={addSecret}>Add</button>
              </div>
            </div>
          </div>
        )}

        {activeTab === "history" && (
          <div>
            <h4 style={{ margin: "0 0 12px" }}>Generated Workflows</h4>
            {history.length === 0 && <p style={{ opacity: 0.6 }}>No workflows generated yet.</p>}
            {history.map((h, i) => (
              <div key={h.id || i} className="panel-card">
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "8px" }}>
                  <strong>{h.name}</strong>
                  <span style={{ opacity: 0.6, fontSize: "var(--font-size-sm)" }}>{h.generatedAt ? new Date(h.generatedAt).toLocaleString() : ""}</span>
                </div>
                <div style={{ fontSize: "var(--font-size-base)", opacity: 0.7, marginBottom: "4px" }}>
                  Triggers: {(h.triggers || []).join(", ")} | Jobs: {(h.jobs || []).join(", ")}
                </div>
                <button
                  className="panel-btn panel-btn-secondary"
                  style={{ fontSize: "var(--font-size-sm)", padding: "4px 10px" }}
                  onClick={() => { setYamlPreview(h.yaml || ""); setWorkflowName(h.name); setActiveTab("workflows"); }}
                >
                  View YAML
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default GhActionsPanel;
