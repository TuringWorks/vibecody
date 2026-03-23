import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const BuildPanel = lazy(() => import("../BuildPanel").then(m => ({ default: m.BuildPanel }))) as any;
const DeployPanel = lazy(() => import("../DeployPanel").then(m => ({ default: m.DeployPanel }))) as any;
const ScaffoldPanel = lazy(() => import("../ScaffoldPanel").then(m => ({ default: m.ScaffoldPanel }))) as any;
const AppBuilderPanel = lazy(() => import("../AppBuilderPanel").then(m => ({ default: m.AppBuilderPanel }))) as any;
const FullStackGenPanel = lazy(() => import("../FullStackGenPanel")) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
  provider: string;
  currentFile: string | null;
  onOpenFile?: (path: string, line?: number) => void;
}

export function BuildDeployComposite({ workspacePath, provider, currentFile, onOpenFile }: Props) {
  const wp = workspacePath;
  return (
    <TabbedPanel tabs={[
      { id: "build", label: "Build", content: <Suspense fallback={<Loading />}><BuildPanel workspacePath={wp} currentFile={currentFile} onOpenFile={onOpenFile} /></Suspense> },
      { id: "deploy", label: "Deploy", content: <Suspense fallback={<Loading />}><DeployPanel workspacePath={wp} /></Suspense> },
      { id: "scaffold", label: "Scaffold", content: <Suspense fallback={<Loading />}><ScaffoldPanel workspacePath={wp} provider={provider} /></Suspense> },
      { id: "appbuilder", label: "App Builder", content: <Suspense fallback={<Loading />}><AppBuilderPanel workspacePath={wp || ""} provider={provider} /></Suspense> },
      { id: "fullstack", label: "Full Stack", content: <Suspense fallback={<Loading />}><FullStackGenPanel provider={provider} /></Suspense> },
    ]} />
  );
}
