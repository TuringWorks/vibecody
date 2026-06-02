import { useState } from "react";
import { PanelLeftOpen, PanelRightOpen } from "lucide-react";
import { ProjectNavRail } from "../components/ProjectNavRail";
import { SessionStream } from "../components/SessionStream";
import { EnvironmentInspector } from "../components/EnvironmentInspector";
import { ReviewView } from "../components/ReviewView";
import { FilesView } from "../components/FilesView";
import type { QuickAction } from "../components/QuickActionDrawer";
import type { useTasks } from "../hooks/useTasks";

type TasksApi = ReturnType<typeof useTasks>;
type Overlay = null | "review" | "files";

interface ShellLayoutProps {
  daemonUrl: string;
  daemonOnline: boolean;
  tasks: TasksApi;
}

/**
 * VX-101 — the Codex-faithful three-column shell:
 *   left ProjectNavRail · center SessionStream · right EnvironmentInspector.
 * Side rails collapse to a thin strip with an expand button (so collapse is
 * reversible). No persistent editor pane — code is summoned via the
 * Review/Files quick-actions, which open as a center overlay.
 */
export function ShellLayout({ daemonUrl, daemonOnline, tasks }: ShellLayoutProps) {
  const [navCollapsed, setNavCollapsed] = useState(false);
  const [envCollapsed, setEnvCollapsed] = useState(false);
  const [overlay, setOverlay] = useState<Overlay>(null);
  // Bumped when a run finishes so the Environment inspector refetches git status.
  const [envRefresh, setEnvRefresh] = useState(0);
  // Remounting SessionStream on a fresh nonce is how "New chat" resets the
  // conversation (the live stream state lives inside SessionStream).
  const [chatNonce, setChatNonce] = useState(0);
  // The active project the next task is scoped to (set by "New project" /
  // selecting a project). null → daemon workspace_root default.
  const [activeProject, setActiveProject] = useState<string | null>(null);
  // The currently selected chat/task id (visual highlight in the nav).
  const [activeChatId, setActiveChatId] = useState<string | null>(null);

  function handleQuickAction(action: QuickAction) {
    if (action === "review") setOverlay("review");
    else if (action === "files") setOverlay("files");
    // side-chat / browser / terminal: Phase 3.
  }

  function newChat() {
    setOverlay(null);
    setActiveChatId(null);
    setChatNonce((n) => n + 1);
  }

  return (
    <div
      className="vibex-shell"
      data-nav={navCollapsed ? "collapsed" : "expanded"}
      data-env={envCollapsed ? "collapsed" : "expanded"}
    >
      <div className="vibex-col vibex-col--nav">
        {navCollapsed ? (
          <div className="vx-rail vx-rail--left">
            <button
              className="vx-icon-btn"
              title="Expand navigation"
              aria-label="Expand navigation"
              onClick={() => setNavCollapsed(false)}
            >
              <PanelLeftOpen size={16} />
            </button>
          </div>
        ) : (
          <ProjectNavRail
            daemonUrl={daemonUrl}
            daemonOnline={daemonOnline}
            tasks={tasks.tasks}
            activeChatId={activeChatId}
            activeProject={activeProject}
            onNewChat={newChat}
            onNewProject={(path) => {
              setActiveProject(path);
              newChat();
            }}
            onSelectProject={(path) => setActiveProject(path)}
            onSelectChat={(id) => setActiveChatId(id)}
            onToggle={() => setNavCollapsed(true)}
          />
        )}
      </div>

      <div className="vibex-col vibex-col--stream">
        {overlay === "review" ? (
          <ReviewView daemonUrl={daemonUrl} onClose={() => setOverlay(null)} />
        ) : overlay === "files" ? (
          <FilesView daemonUrl={daemonUrl} onClose={() => setOverlay(null)} />
        ) : (
          <SessionStream
            key={chatNonce}
            daemonUrl={daemonUrl}
            daemonOnline={daemonOnline}
            projectPath={activeProject}
            createTask={tasks.createTask}
            linkSession={tasks.linkSession}
            onQuickAction={handleQuickAction}
            onRunFinished={() => setEnvRefresh((n) => n + 1)}
          />
        )}
      </div>

      <div className="vibex-col vibex-col--env">
        {envCollapsed ? (
          <div className="vx-rail vx-rail--right">
            <button
              className="vx-icon-btn"
              title="Expand environment"
              aria-label="Expand environment"
              onClick={() => setEnvCollapsed(false)}
            >
              <PanelRightOpen size={16} />
            </button>
          </div>
        ) : (
          <EnvironmentInspector
            daemonUrl={daemonUrl}
            daemonOnline={daemonOnline}
            refreshKey={envRefresh}
            onOpenReview={() => setOverlay("review")}
            onToggle={() => setEnvCollapsed(true)}
          />
        )}
      </div>
    </div>
  );
}
