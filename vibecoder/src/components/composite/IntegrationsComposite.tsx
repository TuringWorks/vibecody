import { createComposite } from "./createComposite";

export const IntegrationsComposite = createComposite([
  { id: "mcp", label: "MCP", importFn: () => import("../McpPanel"), exportName: "McpPanel" },
  { id: "acpprotocol", label: "ACP", importFn: () => import("../AcpPanel"), exportName: "AcpPanel" },
  { id: "webhooks", label: "Webhooks", importFn: () => import("../WebhookPanel"), exportName: "WebhookPanel" },
  { id: "connectors", label: "Connectors", importFn: () => import("../ConnectorsPanel"), exportName: "ConnectorsPanel" },
  { id: "a2a", label: "A2A", importFn: () => import("../A2aPanel"), exportName: "A2aPanel" },
  { id: "langgraph", label: "LangGraph", importFn: () => import("../LangGraphPanel"), exportName: "LangGraphPanel" },
  { id: "ide-bridge", label: "IDE Bridge", importFn: () => import("../IdeBridgePanel"), exportName: "IdeBridgePanel" },
]);
