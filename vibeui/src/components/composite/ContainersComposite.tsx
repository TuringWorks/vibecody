import { createComposite } from "./createComposite";

export const ContainersComposite = createComposite([
  { id: "docker", label: "Docker", importFn: () => import("../DockerPanel"), exportName: "DockerPanel" },
  { id: "k8s", label: "K8s", importFn: () => import("../K8sPanel") },
  { id: "sandbox", label: "Sandbox", importFn: () => import("../SandboxPanel"), exportName: "SandboxPanel" },
  { id: "cloudsandbox", label: "Cloud Sandbox", importFn: () => import("../CloudSandboxPanel") },
]);
