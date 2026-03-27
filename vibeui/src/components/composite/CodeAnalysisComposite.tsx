import { createComposite } from "./createComposite";

export const CodeAnalysisComposite = createComposite([
  { id: "transform", label: "Transform", importFn: () => import("../TransformPanel"), exportName: "TransformPanel" },
  { id: "metrics", label: "Metrics", importFn: () => import("../CodeMetricsPanel"), exportName: "CodeMetricsPanel" },
  { id: "astedit", label: "AST Edit", importFn: () => import("../AstEditPanel") },
  { id: "editpredict", label: "Predict", importFn: () => import("../EditPredictionPanel") },
  { id: "snippets", label: "Snippets", importFn: () => import("../SnippetPanel"), exportName: "SnippetPanel" },
  { id: "triage", label: "Triage", importFn: () => import("../TriagePanel"), exportName: "TriagePanel" },
  { id: "docsync", label: "Doc Sync", importFn: () => import("../DocSyncPanel"), exportName: "DocSyncPanel" },
]);
