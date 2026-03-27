import { createComposite } from "./createComposite";

export const AiPlaygroundComposite = createComposite([
  { id: "counsel", label: "Counsel", importFn: () => import("../CounselPanel"), exportName: "CounselPanel" },
  { id: "superbrain", label: "SuperBrain", importFn: () => import("../SuperBrainPanel"), exportName: "SuperBrainPanel" },
  { id: "compare", label: "Compare", importFn: () => import("../MultiModelPanel"), exportName: "MultiModelPanel" },
  { id: "arena", label: "Arena", importFn: () => import("../ArenaPanel"), exportName: "ArenaPanel" },
  { id: "mctsrepair", label: "MCTS Repair", importFn: () => import("../MctsRepairPanel"), exportName: "MctsRepairPanel" },
  { id: "costrouter", label: "Cost Router", importFn: () => import("../CostRouterPanel"), exportName: "CostRouterPanel" },
  { id: "rlcef", label: "RLCEF", importFn: () => import("../RlcefPanel"), exportName: "RlcefPanel" },
  { id: "langgraph", label: "LangGraph", importFn: () => import("../LangGraphPanel"), exportName: "LangGraphPanel" },
]);
