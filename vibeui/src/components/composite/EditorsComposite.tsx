import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const DiffToolPanel = lazy(() => import("../DiffToolPanel").then(m => ({ default: m.DiffToolPanel }))) as any;
const MarkdownPanel = lazy(() => import("../MarkdownPanel").then(m => ({ default: m.MarkdownPanel }))) as any;
const CanvasPanel = lazy(() => import("../CanvasPanel")) as any;
const ColorPalettePanel = lazy(() => import("../ColorPalettePanel").then(m => ({ default: m.ColorPalettePanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
}

export function EditorsComposite({ workspacePath: wp }: Props) {
  return (
    <TabbedPanel tabs={[
      { id: "difftool", label: "Diff", content: <Suspense fallback={<Loading />}><DiffToolPanel /></Suspense> },
      { id: "markdown", label: "Markdown", content: <Suspense fallback={<Loading />}><MarkdownPanel workspacePath={wp} /></Suspense> },
      { id: "canvas", label: "Canvas", content: <Suspense fallback={<Loading />}><CanvasPanel /></Suspense> },
      { id: "colors", label: "Palette", content: <Suspense fallback={<Loading />}><ColorPalettePanel workspacePath={wp} /></Suspense> },
    ]} />
  );
}
