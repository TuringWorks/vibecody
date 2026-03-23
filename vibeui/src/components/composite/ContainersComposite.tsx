import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const DockerPanel = lazy(() => import("../DockerPanel").then(m => ({ default: m.DockerPanel }))) as any;
const K8sPanel = lazy(() => import("../K8sPanel")) as any;
const SandboxPanel = lazy(() => import("../SandboxPanel").then(m => ({ default: m.SandboxPanel }))) as any;
const CloudSandboxPanel = lazy(() => import("../CloudSandboxPanel")) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
  provider: string;
}

export function ContainersComposite({ workspacePath, provider }: Props) {
  const wp = workspacePath;
  return (
    <TabbedPanel tabs={[
      { id: "docker", label: "Docker", content: <Suspense fallback={<Loading />}><DockerPanel workspacePath={wp} /></Suspense> },
      { id: "k8s", label: "K8s", content: <Suspense fallback={<Loading />}><K8sPanel workspacePath={wp} /></Suspense> },
      { id: "sandbox", label: "Sandbox", content: <Suspense fallback={<Loading />}><SandboxPanel provider={provider} /></Suspense> },
      { id: "cloudsandbox", label: "Cloud Sandbox", content: <Suspense fallback={<Loading />}><CloudSandboxPanel provider={provider} /></Suspense> },
    ]} />
  );
}
