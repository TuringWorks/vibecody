import { createComposite } from "./createComposite";

export const ToolsSettingsComposite = createComposite([
  { id: "artifacts", label: "Artifacts", importFn: () => import("../ArtifactsPanel"), exportName: "ArtifactsPanel" },
  { id: "img2app", label: "Img2App", importFn: () => import("../ScreenshotToApp"), exportName: "ScreenshotToApp" },
  { id: "visualtest", label: "Visual Test", importFn: () => import("../VisualTestPanel"), exportName: "VisualTestPanel" },
]);
