import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";
const GitHubSyncPanel = lazy(() => import("../GitHubSyncPanel").then(m => ({ default: m.GitHubSyncPanel }))) as any;
const GhActionsPanel = lazy(() => import("../GhActionsPanel")) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
  provider: string;
}

export function GitHubComposite({ workspacePath, provider }: Props) {
  const wp = workspacePath;
  return (
    <TabbedPanel tabs={[
      { id: "github", label: "Sync", content: <Suspense fallback={<Loading />}><GitHubSyncPanel workspacePath={wp} /></Suspense> },
      { id: "ghactions", label: "Actions", content: <Suspense fallback={<Loading />}><GhActionsPanel provider={provider} /></Suspense> },
    ]} />
  );
}
