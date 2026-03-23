import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const AdminPanel = lazy(() => import("../AdminPanel").then(m => ({ default: m.AdminPanel }))) as any;
const AuthPanel = lazy(() => import("../AuthPanel").then(m => ({ default: m.AuthPanel }))) as any;
const TeamGovernancePanel = lazy(() => import("../TeamGovernancePanel")) as any;
const SessionBrowserPanel = lazy(() => import("../SessionBrowserPanel")) as any;
const ManagerView = lazy(() => import("../ManagerView").then(m => ({ default: m.ManagerView }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
  provider: string;
}

export function AdministrationComposite({ workspacePath: wp, provider }: Props) {
  return (
    <TabbedPanel tabs={[
      { id: "admin", label: "Admin", content: <Suspense fallback={<Loading />}><AdminPanel /></Suspense> },
      { id: "auth", label: "Auth", content: <Suspense fallback={<Loading />}><AuthPanel workspacePath={wp} provider={provider} /></Suspense> },
      { id: "governance", label: "Governance", content: <Suspense fallback={<Loading />}><TeamGovernancePanel provider={provider} /></Suspense> },
      { id: "sessions", label: "Sessions", content: <Suspense fallback={<Loading />}><SessionBrowserPanel /></Suspense> },
      { id: "manager", label: "Manager", content: <Suspense fallback={<Loading />}><ManagerView provider={provider} /></Suspense> },
    ]} />
  );
}
