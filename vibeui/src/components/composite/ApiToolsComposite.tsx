import { createComposite } from "./createComposite";

export const ApiToolsComposite = createComposite([
  { id: "http", label: "HTTP", importFn: () => import("../HttpPlayground"), exportName: "HttpPlayground" },
  { id: "graphql", label: "GraphQL", importFn: () => import("../GraphQLPanel"), exportName: "GraphQLPanel" },
  { id: "mock", label: "Mock Server", importFn: () => import("../MockServerPanel"), exportName: "MockServerPanel" },
  { id: "websocket", label: "WebSocket", importFn: () => import("../WebSocketPanel"), exportName: "WebSocketPanel" },
  { id: "docs", label: "Docs", importFn: () => import("../ApiDocsPanel"), exportName: "ApiDocsPanel" },
]);
