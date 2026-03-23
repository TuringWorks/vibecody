import { createComposite } from "./createComposite";

export const EditorsComposite = createComposite([
  { id: "difftool", label: "Diff", importFn: () => import("../DiffToolPanel"), exportName: "DiffToolPanel" },
  { id: "markdown", label: "Markdown", importFn: () => import("../MarkdownPanel"), exportName: "MarkdownPanel" },
  { id: "canvas", label: "Canvas", importFn: () => import("../CanvasPanel") },
  { id: "colors", label: "Palette", importFn: () => import("../ColorPalettePanel"), exportName: "ColorPalettePanel" },
]);
