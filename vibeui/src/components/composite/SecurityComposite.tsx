import { createComposite } from "./createComposite";

export const SecurityComposite = createComposite([
  { id: "redteam", label: "Red Team", importFn: () => import("../RedTeamPanel"), exportName: "RedTeamPanel" },
  { id: "blueteam", label: "Blue Team", importFn: () => import("../BlueTeamPanel"), exportName: "BlueTeamPanel" },
  { id: "purpleteam", label: "Purple Team", importFn: () => import("../PurpleTeamPanel"), exportName: "PurpleTeamPanel" },
  { id: "securityscan", label: "Scanner", importFn: () => import("../SecurityScanPanel") },
]);
