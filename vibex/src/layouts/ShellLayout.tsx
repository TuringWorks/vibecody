import { useState } from "react";
import { ProjectNavRail } from "../components/ProjectNavRail";
import { SessionStream } from "../components/SessionStream";
import { EnvironmentInspector } from "../components/EnvironmentInspector";

interface ShellLayoutProps {
  daemonUrl: string;
  daemonOnline: boolean;
}

/**
 * VX-101 — the Codex-faithful three-column shell:
 *   left ProjectNavRail · center SessionStream · right EnvironmentInspector.
 * Side rails are collapsible. No persistent editor pane (Codex hands off to
 * external editors; code is summoned via the Review/Files quick-actions).
 */
export function ShellLayout({ daemonUrl, daemonOnline }: ShellLayoutProps) {
  const [navCollapsed, setNavCollapsed] = useState(false);
  const [envCollapsed, setEnvCollapsed] = useState(false);

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
          onToggle={() => setNavCollapsed((v) => !v)}
        />
      </div>
      <div className="vibex-col vibex-col--stream">
        <SessionStream daemonUrl={daemonUrl} daemonOnline={daemonOnline} />
      </div>
      <div className="vibex-col vibex-col--env">
        <EnvironmentInspector
          daemonUrl={daemonUrl}
          daemonOnline={daemonOnline}
          onToggle={() => setEnvCollapsed((v) => !v)}
        />
      </div>
    </div>
  );
}
