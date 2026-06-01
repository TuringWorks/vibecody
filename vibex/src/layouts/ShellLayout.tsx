import { useState } from "react";
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
 * Side rails are collapsible. No persistent editor pane (Codex hands off to
 * external editors; code is summoned via the Review/Files quick-actions, which
 * open as a center overlay over the conversation).
 */
export function ShellLayout({ daemonUrl, daemonOnline, tasks }: ShellLayoutProps) {
  const [navCollapsed, setNavCollapsed] = useState(false);
  const [envCollapsed, setEnvCollapsed] = useState(false);
  const [overlay, setOverlay] = useState<Overlay>(null);
  // Bumped when a run finishes so the Environment inspector refetches git status.
  const [envRefresh, setEnvRefresh] = useState(0);

  function handleQuickAction(action: QuickAction) {
    if (action === "review") setOverlay("review");
    else if (action === "files") setOverlay("files");
    // side-chat / browser / terminal: Phase 3.
  }

  return (
    <div
      className="vibex-shell"
      data-nav={navCollapsed ? "collapsed" : "expanded"}
      data-env={envCollapsed ? "collapsed" : "expanded"}
    >
      <div className="vibex-col vibex-col--nav">
        <ProjectNavRail
          daemonUrl={daemonUrl}
          daemonOnline={daemonOnline}
          tasks={tasks.tasks}
          onToggle={() => setNavCollapsed((v) => !v)}
        />
      </div>
      <div className="vibex-col vibex-col--stream">
        {overlay === "review" ? (
          <ReviewView daemonUrl={daemonUrl} onClose={() => setOverlay(null)} />
        ) : overlay === "files" ? (
          <FilesView daemonUrl={daemonUrl} onClose={() => setOverlay(null)} />
        ) : (
          <SessionStream
            daemonUrl={daemonUrl}
            daemonOnline={daemonOnline}
            createTask={tasks.createTask}
            linkSession={tasks.linkSession}
            onQuickAction={handleQuickAction}
            onRunFinished={() => setEnvRefresh((n) => n + 1)}
          />
        )}
      </div>
      <div className="vibex-col vibex-col--env">
        <EnvironmentInspector
          daemonUrl={daemonUrl}
          daemonOnline={daemonOnline}
          refreshKey={envRefresh}
          onOpenReview={() => setOverlay("review")}
          onToggle={() => setEnvCollapsed((v) => !v)}
        />
      </div>
    </div>
  );
}
