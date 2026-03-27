import { createComposite } from "./createComposite";

export const AiTeamsComposite = createComposite([
  { id: "teams", label: "Teams", importFn: () => import("../AgentTeamPanel"), exportName: "AgentTeamPanel" },
  { id: "agentteams", label: "Hierarchy", importFn: () => import("../AgentTeamsPanel") },
  { id: "subagents", label: "Sub-Agents", importFn: () => import("../SubAgentPanel") },
  { id: "cloud", label: "Cloud", importFn: () => import("../CloudAgentPanel"), exportName: "CloudAgentPanel" },
  { id: "cibot", label: "CI Bot", importFn: () => import("../CIReviewPanel"), exportName: "CIReviewPanel" },
  { id: "agentmodes", label: "Modes", importFn: () => import("../AgentModesPanel") },
  { id: "spawnagent", label: "Spawn", importFn: () => import("../SpawnAgentPanel") },
]);
