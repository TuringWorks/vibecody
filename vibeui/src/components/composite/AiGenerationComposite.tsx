import { createComposite } from "./createComposite";

export const AiGenerationComposite = createComposite([
  { id: "batchbuilder", label: "Batch Builder", importFn: () => import("../BatchBuilderPanel") },
  { id: "imagegen", label: "Image Gen", importFn: () => import("../ImageGenPanel") },
  { id: "autoresearch", label: "Research", importFn: () => import("../AutoResearchPanel"), exportName: "AutoResearchPanel" },
]);
