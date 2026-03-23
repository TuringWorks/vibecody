import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const McpPanel = lazy(() => import("../McpPanel").then(m => ({ default: m.McpPanel }))) as any;
const AcpPanel = lazy(() => import("../AcpPanel").then(m => ({ default: m.AcpPanel }))) as any;
const WebhookPanel = lazy(() => import("../WebhookPanel").then(m => ({ default: m.WebhookPanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  provider: string;
}

export function IntegrationsComposite({ provider }: Props) {
  return (
    <TabbedPanel tabs={[
      { id: "mcp", label: "MCP", content: <Suspense fallback={<Loading />}><McpPanel /></Suspense> },
      { id: "acpprotocol", label: "ACP", content: <Suspense fallback={<Loading />}><AcpPanel provider={provider} /></Suspense> },
      { id: "webhooks", label: "Webhooks", content: <Suspense fallback={<Loading />}><WebhookPanel /></Suspense> },
    ]} />
  );
}
