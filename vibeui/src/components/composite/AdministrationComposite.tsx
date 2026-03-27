import { createComposite } from "./createComposite";

export const AdministrationComposite = createComposite([
  { id: "admin", label: "Admin", importFn: () => import("../AdminPanel"), exportName: "AdminPanel" },
  { id: "auth", label: "Auth", importFn: () => import("../AuthPanel"), exportName: "AuthPanel" },
  { id: "governance", label: "Governance", importFn: () => import("../TeamGovernancePanel") },
  { id: "sessions", label: "Sessions", importFn: () => import("../SessionBrowserPanel") },
  { id: "manager", label: "Manager", importFn: () => import("../ManagerView"), exportName: "ManagerView" },
  { id: "analytics", label: "Analytics", importFn: () => import("../AnalyticsPanel"), exportName: "AnalyticsPanel" },
  { id: "trust", label: "Trust", importFn: () => import("../TrustPanel"), exportName: "TrustPanel" },
]);
