import { createComposite } from "./createComposite";

export const PlanningComposite = createComposite([
  { id: "specs", label: "Specs", importFn: () => import("../SpecPanel"), exportName: "SpecPanel" },
  { id: "plandoc", label: "Plan Doc", importFn: () => import("../PlanDocumentPanel") },
  { id: "workflow", label: "Workflow", importFn: () => import("../WorkflowPanel"), exportName: "WorkflowPanel" },
  { id: "orchestration", label: "Orchestration", importFn: () => import("../OrchestrationPanel"), exportName: "OrchestrationPanel" },
  { id: "clarify", label: "Clarify", importFn: () => import("../ClarifyingQuestionsPanel") },
  { id: "codesearch", label: "Search", importFn: () => import("../ConversationalSearchPanel") },
  { id: "nexttask", label: "Next Task", importFn: () => import("../NextTaskPanel"), exportName: "NextTaskPanel" },
  { id: "docsync", label: "Doc Sync", importFn: () => import("../DocSyncPanel"), exportName: "DocSyncPanel" },
]);
