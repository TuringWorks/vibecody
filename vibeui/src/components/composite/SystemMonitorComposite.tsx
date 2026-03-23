import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const ProcessPanel = lazy(() => import("../ProcessPanel")) as any;
const ProfilerPanel = lazy(() => import("../ProfilerPanel").then(m => ({ default: m.ProfilerPanel }))) as any;
const DebugModePanel = lazy(() => import("../DebugModePanel")) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
}

export function SystemMonitorComposite({ workspacePath }: Props) {
  const wp = workspacePath;
  return (
    <TabbedPanel tabs={[
      { id: "processes", label: "Processes", content: <Suspense fallback={<Loading />}><ProcessPanel /></Suspense> },
      { id: "profiler", label: "Profiler", content: <Suspense fallback={<Loading />}><ProfilerPanel workspacePath={wp} /></Suspense> },
      { id: "debug", label: "Debug", content: <Suspense fallback={<Loading />}><DebugModePanel /></Suspense> },
    ]} />
  );
}
