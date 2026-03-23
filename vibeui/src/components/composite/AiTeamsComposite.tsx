import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";
const AgentTeamPanel = lazy(() => import("../AgentTeamPanel").then(m => ({ default: m.AgentTeamPanel }))) as any;
const AgentTeamsPanel = lazy(() => import("../AgentTeamsPanel")) as any;
const SubAgentPanel = lazy(() => import("../SubAgentPanel")) as any;
const CloudAgentPanel = lazy(() => import("../CloudAgentPanel").then(m => ({ default: m.CloudAgentPanel }))) as any;
const CIReviewPanel = lazy(() => import("../CIReviewPanel").then(m => ({ default: m.CIReviewPanel }))) as any;
const AgentModesPanel = lazy(() => import("../AgentModesPanel")) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  provider: string;
}

export function AiTeamsComposite({ provider }: Props) {
  return (
    <TabbedPanel tabs={[
      { id: "teams", label: "Teams", content: <Suspense fallback={<Loading />}><AgentTeamPanel provider={provider} /></Suspense> },
      { id: "agentteams", label: "Hierarchy", content: <Suspense fallback={<Loading />}><AgentTeamsPanel provider={provider} /></Suspense> },
      { id: "subagents", label: "Sub-Agents", content: <Suspense fallback={<Loading />}><SubAgentPanel provider={provider} /></Suspense> },
      { id: "cloud", label: "Cloud", content: <Suspense fallback={<Loading />}><CloudAgentPanel provider={provider} /></Suspense> },
      { id: "cibot", label: "CI Bot", content: <Suspense fallback={<Loading />}><CIReviewPanel provider={provider} /></Suspense> },
      { id: "agentmodes", label: "Modes", content: <Suspense fallback={<Loading />}><AgentModesPanel /></Suspense> },
    ]} />
  );
}
