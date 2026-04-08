import { createComposite } from "./createComposite";

export const ToolsSettingsComposite = createComposite([
  { id: "automations", label: "Automations", importFn: () => import("../AutomationsPanel"), exportName: "AutomationsPanel" },
  { id: "selfreview", label: "Self-Review", importFn: () => import("../SelfReviewPanel"), exportName: "SelfReviewPanel" },
]);
