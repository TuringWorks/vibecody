import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const CloudProviderPanel = lazy(() => import("../CloudProviderPanel").then(m => ({ default: m.CloudProviderPanel }))) as any;
const EnvPanel = lazy(() => import("../EnvPanel").then(m => ({ default: m.EnvPanel }))) as any;
const HealthMonitorPanel = lazy(() => import("../HealthMonitorPanel").then(m => ({ default: m.HealthMonitorPanel }))) as any;
const IdpPanel = lazy(() => import("../IdpPanel").then(m => ({ default: m.IdpPanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
  provider: string;
}

export function CloudPlatformComposite({ workspacePath, provider }: Props) {
  const wp = workspacePath;
  return (
    <TabbedPanel tabs={[
      { id: "providers", label: "Providers", content: <Suspense fallback={<Loading />}><CloudProviderPanel workspacePath={wp} /></Suspense> },
      { id: "env", label: "Environment", content: <Suspense fallback={<Loading />}><EnvPanel workspacePath={wp} /></Suspense> },
      { id: "health", label: "Health", content: <Suspense fallback={<Loading />}><HealthMonitorPanel /></Suspense> },
      { id: "idp", label: "IDP", content: <Suspense fallback={<Loading />}><IdpPanel provider={provider} /></Suspense> },
    ]} />
  );
}
