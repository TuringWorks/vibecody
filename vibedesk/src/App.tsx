import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ShellLayout } from "./layouts/ShellLayout";
import { useDaemon } from "./hooks/useDaemon";
import { useTasks } from "./hooks/useTasks";
import { useTheme } from "./hooks/useTheme";

/**
 * VibeDesk root. Renders the daemon status banner (zero-config-first: connection
 * state is always visible and plain) above the three-column Codex-style shell.
 * The daemon is autostarted on launch (src-tauri); the banner offers a manual
 * "Start daemon" retry if that didn't take (e.g. `vibecli` not on PATH).
 */
export function App() {
  const daemon = useDaemon();
  const tasks = useTasks(daemon.url, daemon.status === "online");
  const [starting, setStarting] = useState(false);
  // Apply the persisted theme app-wide (data-theme on <html>) on boot.
  useTheme();

  async function startDaemon() {
    setStarting(true);
    try {
      await invoke<string>("start_daemon", {});
    } catch (e) {
      console.error("start daemon failed", e);
    } finally {
      setStarting(false);
      daemon.recheck();
    }
  }

  return (
    <div className="vibedesk-root">
      <div className="vibedesk-banner">
        <span className={`vibedesk-banner__dot vibedesk-banner__dot--${daemon.status}`} />
        <span>
          {daemon.status === "online" && `VibeDesk · daemon online (${daemon.url})`}
          {daemon.status === "checking" && "VibeDesk · starting daemon…"}
          {daemon.status === "offline" &&
            `VibeDesk · daemon offline (${daemon.error ?? daemon.url})`}
        </span>
        {daemon.status === "offline" && (
          <button className="vibedesk-banner__btn" onClick={startDaemon} disabled={starting}>
            {starting ? "Starting…" : "Start daemon"}
          </button>
        )}
      </div>
      <ShellLayout
        daemonUrl={daemon.url}
        daemonOnline={daemon.status === "online"}
        tasks={tasks}
      />
    </div>
  );
}
