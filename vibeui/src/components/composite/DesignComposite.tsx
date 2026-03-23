import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";
const DesignMode = lazy(() => import("../DesignMode").then(m => ({ default: m.DesignMode }))) as any;
const RemoteControlPanel = lazy(() => import("../RemoteControlPanel")) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
  provider: string;
}

export function DesignComposite({ workspacePath: wp, provider }: Props) {
  return (
    <TabbedPanel tabs={[
      { id: "design", label: "Design", content: <Suspense fallback={<Loading />}><DesignMode workspacePath={wp} provider={provider} /></Suspense> },
      { id: "remotecontrol", label: "Remote Control", content: <Suspense fallback={<Loading />}><RemoteControlPanel provider={provider} /></Suspense> },
    ]} />
  );
}
