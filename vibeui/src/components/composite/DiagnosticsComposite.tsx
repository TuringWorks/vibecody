import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const DepsPanel = lazy(() => import("../DepsPanel").then(m => ({ default: m.DepsPanel }))) as any;
const NetworkPanel = lazy(() => import("../NetworkPanel").then(m => ({ default: m.NetworkPanel }))) as any;
const LoadTestPanel = lazy(() => import("../LoadTestPanel").then(m => ({ default: m.LoadTestPanel }))) as any;
const RenderOptimizePanel = lazy(() => import("../RenderOptimizePanel")) as any;
const SmartDepsPanel = lazy(() => import("../SmartDepsPanel").then(m => ({ default: m.SmartDepsPanel }))) as any;
const ResiliencePanel = lazy(() => import("../ResiliencePanel").then(m => ({ default: m.ResiliencePanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
  provider: string;
  onOpenFile?: (path: string, line?: number) => void;
}

export function DiagnosticsComposite({ workspacePath, provider, onOpenFile }: Props) {
  const wp = workspacePath;
  return (
    <TabbedPanel tabs={[
      { id: "deps", label: "Deps", content: <Suspense fallback={<Loading />}><DepsPanel workspacePath={wp} onOpenFile={onOpenFile} /></Suspense> },
      { id: "network", label: "Network", content: <Suspense fallback={<Loading />}><NetworkPanel /></Suspense> },
      { id: "loadtest", label: "Load Test", content: <Suspense fallback={<Loading />}><LoadTestPanel provider={provider} /></Suspense> },
      { id: "renderopt", label: "Render", content: <Suspense fallback={<Loading />}><RenderOptimizePanel /></Suspense> },
      { id: "resilience", label: "Resilience", content: <Suspense fallback={<Loading />}><ResiliencePanel /></Suspense> },
      { id: "smartdeps", label: "Smart Deps", content: <Suspense fallback={<Loading />}><SmartDepsPanel onOpenFile={onOpenFile} /></Suspense> },
    ]} />
  );
}
