import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const DepsPanel = lazy(() => import("../DepsPanel").then(m => ({ default: m.DepsPanel }))) as any;
const NetworkPanel = lazy(() => import("../NetworkPanel").then(m => ({ default: m.NetworkPanel }))) as any;
const LoadTestPanel = lazy(() => import("../LoadTestPanel").then(m => ({ default: m.LoadTestPanel }))) as any;
const RenderOptimizePanel = lazy(() => import("../RenderOptimizePanel")) as any;
const SweBenchPanel = lazy(() => import("../SweBenchPanel").then(m => ({ default: m.SweBenchPanel }))) as any;
const SessionMemoryPanel = lazy(() => import("../SessionMemoryPanel").then(m => ({ default: m.SessionMemoryPanel }))) as any;
const ResiliencePanel = lazy(() => import("../ResiliencePanel").then(m => ({ default: m.ResiliencePanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
  provider: string;
}

export function DiagnosticsComposite({ workspacePath, provider }: Props) {
  const wp = workspacePath;
  return (
    <TabbedPanel tabs={[
      { id: "deps", label: "Deps", content: <Suspense fallback={<Loading />}><DepsPanel workspacePath={wp} /></Suspense> },
      { id: "network", label: "Network", content: <Suspense fallback={<Loading />}><NetworkPanel /></Suspense> },
      { id: "loadtest", label: "Load Test", content: <Suspense fallback={<Loading />}><LoadTestPanel provider={provider} /></Suspense> },
      { id: "renderopt", label: "Render", content: <Suspense fallback={<Loading />}><RenderOptimizePanel /></Suspense> },
      { id: "swebench", label: "SWE-Bench", content: <Suspense fallback={<Loading />}><SweBenchPanel provider={provider} /></Suspense> },
      { id: "sessionmemory", label: "Memory", content: <Suspense fallback={<Loading />}><SessionMemoryPanel /></Suspense> },
      { id: "resilience", label: "Resilience", content: <Suspense fallback={<Loading />}><ResiliencePanel /></Suspense> },
    ]} />
  );
}
