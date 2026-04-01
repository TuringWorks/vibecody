import { createComposite } from "./createComposite";

export const AgentOSComposite = createComposite([
  { id: "dashboard", label: "Dashboard", importFn: () => import("../AgentOSDashboard"), exportName: "AgentOSDashboard" },
  { id: "agent", label: "Agent", importFn: () => import("../AgentPanel"), exportName: "AgentPanel" },
  { id: "teams", label: "Teams", importFn: () => import("../AgentTeamsPanel") },
  { id: "spawn", label: "Spawn", importFn: () => import("../SpawnAgentPanel") },
  { id: "subagents", label: "Sub-Agents", importFn: () => import("../SubAgentPanel") },
  { id: "modes", label: "Modes", importFn: () => import("../AgentModesPanel") },
  { id: "host", label: "Host", importFn: () => import("../AgentHostPanel"), exportName: "AgentHostPanel" },
  { id: "branch", label: "Branch", importFn: () => import("../BranchAgentPanel"), exportName: "BranchAgentPanel" },
  { id: "browser", label: "Browser", importFn: () => import("../BrowserAgentPanel"), exportName: "BrowserAgentPanel" },
  { id: "cloud", label: "Cloud", importFn: () => import("../CloudAgentPanel"), exportName: "CloudAgentPanel" },
  { id: "orchestration", label: "Orchestration", importFn: () => import("../OrchestrationPanel"), exportName: "OrchestrationPanel" },
]);
