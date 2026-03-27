import { createComposite } from "./createComposite";

export const DesignComposite = createComposite([
  { id: "design", label: "Design", importFn: () => import("../DesignMode"), exportName: "DesignMode" },
  { id: "sketch", label: "Sketch", importFn: () => import("../SketchCanvasPanel"), exportName: "SketchCanvasPanel" },
  { id: "img2app", label: "Img2App", importFn: () => import("../ScreenshotToApp"), exportName: "ScreenshotToApp" },
]);
