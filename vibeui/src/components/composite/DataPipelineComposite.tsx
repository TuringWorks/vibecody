import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const StreamingPanel = lazy(() => import("../StreamingPanel").then(m => ({ default: m.StreamingPanel }))) as any;
const DocumentIngestPanel = lazy(() => import("../DocumentIngestPanel").then(m => ({ default: m.DocumentIngestPanel }))) as any;
const WebCrawlerPanel = lazy(() => import("../WebCrawlerPanel").then(m => ({ default: m.WebCrawlerPanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  provider: string;
}

export function DataPipelineComposite({ provider }: Props) {
  return (
    <TabbedPanel tabs={[
      { id: "streaming", label: "Streaming", content: <Suspense fallback={<Loading />}><StreamingPanel provider={provider} /></Suspense> },
      { id: "ingest", label: "Ingest", content: <Suspense fallback={<Loading />}><DocumentIngestPanel provider={provider} /></Suspense> },
      { id: "crawler", label: "Crawler", content: <Suspense fallback={<Loading />}><WebCrawlerPanel /></Suspense> },
    ]} />
  );
}
