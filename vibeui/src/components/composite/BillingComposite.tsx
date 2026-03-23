import { createComposite } from "./createComposite";

export const BillingComposite = createComposite([
  { id: "cost", label: "Cost", importFn: () => import("../CostPanel"), exportName: "CostPanel" },
  { id: "usagemetering", label: "Usage", importFn: () => import("../UsageMeteringPanel"), exportName: "UsageMeteringPanel" },
]);
