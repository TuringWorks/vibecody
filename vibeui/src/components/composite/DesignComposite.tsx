import { createComposite } from "./createComposite";

export const DesignComposite = createComposite([
  { id: "design", label: "Design", importFn: () => import("../DesignMode"), exportName: "DesignMode" },
  { id: "sketch", label: "Sketch", importFn: () => import("../SketchCanvasPanel"), exportName: "SketchCanvasPanel" },
  { id: "img2app", label: "Screenshot to App", importFn: () => import("../ScreenshotToApp"), exportName: "ScreenshotToApp" },
  { id: "design-mode", label: "Design Mode", importFn: () => import("../DesignModePanel"), exportName: "DesignModePanel" },
]);
