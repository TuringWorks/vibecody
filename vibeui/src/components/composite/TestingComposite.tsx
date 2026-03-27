import { createComposite } from "./createComposite";

export const TestingComposite = createComposite([
  { id: "tests", label: "Tests", importFn: () => import("../TestPanel"), exportName: "TestPanel" },
  { id: "coverage", label: "Coverage", importFn: () => import("../CoveragePanel"), exportName: "CoveragePanel" },
  { id: "bugbot", label: "BugBot", importFn: () => import("../BugBotPanel"), exportName: "BugBotPanel" },
  { id: "autofix", label: "Autofix", importFn: () => import("../AutofixPanel"), exportName: "AutofixPanel" },
  { id: "cloudautofix", label: "Cloud Fix", importFn: () => import("../CloudAutofixPanel") },
  { id: "qa-validation", label: "QA", importFn: () => import("../QaValidationPanel"), exportName: "QaValidationPanel" },
  { id: "visualverify", label: "Visual Verify", importFn: () => import("../VisualVerifyPanel"), exportName: "VisualVerifyPanel" },
  { id: "mctsrepair", label: "MCTS Repair", importFn: () => import("../MctsRepairPanel"), exportName: "MctsRepairPanel" },
  { id: "swebench", label: "SWE-Bench", importFn: () => import("../SweBenchPanel"), exportName: "SweBenchPanel" },
  { id: "visualtest", label: "Visual Test", importFn: () => import("../VisualTestPanel") },
]);
