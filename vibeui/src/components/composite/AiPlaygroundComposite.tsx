import { createComposite } from "./createComposite";

export const AiPlaygroundComposite = createComposite([
  { id: "counsel", label: "Counsel", importFn: () => import("../CounselPanel"), exportName: "CounselPanel" },
  { id: "superbrain", label: "SuperBrain", importFn: () => import("../SuperBrainPanel"), exportName: "SuperBrainPanel" },
  { id: "compare", label: "Compare", importFn: () => import("../MultiModelPanel"), exportName: "MultiModelPanel" },
  { id: "arena", label: "Arena", importFn: () => import("../ArenaPanel"), exportName: "ArenaPanel" },
]);
