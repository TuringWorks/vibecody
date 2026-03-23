import { createComposite } from "./createComposite";

export const ObservabilityComposite = createComposite([
  { id: "traces", label: "Traces", importFn: () => import("../TraceDashboard"), exportName: "TraceDashboard" },
  { id: "recording", label: "Recording", importFn: () => import("../AgentRecordingPanel"), exportName: "AgentRecordingPanel" },
  { id: "demo", label: "Demo", importFn: () => import("../DemoPanel"), exportName: "DemoPanel" },
]);
