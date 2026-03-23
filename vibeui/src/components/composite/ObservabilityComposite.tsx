import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";
const TraceDashboard = lazy(() => import("../TraceDashboard").then(m => ({ default: m.TraceDashboard }))) as any;
const AgentRecordingPanel = lazy(() => import("../AgentRecordingPanel").then(m => ({ default: m.AgentRecordingPanel }))) as any;
const DemoPanel = lazy(() => import("../DemoPanel").then(m => ({ default: m.DemoPanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  provider: string;
}

export function ObservabilityComposite({ provider }: Props) {
  return (
    <TabbedPanel tabs={[
      { id: "traces", label: "Traces", content: <Suspense fallback={<Loading />}><TraceDashboard /></Suspense> },
      { id: "recording", label: "Recording", content: <Suspense fallback={<Loading />}><AgentRecordingPanel provider={provider} /></Suspense> },
      { id: "demo", label: "Demo", content: <Suspense fallback={<Loading />}><DemoPanel /></Suspense> },
    ]} />
  );
}
