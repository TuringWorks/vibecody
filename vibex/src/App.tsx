import { ShellLayout } from "./layouts/ShellLayout";
import { useDaemon } from "./hooks/useDaemon";
import { useTasks } from "./hooks/useTasks";

/**
 * VibeX root. Renders the daemon status banner (zero-config-first: connection
 * state is always visible and plain) above the three-column Codex-style shell.
 */
export function App() {
  const daemon = useDaemon();
  const tasks = useTasks(daemon.url, daemon.status === "online");

  return (
    <div className="vibex-root">
      <div className="vibex-banner">
        <span className={`vibex-banner__dot vibex-banner__dot--${daemon.status}`} />
        <span>
          {daemon.status === "online" && `VibeX · daemon online (${daemon.url})`}
          {daemon.status === "checking" && "VibeX · connecting to daemon…"}
          {daemon.status === "offline" &&
            `VibeX · daemon offline — run \`vibecli serve\` (${daemon.error ?? daemon.url})`}
        </span>
      </div>
      <ShellLayout
        daemonUrl={daemon.url}
        daemonOnline={daemon.status === "online"}
        tasks={tasks}
      />
    </div>
  );
}
