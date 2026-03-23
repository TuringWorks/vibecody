import { createComposite } from "./createComposite";

export const CollaborationComposite = createComposite([
  { id: "collab", label: "Collab", importFn: () => import("../CollabPanel"), exportName: "CollabPanel" },
  { id: "compliance", label: "Compliance", importFn: () => import("../CompliancePanel"), exportName: "CompliancePanel" },
]);
