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
    // Slices 1-4 shipped: persistence, run lifecycle, real Python-sidecar
    // training (PPO on Gymnasium), env registry, and eval suites with
    // bootstrap-CI metric storage + paired comparison. Eval rollout
    // execution still requires the sidecar (slice 4.5 wires `eval`
    // subcommand fully); slice 4 ships the storage + comparison surface.
    banner: (
      <SimulationModeBadge
        description="RL-OS productionization is in progress. Training, Environments, Evaluation, and Compare are real (vibe-rl-py sidecar); the panels listed below still render illustrative data while their slices land."
        covers={[
          "Optimization",
          "Deployment",
          "Lineage",
          "Multi-Agent",
          "Rewards",
          "RLHF",
        ]}
      />
    ),
  }
);
