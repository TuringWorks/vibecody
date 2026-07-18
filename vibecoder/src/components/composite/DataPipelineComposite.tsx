import { createComposite } from "./createComposite";

export const DataPipelineComposite = createComposite([
  { id: "streaming", label: "Streaming", importFn: () => import("../StreamingPanel"), exportName: "StreamingPanel" },
  { id: "ingest", label: "Ingest", importFn: () => import("../DocumentIngestPanel"), exportName: "DocumentIngestPanel" },
  { id: "crawler", label: "Crawler", importFn: () => import("../WebCrawlerPanel"), exportName: "WebCrawlerPanel" },
]);
