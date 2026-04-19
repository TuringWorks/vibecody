import { createComposite } from "./createComposite";

export const ConfigComposite = createComposite([
  { id: "keys", label: "Keys", importFn: () => import("../KeysPanel"), exportName: "KeysPanel" },
  { id: "models", label: "Models", importFn: () => import("../ModelManagerPanel"), exportName: "ModelManagerPanel" },
  { id: "settings", label: "Settings", importFn: () => import("../SettingsPanel"), exportName: "SettingsPanel" },
  { id: "security", label: "Security", importFn: () => import("../SecurityPanel"), exportName: "SecurityPanel" },
  { id: "hooks", label: "Hooks", importFn: () => import("../HooksPanel"), exportName: "HooksPanel" },
  { id: "markers", label: "Bookmarks", importFn: () => import("../BookmarkPanel"), exportName: "BookmarkPanel" },
  { id: "jobs", label: "Jobs", importFn: () => import("../BackgroundJobsPanel"), exportName: "BackgroundJobsPanel" },
]);
