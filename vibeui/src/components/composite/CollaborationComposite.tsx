import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";
const CollabPanel = lazy(() => import("../CollabPanel").then(m => ({ default: m.CollabPanel }))) as any;
const CompliancePanel = lazy(() => import("../CompliancePanel").then(m => ({ default: m.CompliancePanel }))) as any;

const Loading = () => <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: 13 }}>Loading...</div>;

interface Props {
  collab: {
    connected: boolean;
    roomId: string | null;
    peerId: string | null;
    peers: Array<{ peerId: string; name: string; color: string }>;
    connect: (...args: any[]) => void;
    disconnect: () => void;
  };
}

export function CollaborationComposite({ collab }: Props) {
  return (
    <TabbedPanel tabs={[
      {
        id: "collab",
        label: "Collab",
        content: (
          <Suspense fallback={<Loading />}>
            <CollabPanel
              connected={collab.connected}
              roomId={collab.roomId || ""}
              peerId={collab.peerId || ""}
              peers={collab.peers}
              onConnect={collab.connect}
              onDisconnect={collab.disconnect}
            />
          </Suspense>
        ),
      },
      { id: "compliance", label: "Compliance", content: <Suspense fallback={<Loading />}><CompliancePanel /></Suspense> },
    ]} />
  );
}
