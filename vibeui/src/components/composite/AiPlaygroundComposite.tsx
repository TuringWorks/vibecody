import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";
const MultiModelPanel = lazy(() => import("../MultiModelPanel").then(m => ({ default: m.MultiModelPanel }))) as any;
const ArenaPanel = lazy(() => import("../ArenaPanel").then(m => ({ default: m.ArenaPanel }))) as any;
const CascadePanel = lazy(() => import("../CascadePanel").then(m => ({ default: m.CascadePanel }))) as any;
const DiscussionModePanel = lazy(() => import("../DiscussionModePanel")) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  provider: string;
  onInjectContext: (text: string) => void;
}

export function AiPlaygroundComposite({ provider, onInjectContext }: Props) {
  return (
    <TabbedPanel tabs={[
      { id: "compare", label: "Compare", content: <Suspense fallback={<Loading />}><MultiModelPanel provider={provider} /></Suspense> },
      { id: "arena", label: "Arena", content: <Suspense fallback={<Loading />}><ArenaPanel provider={provider} /></Suspense> },
      { id: "cascade", label: "Cascade", content: <Suspense fallback={<Loading />}><CascadePanel onInjectContext={onInjectContext} /></Suspense> },
      { id: "discuss", label: "Discussion", content: <Suspense fallback={<Loading />}><DiscussionModePanel provider={provider} /></Suspense> },
    ]} />
  );
}
