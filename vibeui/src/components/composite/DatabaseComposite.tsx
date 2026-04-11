import { createComposite } from "./createComposite";

export const DatabaseComposite = createComposite([
  { id: "connections", label: "Connections", importFn: () => import("../DatabasePanel"), exportName: "DatabasePanel" },
  { id: "supabase", label: "Supabase", importFn: () => import("../SupabasePanel"), exportName: "SupabasePanel" },
  { id: "migrations", label: "Migrations", importFn: () => import("../MigrationsPanel"), exportName: "MigrationsPanel" },
  { id: "vectordb", label: "Vector DB", importFn: () => import("../VectorDbPanel"), exportName: "VectorDbPanel" },
  { id: "turboquant", label: "TurboQuant", importFn: () => import("../TurboQuantPanel"), exportName: "TurboQuantPanel" },
]);
