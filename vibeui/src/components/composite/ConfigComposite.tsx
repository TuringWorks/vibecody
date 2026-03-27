import { createComposite } from "./createComposite";

export const ConfigComposite = createComposite([
  { id: "settings", label: "Keys", importFn: () => import("../SettingsPanel"), exportName: "SettingsPanel" },
  { id: "hooks", label: "Hooks", importFn: () => import("../HooksPanel"), exportName: "HooksPanel" },
  { id: "markers", label: "Bookmarks", importFn: () => import("../BookmarkPanel"), exportName: "BookmarkPanel" },
  { id: "jobs", label: "Jobs", importFn: () => import("../BackgroundJobsPanel"), exportName: "BackgroundJobsPanel" },
]);
