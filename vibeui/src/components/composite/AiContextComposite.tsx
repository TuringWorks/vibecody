import { createComposite } from "./createComposite";

export const AiContextComposite = createComposite([
  { id: "icontext", label: "Infinite Context", importFn: () => import("../InfiniteContextPanel"), exportName: "InfiniteContextPanel" },
  { id: "bundles", label: "Bundles", importFn: () => import("../ContextBundlePanel"), exportName: "ContextBundlePanel" },
  { id: "openmemory", label: "Open Memory", importFn: () => import("../OpenMemoryPanel") },
  { id: "fastcontext", label: "Fast Context", importFn: () => import("../FastContextPanel") },
]);
