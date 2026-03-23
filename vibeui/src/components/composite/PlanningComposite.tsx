import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";
const SpecPanel = lazy(() => import("../SpecPanel").then(m => ({ default: m.SpecPanel }))) as any;
const PlanDocumentPanel = lazy(() => import("../PlanDocumentPanel")) as any;
const WorkflowPanel = lazy(() => import("../WorkflowPanel").then(m => ({ default: m.WorkflowPanel }))) as any;
const OrchestrationPanel = lazy(() => import("../OrchestrationPanel").then(m => ({ default: m.OrchestrationPanel }))) as any;
const ClarifyingQuestionsPanel = lazy(() => import("../ClarifyingQuestionsPanel")) as any;
const ConversationalSearchPanel = lazy(() => import("../ConversationalSearchPanel")) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
  provider: string;
}

export function PlanningComposite({ workspacePath: wp, provider }: Props) {
  return (
    <TabbedPanel tabs={[
      { id: "specs", label: "Specs", content: <Suspense fallback={<Loading />}><SpecPanel workspacePath={wp} provider={provider} /></Suspense> },
      { id: "plandoc", label: "Plan Doc", content: <Suspense fallback={<Loading />}><PlanDocumentPanel provider={provider} /></Suspense> },
      { id: "workflow", label: "Workflow", content: <Suspense fallback={<Loading />}><WorkflowPanel workspacePath={wp} provider={provider} /></Suspense> },
      { id: "orchestration", label: "Orchestration", content: <Suspense fallback={<Loading />}><OrchestrationPanel workspacePath={wp} provider={provider} /></Suspense> },
      { id: "clarify", label: "Clarify", content: <Suspense fallback={<Loading />}><ClarifyingQuestionsPanel provider={provider} /></Suspense> },
      { id: "codesearch", label: "Search", content: <Suspense fallback={<Loading />}><ConversationalSearchPanel provider={provider} /></Suspense> },
    ]} />
  );
}
