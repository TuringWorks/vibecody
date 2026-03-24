import { createComposite } from "./createComposite";

export const AiPlaygroundComposite = createComposite([
  { id: "compare", label: "Compare", importFn: () => import("../MultiModelPanel"), exportName: "MultiModelPanel" },
  { id: "arena", label: "Arena", importFn: () => import("../ArenaPanel"), exportName: "ArenaPanel" },
  { id: "cascade", label: "Cascade", importFn: () => import("../CascadePanel"), exportName: "CascadePanel" },
]);
