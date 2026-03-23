import { createComposite } from "./createComposite";

export const DesignComposite = createComposite([
  { id: "design", label: "Design", importFn: () => import("../DesignMode"), exportName: "DesignMode" },
  { id: "remotecontrol", label: "Remote Control", importFn: () => import("../RemoteControlPanel") },
]);
