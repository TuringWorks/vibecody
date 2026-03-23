import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const VibeSqlPanel = lazy(() => import("../VibeSqlPanel").then(m => ({ default: m.VibeSqlPanel }))) as any;
const DatabasePanel = lazy(() => import("../DatabasePanel").then(m => ({ default: m.DatabasePanel }))) as any;
const SupabasePanel = lazy(() => import("../SupabasePanel").then(m => ({ default: m.SupabasePanel }))) as any;
const MigrationsPanel = lazy(() => import("../MigrationsPanel").then(m => ({ default: m.MigrationsPanel }))) as any;
const VectorDbPanel = lazy(() => import("../VectorDbPanel").then(m => ({ default: m.VectorDbPanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  workspacePath: string | null;
  provider: string;
}

export function DatabaseComposite({ workspacePath, provider }: Props) {
  const wp = workspacePath;
  return (
    <TabbedPanel tabs={[
      { id: "vibesql", label: "VibeSQL", content: <Suspense fallback={<Loading />}><VibeSqlPanel workspacePath={wp} provider={provider} /></Suspense> },
      { id: "connections", label: "Connections", content: <Suspense fallback={<Loading />}><DatabasePanel workspacePath={wp} provider={provider} /></Suspense> },
      { id: "supabase", label: "Supabase", content: <Suspense fallback={<Loading />}><SupabasePanel workspacePath={wp} provider={provider} /></Suspense> },
      { id: "migrations", label: "Migrations", content: <Suspense fallback={<Loading />}><MigrationsPanel workspacePath={wp} provider={provider} /></Suspense> },
      { id: "vectordb", label: "Vector DB", content: <Suspense fallback={<Loading />}><VectorDbPanel provider={provider} /></Suspense> },
    ]} />
  );
}
