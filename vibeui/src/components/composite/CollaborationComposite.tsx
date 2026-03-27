import { createComposite } from "./createComposite";

export const CollaborationComposite = createComposite([
  { id: "collab", label: "Collab", importFn: () => import("../CollabPanel"), exportName: "CollabPanel" },
  { id: "remotecontrol", label: "Remote", importFn: () => import("../RemoteControlPanel"), exportName: "RemoteControlPanel" },
]);
