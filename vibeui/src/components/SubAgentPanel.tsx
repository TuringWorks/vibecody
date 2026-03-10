import React, { useState } from "react";

// -- Types --------------------------------------------------------------------

type AgentStatus = "Running" | "Completed" | "Failed" | "Queued";
type TabName = "Agents" | "Results" | "Config";

interface SubAgent {
  id: string;
  role: string;
  status: AgentStatus;
  contextFiles: string[];
  startedAt: string;
  duration: string | null;
  provider: string;
  turnCount: number;
  maxTurns: number;
}

interface AgentResult {
  agentId: string;
  role: string;
  summary: string;
  findings: string[];
  filesModified: string[];
  completedAt: string;
  success: boolean;
}

interface RoleConfig {
  role: string;
  description: string;
  tools: string[];
  maxTurns: number;
  autoSpawnTriggers: string[];
  enabled: boolean;
}

// -- Mock Data ----------------------------------------------------------------

const MOCK_AGENTS: SubAgent[] = [
  { id: "sa-001", role: "CodeReviewer", status: "Running", contextFiles: ["src/auth/mod.rs", "src/auth/jwt.rs"], startedAt: "15:30:12", duration: null, provider: "Claude", turnCount: 3, maxTurns: 10 },
  { id: "sa-002", role: "TestWriter", status: "Completed", contextFiles: ["src/handlers/users.rs"], startedAt: "15:28:00", duration: "2m 15s", provider: "Claude", turnCount: 8, maxTurns: 10 },
  { id: "sa-003", role: "SecurityAuditor", status: "Completed", contextFiles: ["src/db/queries.rs", "src/middleware/auth.rs", "src/handlers/files.rs"], startedAt: "15:25:00", duration: "4m 30s", provider: "OpenAI", turnCount: 10, maxTurns: 10 },
  { id: "sa-004", role: "DocWriter", status: "Queued", contextFiles: ["src/lib.rs", "src/api/mod.rs"], startedAt: "15:31:00", duration: null, provider: "Gemini", turnCount: 0, maxTurns: 5 },
  { id: "sa-005", role: "Refactorer", status: "Failed", contextFiles: ["src/legacy/compat.rs"], startedAt: "15:20:00", duration: "1m 45s", provider: "Claude", turnCount: 4, maxTurns: 10 },
];

const MOCK_RESULTS: AgentResult[] = [
  { agentId: "sa-002", role: "TestWriter", summary: "Generated 12 unit tests for users handler with 95% coverage", findings: ["Added tests for create_user, get_user, update_user, delete_user", "Covered error cases: 404, 409, 422", "Added integration test for auth flow"], filesModified: ["tests/handlers/users_test.rs"], completedAt: "15:30:15", success: true },
  { agentId: "sa-003", role: "SecurityAuditor", summary: "Found 3 security issues: 1 Critical, 1 High, 1 Medium", findings: ["SQL injection in queries.rs line 42 (Critical)", "Missing CSRF validation in auth middleware (High)", "Verbose error messages expose internals (Medium)"], filesModified: [], completedAt: "15:29:30", success: true },
  { agentId: "sa-005", role: "Refactorer", summary: "Failed to refactor legacy module due to circular dependencies", findings: ["Detected circular dependency between compat.rs and legacy/types.rs", "Suggested manual resolution before automated refactoring"], filesModified: [], completedAt: "15:21:45", success: false },
];

const MOCK_CONFIGS: RoleConfig[] = [
  { role: "CodeReviewer", description: "Reviews code changes for quality, patterns, and best practices", tools: ["read_file", "grep", "glob", "list_files"], maxTurns: 10, autoSpawnTriggers: ["on_pr_created", "on_commit"], enabled: true },
  { role: "TestWriter", description: "Generates unit and integration tests for modified code", tools: ["read_file", "write_file", "run_tests", "grep"], maxTurns: 10, autoSpawnTriggers: ["on_file_modified"], enabled: true },
  { role: "SecurityAuditor", description: "Scans code for security vulnerabilities and misconfigurations", tools: ["read_file", "grep", "glob", "list_files"], maxTurns: 10, autoSpawnTriggers: ["on_pr_created"], enabled: true },
  { role: "DocWriter", description: "Generates and updates documentation for public APIs", tools: ["read_file", "write_file", "grep"], maxTurns: 5, autoSpawnTriggers: [], enabled: true },
  { role: "Refactorer", description: "Suggests and applies code refactoring improvements", tools: ["read_file", "write_file", "grep", "glob", "run_tests"], maxTurns: 10, autoSpawnTriggers: [], enabled: false },
];

