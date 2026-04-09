import { createComposite } from "./createComposite";

export const CiCdComposite = createComposite([
  { id: "pipeline", label: "Pipeline", importFn: () => import("../CicdPanel") },
  { id: "status",   label: "Status",   importFn: () => import("../CiStatusPanel") },
  { id: "gates",    label: "Gates",    importFn: () => import("../CiGatesPanel") },
]);
