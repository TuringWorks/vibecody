import { createComposite } from "./createComposite";

// FIT-GAP v8: cross-env dispatch, nested agents, thought streaming,
// hard-problem decomposition, reproducible agents.
export const AgentIntelligenceComposite = createComposite([
  { id: "env-dispatch", label: "Env Dispatch", importFn: () => import("../EnvDispatchPanel"), exportName: "EnvDispatchPanel" },
  { id: "nested-agents", label: "Nested Agents", importFn: () => import("../NestedAgentsPanel"), exportName: "NestedAgentsPanel" },
  { id: "thought-stream", label: "Thought Stream", importFn: () => import("../ThoughtStreamPanel"), exportName: "ThoughtStreamPanel" },
  { id: "hard-problem", label: "Hard Problem", importFn: () => import("../HardProblemPanel"), exportName: "HardProblemPanel" },
  { id: "repro-agent", label: "Repro Agent", importFn: () => import("../ReproAgentPanel"), exportName: "ReproAgentPanel" },
]);
