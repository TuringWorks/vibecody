import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";
const BatchBuilderPanel = lazy(() => import("../BatchBuilderPanel")) as any;
const ImageGenPanel = lazy(() => import("../ImageGenPanel")) as any;
const AutoResearchPanel = lazy(() => import("../AutoResearchPanel").then(m => ({ default: m.AutoResearchPanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string;
  provider: string;
}

export function AiGenerationComposite({ workspacePath, provider }: Props) {
  return (
    <TabbedPanel tabs={[
      { id: "batchbuilder", label: "Batch Builder", content: <Suspense fallback={<Loading />}><BatchBuilderPanel provider={provider} /></Suspense> },
      { id: "imagegen", label: "Image Gen", content: <Suspense fallback={<Loading />}><ImageGenPanel provider={provider} /></Suspense> },
      { id: "autoresearch", label: "Research", content: <Suspense fallback={<Loading />}><AutoResearchPanel workspacePath={workspacePath} provider={provider} /></Suspense> },
    ]} />
  );
}
