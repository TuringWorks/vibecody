import { createComposite } from "./createComposite";

export const GitHubComposite = createComposite([
  { id: "github", label: "Sync", importFn: () => import("../GitHubSyncPanel"), exportName: "GitHubSyncPanel" },
  { id: "ghactions", label: "Actions", importFn: () => import("../GhActionsPanel") },
]);
