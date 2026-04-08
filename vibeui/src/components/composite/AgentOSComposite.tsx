import { createComposite } from "./createComposite";

// Single-agent control: dashboard, runtime, execution modes, host, branch, browser, task orchestration.
// Multi-agent coordination (Teams, Spawn, Sub-Agents, Cloud) lives in AI Teams.
export const AgentOSComposite = createComposite([
  { id: "dashboard", label: "Dashboard", importFn: () => import("../AgentOSDashboard"), exportName: "AgentOSDashboard" },
  { id: "agent", label: "Agent", importFn: () => import("../AgentPanel"), exportName: "AgentPanel" },
  { id: "modes", label: "Modes", importFn: () => import("../AgentModesPanel") },
  { id: "host", label: "Host", importFn: () => import("../AgentHostPanel"), exportName: "AgentHostPanel" },
  { id: "branch", label: "Branch", importFn: () => import("../BranchAgentPanel"), exportName: "BranchAgentPanel" },
  { id: "browser", label: "Browser", importFn: () => import("../BrowserAgentPanel"), exportName: "BrowserAgentPanel" },
  { id: "orchestration", label: "Orchestration", importFn: () => import("../OrchestrationPanel"), exportName: "OrchestrationPanel" },
]);