// -- Helpers ------------------------------------------------------------------

const statusColor = (s: AgentStatus): string => {
  switch (s) {
    case "Running": return "var(--vscode-charts-green, #4caf50)";
    case "Completed": return "var(--vscode-charts-blue, #007acc)";
    case "Failed": return "var(--vscode-errorForeground, #f44336)";
    case "Queued": return "var(--vscode-charts-yellow, #ff9800)";
  }
};

// -- Component ----------------------------------------------------------------

const SubAgentPanel: React.FC = () => {
  const [tab, setTab] = useState<TabName>("Agents");
  const [configs, setConfigs] = useState<RoleConfig[]>(MOCK_CONFIGS);
  const [expandedAgent, setExpandedAgent] = useState<string | null>(null);

  const tabs: TabName[] = ["Agents", "Results", "Config"];

  const toggleConfig = (role: string) => {
    setConfigs((prev) => prev.map((c) => c.role === role ? { ...c, enabled: !c.enabled } : c));
  };

  return (
    <div style={{ padding: 12, fontFamily: "var(--vscode-font-family, sans-serif)", fontSize: 13, height: "100%", overflowY: "auto", color: "var(--vscode-foreground, #ccc)", background: "var(--vscode-editor-background, #1e1e1e)" }}>
      <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 12 }}>Sub-Agents</div>

      {/* Tab bar */}
      <div style={{ display: "flex", gap: 0, marginBottom: 12, borderBottom: "1px solid var(--vscode-panel-border, #444)" }}>
        {tabs.map((t) => (
          <button key={t} onClick={() => setTab(t)} style={{ padding: "6px 16px", fontSize: 12, background: "none", border: "none", borderBottom: tab === t ? "2px solid var(--vscode-focusBorder, #007acc)" : "2px solid transparent", color: tab === t ? "var(--vscode-foreground, #fff)" : "var(--vscode-disabledForeground, #888)", cursor: "pointer", fontWeight: tab === t ? 600 : 400 }}>
            {t}
          </button>
        ))}
      </div>

      {/* Agents Tab */}
      {tab === "Agents" && (
        <div>
          {MOCK_AGENTS.map((agent) => (
            <div key={agent.id} onClick={() => setExpandedAgent(expandedAgent === agent.id ? null : agent.id)} style={{ padding: "8px 10px", marginBottom: 6, borderRadius: 4, background: "var(--vscode-editor-inactiveSelectionBackground, #2d2d2d)", borderLeft: `3px solid ${statusColor(agent.status)}`, cursor: "pointer" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontWeight: 600, fontSize: 12 }}>{agent.role}</span>
                <span style={{ fontSize: 10, padding: "2px 8px", borderRadius: 10, background: statusColor(agent.status), color: "#fff", fontWeight: 600 }}>{agent.status}</span>
                <span style={{ marginLeft: "auto", fontSize: 10, color: "var(--vscode-disabledForeground, #888)" }}>{agent.provider}</span>
              </div>
              <div style={{ display: "flex", gap: 12, marginTop: 4, fontSize: 11, color: "var(--vscode-disabledForeground, #888)" }}>
                <span>Turn {agent.turnCount}/{agent.maxTurns}</span>
                <span>Started {agent.startedAt}</span>
                {agent.duration && <span>Duration: {agent.duration}</span>}
              </div>
              {expandedAgent === agent.id && (
                <div style={{ marginTop: 8 }}>
                  <div style={{ fontSize: 11, color: "var(--vscode-disabledForeground, #888)", marginBottom: 4 }}>Context Files:</div>
                  {agent.contextFiles.map((f) => (
                    <div key={f} style={{ fontSize: 11, fontFamily: "monospace", padding: "2px 6px", marginBottom: 2, background: "var(--vscode-editor-background, #1e1e1e)", borderRadius: 3 }}>{f}</div>
                  ))}
                  {/* Progress bar */}
                  <div style={{ marginTop: 8 }}>
                    <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, marginBottom: 3 }}>
                      <span>Progress</span>
                      <span>{Math.round((agent.turnCount / agent.maxTurns) * 100)}%</span>
                    </div>
                    <div style={{ background: "var(--vscode-editor-background, #1e1e1e)", borderRadius: 3, height: 6, overflow: "hidden" }}>
                      <div style={{ width: `${(agent.turnCount / agent.maxTurns) * 100}%`, height: "100%", background: statusColor(agent.status), borderRadius: 3 }} />
                    </div>
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Results Tab */}
      {tab === "Results" && (
        <div>
          {MOCK_RESULTS.map((result) => (
            <div key={result.agentId} style={{ padding: "10px 12px", marginBottom: 8, borderRadius: 4, background: "var(--vscode-editor-inactiveSelectionBackground, #2d2d2d)", borderLeft: `3px solid ${result.success ? "var(--vscode-charts-blue, #007acc)" : "var(--vscode-errorForeground, #f44336)"}` }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                <span style={{ fontWeight: 600, fontSize: 12 }}>{result.role}</span>
                <span style={{ fontSize: 10, padding: "2px 8px", borderRadius: 10, background: result.success ? "var(--vscode-charts-green, #4caf50)" : "var(--vscode-errorForeground, #f44336)", color: "#fff", fontWeight: 600 }}>{result.success ? "Success" : "Failed"}</span>
                <span style={{ marginLeft: "auto", fontSize: 10, color: "var(--vscode-disabledForeground, #888)" }}>{result.completedAt}</span>
              </div>
              <div style={{ fontSize: 12, marginBottom: 6, lineHeight: 1.5 }}>{result.summary}</div>
              <div style={{ fontSize: 11, color: "var(--vscode-disabledForeground, #888)", marginBottom: 4 }}>Findings:</div>
              <ul style={{ margin: 0, paddingLeft: 16, fontSize: 11, lineHeight: 1.6 }}>
                {result.findings.map((f, i) => (
                  <li key={i}>{f}</li>
                ))}
              </ul>
              {result.filesModified.length > 0 && (
                <div style={{ marginTop: 6 }}>
                  <span style={{ fontSize: 11, color: "var(--vscode-disabledForeground, #888)" }}>Modified: </span>
                  {result.filesModified.map((f) => (
                    <span key={f} style={{ fontSize: 10, fontFamily: "monospace", padding: "1px 5px", borderRadius: 3, background: "var(--vscode-editor-background, #1e1e1e)", marginLeft: 4 }}>{f}</span>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Config Tab */}
      {tab === "Config" && (
        <div>
          {configs.map((cfg) => (
            <div key={cfg.role} style={{ padding: "10px 12px", marginBottom: 8, borderRadius: 4, background: "var(--vscode-editor-inactiveSelectionBackground, #2d2d2d)", opacity: cfg.enabled ? 1 : 0.5 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 6 }}>
                <input type="checkbox" checked={cfg.enabled} onChange={() => toggleConfig(cfg.role)} style={{ cursor: "pointer" }} />
                <span style={{ fontWeight: 600, fontSize: 12 }}>{cfg.role}</span>
                <span style={{ fontSize: 10, color: "var(--vscode-disabledForeground, #888)" }}>max {cfg.maxTurns} turns</span>
              </div>
              <div style={{ fontSize: 11, color: "var(--vscode-disabledForeground, #888)", marginBottom: 6 }}>{cfg.description}</div>
              <div style={{ marginBottom: 6 }}>
                <span style={{ fontSize: 10, color: "var(--vscode-disabledForeground, #888)" }}>Tools: </span>
                <span style={{ display: "inline-flex", gap: 4, flexWrap: "wrap" }}>
                  {cfg.tools.map((tool) => (
                    <span key={tool} style={{ fontSize: 10, padding: "1px 5px", borderRadius: 3, background: "var(--vscode-badge-background, #444)", color: "var(--vscode-badge-foreground, #fff)" }}>{tool}</span>
                  ))}
                </span>
              </div>
              {cfg.autoSpawnTriggers.length > 0 && (
                <div>
                  <span style={{ fontSize: 10, color: "var(--vscode-disabledForeground, #888)" }}>Auto-spawn: </span>
                  <span style={{ display: "inline-flex", gap: 4, flexWrap: "wrap" }}>
                    {cfg.autoSpawnTriggers.map((trigger) => (
                      <span key={trigger} style={{ fontSize: 10, padding: "1px 5px", borderRadius: 3, background: "var(--vscode-charts-green, #4caf50)", color: "#fff" }}>{trigger}</span>
                    ))}
                  </span>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default SubAgentPanel;
