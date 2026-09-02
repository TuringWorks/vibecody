import { createComposite } from "./createComposite";

export const CloudPlatformComposite = createComposite([
  { id: "providers", label: "Providers", importFn: () => import("../CloudProviderPanel"), exportName: "CloudProviderPanel" },
  { id: "env", label: "Environment", importFn: () => import("../EnvPanel"), exportName: "EnvPanel" },
  { id: "health", label: "Health", importFn: () => import("../HealthMonitorPanel"), exportName: "HealthMonitorPanel" },
  { id: "idp", label: "IDP", importFn: () => import("../IdpPanel"), exportName: "IdpPanel" },
  // Placed beside IDP on purpose: the IDP scorecard grades a service's
  // *metadata*, while this one measures the delivery system the platform
  // exists to improve. Reading them together is the point.
  { id: "devex", label: "Developer Excellence", importFn: () => import("../DeveloperExcellencePanel"), exportName: "DeveloperExcellencePanel" },
], { panelId: "cloud-platform" });
