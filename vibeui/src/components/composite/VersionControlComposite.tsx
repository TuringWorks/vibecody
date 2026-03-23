import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";
const HistoryPanel = lazy(() => import("../HistoryPanel").then(m => ({ default: m.HistoryPanel }))) as any;
const CheckpointPanel = lazy(() => import("../CheckpointPanel").then(m => ({ default: m.CheckpointPanel }))) as any;
const BisectPanel = lazy(() => import("../BisectPanel").then(m => ({ default: m.BisectPanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
}

export function VersionControlComposite({ workspacePath }: Props) {
  const wp = workspacePath;
  return (
    <TabbedPanel tabs={[
      { id: "history", label: "History", content: <Suspense fallback={<Loading />}><HistoryPanel /></Suspense> },
      { id: "checkpoints", label: "Checkpoints", content: <Suspense fallback={<Loading />}><CheckpointPanel workspacePath={wp} /></Suspense> },
      { id: "bisect", label: "Bisect", content: <Suspense fallback={<Loading />}><BisectPanel workspacePath={wp} /></Suspense> },
    ]} />
  );
}
