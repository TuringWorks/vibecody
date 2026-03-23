import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";
const WorkManagementPanel = lazy(() => import("../WorkManagementPanel")) as any;
const DashboardPanel = lazy(() => import("../DashboardPanel")) as any;
const SteeringPanel = lazy(() => import("../SteeringPanel")) as any;
const SoulPanel = lazy(() => import("../SoulPanel").then(m => ({ default: m.SoulPanel }))) as any;
const MemoryPanel = lazy(() => import("../MemoryPanel").then(m => ({ default: m.MemoryPanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
  provider: string;
}

export function ProjectHubComposite({ workspacePath: wp, provider }: Props) {
  return (
    <TabbedPanel tabs={[
      { id: "workmgmt", label: "Work Mgmt", content: <Suspense fallback={<Loading />}><WorkManagementPanel /></Suspense> },
      { id: "dashboard", label: "Dashboard", content: <Suspense fallback={<Loading />}><DashboardPanel provider={provider} /></Suspense> },
      { id: "steering", label: "Steering", content: <Suspense fallback={<Loading />}><SteeringPanel workspaceRoot={wp || undefined} /></Suspense> },
      { id: "soul", label: "Soul", content: <Suspense fallback={<Loading />}><SoulPanel workspacePath={wp} provider={provider} /></Suspense> },
      { id: "rules", label: "Rules", content: <Suspense fallback={<Loading />}><MemoryPanel workspacePath={wp} /></Suspense> },
    ]} />
  );
}
