import { createComposite } from "./createComposite";
import { ExperimentalBadge } from "../ExperimentalBadge";

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
    // The ExperimentalBadge replaces the previous SimulationModeBadge:
    // panels are no longer illustrative, but the workstream is still
    // experimental — semver may break, sidecar extras may shift, and
    // real production training pipelines shouldn't depend on this
    // surface yet. See `docs/design/feature-flags/README.md` for the
    // GA promotion criteria (`composite.rl_os` flag).
    banner: (
      <ExperimentalBadge
        as="banner"
        feature="RL-OS dashboard"
        tooltip="Backend is real but the workstream is still maturing — heavy compute (RLHF / MARL / distillation) needs `uv sync --extra X` and the API may shift before GA."
      />
    ),
  }
);
