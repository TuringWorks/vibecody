import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";
const InfiniteContextPanel = lazy(() => import("../InfiniteContextPanel").then(m => ({ default: m.InfiniteContextPanel }))) as any;
const ContextBundlePanel = lazy(() => import("../ContextBundlePanel").then(m => ({ default: m.ContextBundlePanel }))) as any;
const OpenMemoryPanel = lazy(() => import("../OpenMemoryPanel")) as any;
const FastContextPanel = lazy(() => import("../FastContextPanel")) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string;
  provider: string;
}

export function AiContextComposite({ workspacePath, provider }: Props) {
  return (
    <TabbedPanel tabs={[
      { id: "icontext", label: "Infinite Context", content: <Suspense fallback={<Loading />}><InfiniteContextPanel workspacePath={workspacePath} provider={provider} /></Suspense> },
      { id: "bundles", label: "Bundles", content: <Suspense fallback={<Loading />}><ContextBundlePanel workspacePath={workspacePath} provider={provider} /></Suspense> },
      { id: "openmemory", label: "Open Memory", content: <Suspense fallback={<Loading />}><OpenMemoryPanel /></Suspense> },
      { id: "fastcontext", label: "Fast Context", content: <Suspense fallback={<Loading />}><FastContextPanel provider={provider} /></Suspense> },
    ]} />
  );
}
