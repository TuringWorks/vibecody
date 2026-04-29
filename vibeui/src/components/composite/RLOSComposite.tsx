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
    // Slice 1 has shipped persistence + run lifecycle for the Training panel.
    // The Training panel itself is no longer fully illustrative — runs are
    // durable, but metrics are still empty because the executor (slice 2)
    // hasn't shipped. Each slice in `docs/design/rl-os/` removes panels
    // from this `covers` list as it productionizes them.
    banner: (
      <SimulationModeBadge
        description="RL-OS productionization is in progress. Panels listed below still render illustrative data while their respective slices land. Runs you create are durable; metrics will populate once the executor (slice 2) ships."
        covers={[
          "Training (metrics — durable run list, but no executor yet)",
          "Environments",
          "Evaluation",
          "Optimization",
          "Deployment",
          "Compare",
          "Lineage",
          "Multi-Agent",
          "Rewards",
          "RLHF",
        ]}
      />
    ),
  }
);
