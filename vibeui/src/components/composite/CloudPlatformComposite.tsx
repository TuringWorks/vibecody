import { createComposite } from "./createComposite";

export const CloudPlatformComposite = createComposite([
  { id: "providers", label: "Providers", importFn: () => import("../CloudProviderPanel"), exportName: "CloudProviderPanel" },
  { id: "env", label: "Environment", importFn: () => import("../EnvPanel"), exportName: "EnvPanel" },
  { id: "health", label: "Health", importFn: () => import("../HealthMonitorPanel"), exportName: "HealthMonitorPanel" },
  { id: "idp", label: "IDP", importFn: () => import("../IdpPanel"), exportName: "IdpPanel" },
]);
