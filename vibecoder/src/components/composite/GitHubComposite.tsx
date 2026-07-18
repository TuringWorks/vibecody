import { createComposite } from "./createComposite";

export const GitHubComposite = createComposite([
  { id: "github", label: "Sync", importFn: () => import("../GitHubSyncPanel"), exportName: "GitHubSyncPanel" },
  { id: "ghactions", label: "Actions", importFn: () => import("../GhActionsPanel") },
  { id: "triage", label: "Triage", importFn: () => import("../TriagePanel"), exportName: "TriagePanel" },
]);
