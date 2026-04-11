import { createComposite } from "./createComposite";

export const AiMlComposite = createComposite([
  { id: "workflow", label: "Workflow", importFn: () => import("../AiMlWorkflowPanel"), exportName: "AiMlWorkflowPanel" },
  { id: "wizard", label: "Wizard", importFn: () => import("../ModelWizardPanel"), exportName: "ModelWizardPanel" },
  { id: "training", label: "Training", importFn: () => import("../TrainingPanel"), exportName: "TrainingPanel" },
  { id: "inference", label: "Inference", importFn: () => import("../InferencePanel"), exportName: "InferencePanel" },
  { id: "quantum", label: "Quantum", importFn: () => import("../QuantumComputingPanel"), exportName: "QuantumComputingPanel" },
  { id: "rlcef", label: "RLCEF", importFn: () => import("../RlcefPanel"), exportName: "RlcefPanel" },
  { id: "skilldistill", label: "Distillation", importFn: () => import("../SkillDistillationPanel") },
  { id: "selfimproving", label: "Self-Improving", importFn: () => import("../SelfImprovingSkillsPanel"), exportName: "SelfImprovingSkillsPanel" },
]);
