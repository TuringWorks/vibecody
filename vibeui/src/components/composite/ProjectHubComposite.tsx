import { createComposite } from "./createComposite";

export const ProjectHubComposite = createComposite([
  { id: "workmgmt", label: "Work Mgmt", importFn: () => import("../WorkManagementPanel") },
  { id: "dashboard", label: "Dashboard", importFn: () => import("../DashboardPanel") },
  { id: "steering", label: "Steering", importFn: () => import("../SteeringPanel") },
  { id: "soul", label: "Soul", importFn: () => import("../SoulPanel"), exportName: "SoulPanel" },
  { id: "rules", label: "Rules", importFn: () => import("../MemoryPanel"), exportName: "MemoryPanel" },
]);
