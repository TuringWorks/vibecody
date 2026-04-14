import { createComposite } from "./createComposite";

export const ArchitectureComposite = createComposite([
  { id: "archspec", label: "Architecture", importFn: () => import("../ArchitectureSpecPanel") },
  { id: "policy", label: "Policy Engine", importFn: () => import("../PolicyEnginePanel") },
]);
