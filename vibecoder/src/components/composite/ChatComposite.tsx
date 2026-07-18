import { lazy, Suspense, useState } from "react";
import { TabbedPanel } from "../TabbedPanel";
import { useWatchActiveSession } from "../../hooks/useWatchSync";
import { PinnedGoalBanner } from "../PinnedGoalBanner";

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
  /** /goal slash command → forwarded down to AIChat. */
  onSwitchToGoals?: (seed?: string) => void;
  /** G9.1 — workspace this VibeCoder instance is rooted in. Used by the
   *  PinnedGoalBanner to look up the right `current` pin row. */
  workspacePath?: string | null;
}

export function ChatComposite({
  defaultProvider,
  availableProviders,
  context,
  fileTree,
  currentFile,
  onPendingWrite,
  onSwitchToGoals,
  workspacePath,
}: ChatCompositeProps) {
  const [activeTab, setActiveTab] = useState("chat");

  // When Watch opens a sandbox conversation, auto-switch VibeCoder to the Sandbox tab.
  // Sandbox session IDs start with "sbx-" (derived from sandbox path hash).
  useWatchActiveSession((watchSessionId) => {
    if (watchSessionId.startsWith('sbx-')) {
      setActiveTab('sandbox');
    }
    // Regular session switching is handled inside ChatTabManager
  });

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0 }}>
      <PinnedGoalBanner workspacePath={workspacePath ?? null} />
      <div style={{ flex: 1, minHeight: 0 }}>
        <TabbedPanel
          activeTab={activeTab}
          onTabChange={setActiveTab}
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
                onSwitchToGoals={onSwitchToGoals}
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
      </div>
    </div>
  );
}
