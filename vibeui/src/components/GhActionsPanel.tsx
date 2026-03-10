import React, { useState } from "react";

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

const GhActionsPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("workflows");
  const [workflowName, setWorkflowName] = useState("ci");
  const [triggers, setTriggers] = useState<Record<string, boolean>>({ push: true, pull_request: true, schedule: false, workflow_dispatch: false });
  const [jobs, setJobs] = useState("build, test, lint");
  const [yamlPreview, setYamlPreview] = useState("");
  const [templates] = useState<WorkflowTemplate[]>([
    { id: "t1", name: "CodeReview", description: "AI-powered code review on PRs with inline suggestions", estimatedMinutes: 3, category: "Quality" },
    { id: "t2", name: "AutoFix", description: "Automatically fix lint and type errors, push corrections", estimatedMinutes: 5, category: "Automation" },
    { id: "t3", name: "TestSuite", description: "Run unit, integration, and e2e tests with coverage report", estimatedMinutes: 8, category: "Testing" },
    { id: "t4", name: "SecurityScan", description: "SAST, dependency audit, and secret detection scan", estimatedMinutes: 4, category: "Security" },
    { id: "t5", name: "Deploy", description: "Build, push container, deploy to staging or production", estimatedMinutes: 10, category: "Deployment" },
    { id: "t6", name: "Custom", description: "Blank workflow template with common boilerplate", estimatedMinutes: 1, category: "Custom" },
  ]);
  const [secrets, setSecrets] = useState<SecretEntry[]>([
    { name: "GITHUB_TOKEN", description: "Automatically provided by GitHub Actions", required: true },
    { name: "DEPLOY_KEY", description: "SSH key for deployment target", required: true },
    { name: "SLACK_WEBHOOK", description: "Webhook URL for Slack notifications", required: false },
  ]);
  const [newSecretName, setNewSecretName] = useState("");
  const [newSecretDesc, setNewSecretDesc] = useState("");

  const containerStyle: React.CSSProperties = {
    padding: "16px", color: "var(--vscode-foreground)",
    backgroundColor: "var(--vscode-editor-background)",
    fontFamily: "var(--vscode-font-family)", fontSize: "var(--vscode-font-size)",
    height: "100%", overflow: "auto",
  };
  const tabBar: React.CSSProperties = { display: "flex", gap: "4px", marginBottom: "16px", borderBottom: "1px solid var(--vscode-panel-border)" };
  const tab = (active: boolean): React.CSSProperties => ({
    padding: "8px 16px", cursor: "pointer", border: "none",
    backgroundColor: active ? "var(--vscode-tab-activeBackground)" : "transparent",
    color: active ? "var(--vscode-tab-activeForeground)" : "var(--vscode-tab-inactiveForeground)",
    borderBottom: active ? "2px solid var(--vscode-focusBorder)" : "2px solid transparent",
  });
  const btn: React.CSSProperties = {
    padding: "6px 14px", border: "none", borderRadius: "4px", cursor: "pointer",
    backgroundColor: "var(--vscode-button-background)", color: "var(--vscode-button-foreground)",
  };
  const input: React.CSSProperties = {
    padding: "6px 10px", borderRadius: "4px", border: "1px solid var(--vscode-input-border)",
    backgroundColor: "var(--vscode-input-background)", color: "var(--vscode-input-foreground)",
  };
  const card: React.CSSProperties = {
    padding: "12px", marginBottom: "8px", borderRadius: "6px",
    backgroundColor: "var(--vscode-editorWidget-background)", border: "1px solid var(--vscode-panel-border)",
  };

  const generateYaml = () => {
    const activeTriggers = Object.entries(triggers).filter(([, v]) => v).map(([k]) => k);
    const jobList = jobs.split(",").map(j => j.trim()).filter(Boolean);
    let yaml = `name: ${workflowName}\n\non:\n`;
    activeTriggers.forEach(t => { yaml += `  ${t}:\n`; });
    yaml += "\njobs:\n";
    jobList.forEach(j => {
      yaml += `  ${j}:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - name: Run ${j}\n        run: echo "Running ${j}"\n\n`;
    });
    setYamlPreview(yaml);
  };

  const addSecret = () => {
    if (!newSecretName) return;
    setSecrets(prev => [...prev, { name: newSecretName, description: newSecretDesc, required: false }]);
    setNewSecretName("");
    setNewSecretDesc("");
  };

  return (
    <div style={containerStyle}>
      <h3 style={{ margin: "0 0 12px" }}>GitHub Actions</h3>
      <div style={tabBar}>
        {["workflows", "templates", "secrets"].map(t => (
          <button key={t} style={tab(activeTab === t)} onClick={() => setActiveTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {activeTab === "workflows" && (
        <div>
          <div style={card}>
            <h4 style={{ margin: "0 0 12px" }}>Workflow Configuration</h4>
            <div style={{ marginBottom: "12px" }}>
              <label style={{ display: "block", marginBottom: "4px", fontWeight: 600 }}>Workflow Name</label>
              <input style={{ ...input, width: "100%" }} value={workflowName} onChange={e => setWorkflowName(e.target.value)} />
            </div>
            <div style={{ marginBottom: "12px" }}>
              <label style={{ display: "block", marginBottom: "4px", fontWeight: 600 }}>Triggers</label>
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
              <label style={{ display: "block", marginBottom: "4px", fontWeight: 600 }}>Jobs (comma-separated)</label>
              <input style={{ ...input, width: "100%" }} value={jobs} onChange={e => setJobs(e.target.value)} />
            </div>
            <button style={btn} onClick={generateYaml}>Validate & Preview YAML</button>
          </div>
          {yamlPreview && (
            <div style={card}>
              <h4 style={{ margin: "0 0 8px" }}>YAML Output</h4>
              <pre style={{ margin: 0, padding: "12px", borderRadius: "4px", backgroundColor: "var(--vscode-textCodeBlock-background)", overflow: "auto", fontSize: "12px", lineHeight: 1.5 }}>
                {yamlPreview}
              </pre>
            </div>
          )}
        </div>
      )}

      {activeTab === "templates" && (
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(260px, 1fr))", gap: "8px" }}>
          {templates.map(t => (
            <div key={t.id} style={card}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "8px" }}>
                <strong>{t.name}</strong>
                <span style={{ opacity: 0.6, fontSize: "11px" }}>{t.category}</span>
              </div>
              <p style={{ margin: "0 0 8px", opacity: 0.8, fontSize: "13px" }}>{t.description}</p>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <span style={{ opacity: 0.6, fontSize: "12px" }}>~{t.estimatedMinutes} min</span>
                <button style={btn}>Generate</button>
              </div>
            </div>
          ))}
        </div>
      )}

      {activeTab === "secrets" && (
        <div>
          <h4 style={{ margin: "0 0 12px" }}>Required Secrets</h4>
          {secrets.map((s, i) => (
            <div key={i} style={{ ...card, display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <div>
                <strong>{s.name}</strong>
                {s.required && <span style={{ marginLeft: "6px", fontSize: "11px", color: "#f85149" }}>required</span>}
                <div style={{ opacity: 0.7, fontSize: "12px", marginTop: "2px" }}>{s.description}</div>
              </div>
            </div>
          ))}
          <div style={{ ...card, marginTop: "16px" }}>
            <h4 style={{ margin: "0 0 8px" }}>Add Secret</h4>
            <div style={{ display: "flex", gap: "8px" }}>
              <input style={{ ...input, flex: 1 }} placeholder="SECRET_NAME" value={newSecretName} onChange={e => setNewSecretName(e.target.value)} />
              <input style={{ ...input, flex: 2 }} placeholder="Description" value={newSecretDesc} onChange={e => setNewSecretDesc(e.target.value)} />
              <button style={btn} onClick={addSecret}>Add</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default GhActionsPanel;
