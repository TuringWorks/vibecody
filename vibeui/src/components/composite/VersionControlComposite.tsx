import { createComposite } from "./createComposite";

export const VersionControlComposite = createComposite([
  { id: "history", label: "History", importFn: () => import("../HistoryPanel"), exportName: "HistoryPanel" },
  { id: "checkpoints", label: "Checkpoints", importFn: () => import("../CheckpointPanel"), exportName: "CheckpointPanel" },
  { id: "bisect", label: "Bisect", importFn: () => import("../BisectPanel"), exportName: "BisectPanel" },
]);
