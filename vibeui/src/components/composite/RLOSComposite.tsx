import { createComposite } from "./createComposite";
import { SimulationModeBadge } from "../SimulationModeBadge";

export const RLOSComposite = createComposite(
  [
    { id: "training", label: "Training", importFn: () => import("../RLTrainingDashboard"), exportName: "RLTrainingDashboard" },
    { id: "environments", label: "Environments", importFn: () => import("../RLEnvironmentViewer"), exportName: "RLEnvironmentViewer" },
    { id: "eval", label: "Evaluation", importFn: () => import("../RLEvalResults"), exportName: "RLEvalResults" },
    { id: "optimization", label: "Optimization", importFn: () => import("../RLOptimizationReport"), exportName: "RLOptimizationReport" },
    { id: "deployment", label: "Deployment", importFn: () => import("../RLDeploymentMonitor"), exportName: "RLDeploymentMonitor" },
    { id: "comparison", label: "Compare", importFn: () => import("../RLPolicyComparison"), exportName: "RLPolicyComparison" },
    { id: "lineage", label: "Lineage", importFn: () => import("../RLModelLineage"), exportName: "RLModelLineage" },
    { id: "multiagent", label: "Multi-Agent", importFn: () => import("../RLMultiAgentView"), exportName: "RLMultiAgentView" },
    { id: "rewards", label: "Rewards", importFn: () => import("../RLRewardDecomposition"), exportName: "RLRewardDecomposition" },
    { id: "rlhf", label: "RLHF", importFn: () => import("../RLHFAlignmentDashboard"), exportName: "RLHFAlignmentDashboard" },
  ],
  {
    banner: (
      <SimulationModeBadge description="RL-OS panels show generated data while real training, evaluation, and deployment backends are still in progress. Numbers on these tabs are illustrative and do not reflect production runs." />
    ),
  }
);
