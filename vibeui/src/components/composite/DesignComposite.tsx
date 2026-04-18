import { createComposite } from "./createComposite";

/**
 * DesignComposite — the rolled-up "Design" surface in VibeUI.
 *
 * Tabs are intentionally ordered so the unified hub is first; legacy single-
 * purpose panels follow. DesignMode owns the multi-tab "design mode" tools
 * (excluding Figma — Figma now lives in DesignHubPanel under "Figma").
 */
export const DesignComposite = createComposite([
  { id: "hub", label: "Hub", importFn: () => import("../DesignHubPanel"), exportName: "DesignHubPanel" },
  { id: "import", label: "Import", importFn: () => import("../DesignImportPanel") },
  { id: "design", label: "Design", importFn: () => import("../DesignMode"), exportName: "DesignMode" },
  { id: "annotations", label: "Annotations", importFn: () => import("../DesignAnnotationsPanel"), exportName: "DesignAnnotationsPanel" },
  { id: "sketch", label: "Sketch", importFn: () => import("../SketchCanvasPanel"), exportName: "SketchCanvasPanel" },
  { id: "img2app", label: "Screenshot to App", importFn: () => import("../ScreenshotToApp"), exportName: "ScreenshotToApp" },
]);
