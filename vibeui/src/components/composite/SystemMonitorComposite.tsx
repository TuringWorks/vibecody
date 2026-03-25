import { createComposite } from "./createComposite";

export const SystemMonitorComposite = createComposite([
  { id: "processes", label: "Processes", importFn: () => import("../ProcessPanel") },
  { id: "profiler", label: "Profiler", importFn: () => import("../ProfilerPanel"), exportName: "ProfilerPanel" },
  { id: "debug", label: "Debug", importFn: () => import("../DebugModePanel") },
  { id: "browser", label: "Browser Agent", importFn: () => import("../BrowserAgentPanel"), exportName: "BrowserAgentPanel" },
  { id: "observeact", label: "Observe-Act", importFn: () => import("../ObserveActPanel"), exportName: "ObserveActPanel" },
  { id: "desktop", label: "Desktop", importFn: () => import("../DesktopAgentPanel"), exportName: "DesktopAgentPanel" },
]);
