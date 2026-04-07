import { createComposite } from "./createComposite";

export const CollaborationComposite = createComposite([
  { id: "collab-chat", label: "Collab Chat", importFn: () => import("../CollabChatPanel"), exportName: "CollabChatPanel" },
  { id: "collab", label: "Collab", importFn: () => import("../CollabPanel"), exportName: "CollabPanel" },
  { id: "remotecontrol", label: "Remote", importFn: () => import("../RemoteControlPanel"), exportName: "RemoteControlPanel" },
  { id: "gateway-sandbox", label: "Msg Gateway", importFn: () => import("../GatewaySandboxPanel"), exportName: "GatewaySandboxPanel" },
]);
