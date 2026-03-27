import { createComposite } from "./createComposite";

export const IntegrationsComposite = createComposite([
  { id: "mcp", label: "MCP", importFn: () => import("../McpPanel"), exportName: "McpPanel" },
  { id: "acpprotocol", label: "ACP", importFn: () => import("../AcpPanel"), exportName: "AcpPanel" },
  { id: "webhooks", label: "Webhooks", importFn: () => import("../WebhookPanel"), exportName: "WebhookPanel" },
  { id: "connectors", label: "Connectors", importFn: () => import("../ConnectorsPanel"), exportName: "ConnectorsPanel" },
]);
