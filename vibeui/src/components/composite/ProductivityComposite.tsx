import { createComposite } from "./createComposite";

export const ProductivityComposite = createComposite([
  { id: "productivity", label: "Productivity", importFn: () => import("../ProductivityPanel"), exportName: "ProductivityPanel" },
]);
