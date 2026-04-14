import { lazy, Suspense } from "react";
import { TabbedPanel } from "../TabbedPanel";

const Loading = () => (
  <div style={{ padding: 16, color: "var(--text-secondary)", fontSize: "var(--font-size-md)" }}>Loading...</div>
);

const ChatTabManager = lazy(() =>
  import("../ChatTabManager").then((m) => ({ default: m.ChatTabManager }))
);
const SandboxChatPanel = lazy(() =>
  import("../SandboxChatPanel").then((m) => ({ default: m.SandboxChatPanel }))
);

export interface ChatCompositeProps {
  defaultProvider: string;
  availableProviders: string[];
  context?: string;
  fileTree?: string[];
  currentFile?: string | null;
  onPendingWrite?: (path: string, content: string) => void;
}

export function ChatComposite({
  defaultProvider,
  availableProviders,
  context,
  fileTree,
  currentFile,
  onPendingWrite,
}: ChatCompositeProps) {
  return (
    <TabbedPanel
      tabs={[
        {
          id: "chat",
          label: "Chat",
          content: (
            <Suspense fallback={<Loading />}>
              <ChatTabManager
                defaultProvider={defaultProvider}
                availableProviders={availableProviders}
                context={context}
                fileTree={fileTree}
                currentFile={currentFile}
                onPendingWrite={onPendingWrite}
              />
            </Suspense>
          ),
        },
        {
          id: "sandbox",
          label: "Sandbox",
          content: (
            <Suspense fallback={<Loading />}>
              <SandboxChatPanel
                provider={defaultProvider}
                availableProviders={availableProviders}
              />
            </Suspense>
          ),
        },
      ]}
    />
  );
}
