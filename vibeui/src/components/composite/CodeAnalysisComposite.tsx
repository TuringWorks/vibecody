import { createComposite } from "./createComposite";

export const CodeAnalysisComposite = createComposite([
  { id: "metrics", label: "Metrics", importFn: () => import("../CodeMetricsPanel"), exportName: "CodeMetricsPanel" },
  { id: "astedit", label: "AST Edit", importFn: () => import("../AstEditPanel") },
  { id: "editpredict", label: "Predict", importFn: () => import("../EditPredictionPanel") },
  { id: "snippets", label: "Snippets", importFn: () => import("../SnippetPanel"), exportName: "SnippetPanel" },
  { id: "healthscore", label: "Health Score", importFn: () => import("../HealthScorePanel") },
  { id: "intentrefactor", label: "Refactor", importFn: () => import("../IntentRefactorPanel") },
  { id: "reviewprotocol", label: "Review", importFn: () => import("../ReviewProtocolPanel") },
  { id: "skilldistill", label: "Distillation", importFn: () => import("../SkillDistillationPanel") },
]);
