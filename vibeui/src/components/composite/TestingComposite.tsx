import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";
const TestPanel = lazy(() => import("../TestPanel").then(m => ({ default: m.TestPanel }))) as any;
const CoveragePanel = lazy(() => import("../CoveragePanel").then(m => ({ default: m.CoveragePanel }))) as any;
const BugBotPanel = lazy(() => import("../BugBotPanel").then(m => ({ default: m.BugBotPanel }))) as any;
const AutofixPanel = lazy(() => import("../AutofixPanel").then(m => ({ default: m.AutofixPanel }))) as any;
const CloudAutofixPanel = lazy(() => import("../CloudAutofixPanel")) as any;
const QaValidationPanel = lazy(() => import("../QaValidationPanel").then(m => ({ default: m.QaValidationPanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
  provider: string;
  onOpenFile?: (path: string, line?: number) => void;
}

export function TestingComposite({ workspacePath, provider, onOpenFile }: Props) {
  const wp = workspacePath;
  return (
    <TabbedPanel tabs={[
      { id: "tests", label: "Tests", content: <Suspense fallback={<Loading />}><TestPanel workspacePath={wp} /></Suspense> },
      { id: "coverage", label: "Coverage", content: <Suspense fallback={<Loading />}><CoveragePanel workspacePath={wp} /></Suspense> },
      { id: "bugbot", label: "BugBot", content: <Suspense fallback={<Loading />}><BugBotPanel workspacePath={wp || undefined} onOpenFile={onOpenFile} /></Suspense> },
      { id: "autofix", label: "Autofix", content: <Suspense fallback={<Loading />}><AutofixPanel workspacePath={wp} provider={provider} /></Suspense> },
      { id: "cloudautofix", label: "Cloud Fix", content: <Suspense fallback={<Loading />}><CloudAutofixPanel provider={provider} /></Suspense> },
      { id: "qa-validation", label: "QA", content: <Suspense fallback={<Loading />}><QaValidationPanel provider={provider} /></Suspense> },
    ]} />
  );
}
