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
  /** `/goals` slash command → forwarded down to AIChat. */
  onSwitchToGoals?: () => void;
  /** G9.1 — workspace this VibeCoder instance is rooted in. Used by the
   *  PinnedGoalBanner to look up the right `current` pin row. */
  workspacePath?: string | null;
  /** Show a file in the editor, by absolute path.
   *
   * The spoken path is the only one that needs it: a typed answer that
   * mentions a file leaves the user a name to click, but "open the config"
   * asked out loud has no click in it. Passing it is also what declares the
   * capability to the daemon — see `useVoiceDuplex`'s `onOpenFile`. */
  onOpenFile?: (path: string) => void;
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
  onOpenFile,
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
          panelId="chat"
          // Chat's own tabs are not movable — they take props no generic host
          // can supply — but Chat can still host a tab moved in from elsewhere.
          hostProps={{ workspacePath: workspacePath ?? null, provider: defaultProvider }}
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
                workspacePath={workspacePath ?? null}
                onPendingWrite={onPendingWrite}
                onSwitchToGoals={onSwitchToGoals}
                onOpenFile={onOpenFile}
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
