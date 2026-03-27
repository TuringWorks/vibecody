import { createComposite } from "./createComposite";

export const AiTeamsComposite = createComposite([
  { id: "teams", label: "Teams", importFn: () => import("../AgentTeamPanel"), exportName: "AgentTeamPanel" },
  { id: "agentteams", label: "Hierarchy", importFn: () => import("../AgentTeamsPanel") },
  { id: "subagents", label: "Sub-Agents", importFn: () => import("../SubAgentPanel") },
  { id: "cloud", label: "Cloud", importFn: () => import("../CloudAgentPanel"), exportName: "CloudAgentPanel" },
  { id: "cibot", label: "CI Bot", importFn: () => import("../CIReviewPanel"), exportName: "CIReviewPanel" },
  { id: "agentmodes", label: "Modes", importFn: () => import("../AgentModesPanel") },
  { id: "spawnagent", label: "Spawn", importFn: () => import("../SpawnAgentPanel") },
  { id: "a2a", label: "A2A", importFn: () => import("../A2aPanel"), exportName: "A2aPanel" },
  { id: "agenthost", label: "Host", importFn: () => import("../AgentHostPanel"), exportName: "AgentHostPanel" },
  { id: "worktreepool", label: "Worktrees", importFn: () => import("../WorktreePoolPanel"), exportName: "WorktreePoolPanel" },
  { id: "proactive", label: "Proactive", importFn: () => import("../ProactivePanel"), exportName: "ProactivePanel" },
]);
