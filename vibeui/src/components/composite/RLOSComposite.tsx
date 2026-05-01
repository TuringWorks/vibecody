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
    // Slices 1-7 shipped: every panel reads from the real backend.
    // Compute extensions still live behind opt-in sidecar extras for
    // the slowest paths:
    //   - 6.5 — deployment /act (ONNX runtime or Python inference)
    //   - 7a-sidecar — distill / quantize / prune algorithms
    //   - 7b-sidecar — MAPPO / QMIX / VDN / MADDPG (PettingZoo dep)
    //   - 7c-sidecar — TRL preference loop (HuggingFace transformers)
    // Empty `covers` collapses the badge to nothing — every panel is
    // wired to real data, even if some require additional `uv sync
    // --extra X` to populate.
    banner: (
      <SimulationModeBadge
        description="Compute extensions for the heaviest workloads (real inference / RLHF / MARL / distillation algorithms) ship behind opt-in sidecar extras. The dashboard data below is real."
        covers={[]}
      />
    ),
  }
);
