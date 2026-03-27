import { createComposite } from "./createComposite";

export const AiContextComposite = createComposite([
  { id: "icontext", label: "Infinite Context", importFn: () => import("../InfiniteContextPanel"), exportName: "InfiniteContextPanel" },
  { id: "bundles", label: "Bundles", importFn: () => import("../ContextBundlePanel"), exportName: "ContextBundlePanel" },
  { id: "openmemory", label: "Open Memory", importFn: () => import("../OpenMemoryPanel") },
  { id: "fastcontext", label: "Fast Context", importFn: () => import("../FastContextPanel") },
  { id: "semanticindex", label: "Semantic", importFn: () => import("../SemanticIndexPanel"), exportName: "SemanticIndexPanel" },
  { id: "webgrounding", label: "Web Search", importFn: () => import("../WebGroundingPanel"), exportName: "WebGroundingPanel" },
  { id: "sessionmemory", label: "Session Memory", importFn: () => import("../SessionMemoryPanel"), exportName: "SessionMemoryPanel" },
  { id: "artifacts", label: "Artifacts", importFn: () => import("../ArtifactsPanel"), exportName: "ArtifactsPanel" },
]);
