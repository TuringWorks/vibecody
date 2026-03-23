import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const HttpPlayground = lazy(() => import("../HttpPlayground").then(m => ({ default: m.HttpPlayground }))) as any;
const GraphQLPanel = lazy(() => import("../GraphQLPanel").then(m => ({ default: m.GraphQLPanel }))) as any;
const MockServerPanel = lazy(() => import("../MockServerPanel").then(m => ({ default: m.MockServerPanel }))) as any;
const WebSocketPanel = lazy(() => import("../WebSocketPanel").then(m => ({ default: m.WebSocketPanel }))) as any;
const ApiDocsPanel = lazy(() => import("../ApiDocsPanel").then(m => ({ default: m.ApiDocsPanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
  provider: string;
}

export function ApiToolsComposite({ workspacePath, provider }: Props) {
  const wp = workspacePath;
  return (
    <TabbedPanel tabs={[
      { id: "http", label: "HTTP", content: <Suspense fallback={<Loading />}><HttpPlayground workspacePath={wp} provider={provider} /></Suspense> },
      { id: "graphql", label: "GraphQL", content: <Suspense fallback={<Loading />}><GraphQLPanel provider={provider} /></Suspense> },
      { id: "mock", label: "Mock Server", content: <Suspense fallback={<Loading />}><MockServerPanel /></Suspense> },
      { id: "websocket", label: "WebSocket", content: <Suspense fallback={<Loading />}><WebSocketPanel /></Suspense> },
      { id: "docs", label: "Docs", content: <Suspense fallback={<Loading />}><ApiDocsPanel workspacePath={wp} provider={provider} /></Suspense> },
    ]} />
  );
}
