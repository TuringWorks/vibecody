import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const ScriptPanel = lazy(() => import("../ScriptPanel").then(m => ({ default: m.ScriptPanel }))) as any;
const SshPanel = lazy(() => import("../SshPanel").then(m => ({ default: m.SshPanel }))) as any;
const NotebookPanel = lazy(() => import("../NotebookPanel").then(m => ({ default: m.NotebookPanel }))) as any;
const LogPanel = lazy(() => import("../LogPanel").then(m => ({ default: m.LogPanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
  provider: string;
}

export function TerminalComposite({ workspacePath, provider }: Props) {
  const wp = workspacePath;
  return (
    <TabbedPanel tabs={[
      { id: "scripts", label: "Scripts", content: <Suspense fallback={<Loading />}><ScriptPanel workspacePath={wp} /></Suspense> },
      { id: "ssh", label: "SSH", content: <Suspense fallback={<Loading />}><SshPanel workspacePath={wp} /></Suspense> },
      { id: "notebook", label: "Notebook", content: <Suspense fallback={<Loading />}><NotebookPanel workspacePath={wp} provider={provider} /></Suspense> },
      { id: "logs", label: "Logs", content: <Suspense fallback={<Loading />}><LogPanel workspacePath={wp} /></Suspense> },
    ]} />
  );
}
