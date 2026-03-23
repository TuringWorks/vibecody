import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const CicdPanel = lazy(() => import("../CicdPanel")) as any;
const CiStatusPanel = lazy(() => import("../CiStatusPanel")) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
  provider: string;
}

export function CiCdComposite({ workspacePath, provider }: Props) {
  const wp = workspacePath;
  return (
    <TabbedPanel tabs={[
      { id: "pipeline", label: "Pipeline", content: <Suspense fallback={<Loading />}><CicdPanel workspacePath={wp} provider={provider} /></Suspense> },
      { id: "status", label: "Status", content: <Suspense fallback={<Loading />}><CiStatusPanel provider={provider} /></Suspense> },
    ]} />
  );
}
