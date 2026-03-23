import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";
const TransformPanel = lazy(() => import("../TransformPanel").then(m => ({ default: m.TransformPanel }))) as any;
const CodeMetricsPanel = lazy(() => import("../CodeMetricsPanel").then(m => ({ default: m.CodeMetricsPanel }))) as any;
const AstEditPanel = lazy(() => import("../AstEditPanel")) as any;
const EditPredictionPanel = lazy(() => import("../EditPredictionPanel")) as any;
const SnippetPanel = lazy(() => import("../SnippetPanel").then(m => ({ default: m.SnippetPanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
  provider: string;
}

export function CodeAnalysisComposite({ workspacePath, provider }: Props) {
  const wp = workspacePath;
  return (
    <TabbedPanel tabs={[
      { id: "transform", label: "Transform", content: <Suspense fallback={<Loading />}><TransformPanel provider={provider} /></Suspense> },
      { id: "metrics", label: "Metrics", content: <Suspense fallback={<Loading />}><CodeMetricsPanel workspacePath={wp} /></Suspense> },
      { id: "astedit", label: "AST Edit", content: <Suspense fallback={<Loading />}><AstEditPanel provider={provider} /></Suspense> },
      { id: "editpredict", label: "Predict", content: <Suspense fallback={<Loading />}><EditPredictionPanel provider={provider} /></Suspense> },
      { id: "snippets", label: "Snippets", content: <Suspense fallback={<Loading />}><SnippetPanel workspacePath={wp} /></Suspense> },
    ]} />
  );
}
