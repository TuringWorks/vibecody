import { createComposite } from "./createComposite";

export const SystemMonitorComposite = createComposite([
  { id: "processes", label: "Processes", importFn: () => import("../ProcessPanel") },
  { id: "profiler", label: "Profiler", importFn: () => import("../ProfilerPanel"), exportName: "ProfilerPanel" },
  { id: "debug", label: "Debug", importFn: () => import("../DebugModePanel") },
]);
