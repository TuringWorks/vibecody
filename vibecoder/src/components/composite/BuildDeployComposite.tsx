import { createComposite } from "./createComposite";

export const BuildDeployComposite = createComposite([
  { id: "build", label: "Build", importFn: () => import("../BuildPanel"), exportName: "BuildPanel" },
  { id: "deploy", label: "Deploy", importFn: () => import("../DeployPanel"), exportName: "DeployPanel" },
  { id: "scaffold", label: "Scaffold", importFn: () => import("../ScaffoldPanel"), exportName: "ScaffoldPanel" },
  { id: "appbuilder", label: "App Builder", importFn: () => import("../AppBuilderPanel"), exportName: "AppBuilderPanel" },
  { id: "fullstack", label: "Full Stack", importFn: () => import("../FullStackGenPanel") },
  { id: "worktreepool", label: "Worktrees", importFn: () => import("../WorktreePoolPanel"), exportName: "WorktreePoolPanel" },
  { id: "auto-deploy", label: "Auto Deploy", importFn: () => import("../AutoDeployPanel"), exportName: "AutoDeployPanel" },
]);
