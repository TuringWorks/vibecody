import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";
const RedTeamPanel = lazy(() => import("../RedTeamPanel").then(m => ({ default: m.RedTeamPanel }))) as any;
const BlueTeamPanel = lazy(() => import("../BlueTeamPanel").then(m => ({ default: m.BlueTeamPanel }))) as any;
const PurpleTeamPanel = lazy(() => import("../PurpleTeamPanel").then(m => ({ default: m.PurpleTeamPanel }))) as any;
const SecurityScanPanel = lazy(() => import("../SecurityScanPanel")) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
  provider: string;
  onOpenFile?: (path: string, line?: number) => void;
}

export function SecurityComposite({ workspacePath, provider, onOpenFile }: Props) {
  const wp = workspacePath;
  return (
    <TabbedPanel tabs={[
      { id: "redteam", label: "Red Team", content: <Suspense fallback={<Loading />}><RedTeamPanel workspacePath={wp} provider={provider} /></Suspense> },
      { id: "blueteam", label: "Blue Team", content: <Suspense fallback={<Loading />}><BlueTeamPanel provider={provider} /></Suspense> },
      { id: "purpleteam", label: "Purple Team", content: <Suspense fallback={<Loading />}><PurpleTeamPanel provider={provider} /></Suspense> },
      { id: "securityscan", label: "Scanner", content: <Suspense fallback={<Loading />}><SecurityScanPanel workspacePath={wp || undefined} onOpenFile={onOpenFile} provider={provider} /></Suspense> },
    ]} />
  );
}
