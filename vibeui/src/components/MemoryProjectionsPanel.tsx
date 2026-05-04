/**
 * MemoryProjectionsPanel — read-only view of MEMORY.md (project tier) +
 * USER.md (user tier).
 *
 * Both files are generated projections of the OpenMemory store. Invokes
 * `memory_projections_refresh` which regenerates both files on disk and
 * returns their bodies so we render the latest state without a second
 * round-trip. The on-disk files exist so users can open them in their
 * editor; this panel is the same view inside VibeUI.
 */
import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { MarkdownPreview } from "./MarkdownPreview";
import { MemoryErrorCard } from "./MemoryErrorCard";

interface MemoryProjectionsPanelProps {
  workspacePath?: string | null;
}

interface RefreshResponse {
  memory_md_path: string;
  memory_md_body: string;
  user_md_path: string | null;
  user_md_body: string | null;
}

type Tier = "project" | "user";

export function MemoryProjectionsPanel({
  workspacePath,
}: MemoryProjectionsPanelProps) {
  const [tier, setTier] = useState<Tier>("project");
  const [data, setData] = useState<RefreshResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!workspacePath) return;
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<RefreshResponse>(
        "memory_projections_refresh",
        { workspace: workspacePath },
      );
      setData(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [workspacePath]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  if (!workspacePath) {
    return (
      <div style={{ padding: 16, opacity: 0.6, textAlign: "center" }}>
        <p>Open a workspace folder to view memory projections.</p>
      </div>
    );
  }

  const activePath =
    tier === "project" ? data?.memory_md_path : (data?.user_md_path ?? null);
  const activeBody =
    tier === "project" ? data?.memory_md_body : (data?.user_md_body ?? null);
  const canShowUser = !!data?.user_md_path;

  return (
    <div className="panel-container">
      <div
        className="panel-header"
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "8px 12px",
          borderBottom: "1px solid var(--border-color)",
        }}
      >
        <div style={{ display: "flex", gap: 4 }}>
          <button
            onClick={() => setTier("project")}
            className={`panel-btn panel-btn-sm ${tier === "project" ? "panel-btn-primary" : "panel-btn-secondary"}`}
          >
            Project (MEMORY.md)
          </button>
          <button
            onClick={() => setTier("user")}
            disabled={!canShowUser}
            className={`panel-btn panel-btn-sm ${tier === "user" ? "panel-btn-primary" : "panel-btn-secondary"}`}
            title={
              canShowUser
                ? undefined
                : "USER.md unavailable (no home directory)"
            }
          >
            User (USER.md)
          </button>
        </div>
        <div style={{ flex: 1 }} />
        {activePath && (
          <button
            onClick={() => openUrl(`file://${activePath}`).catch(() => {})}
            className="panel-btn panel-btn-sm panel-btn-secondary"
            title="Open file in system viewer"
          >
            Open file
          </button>
        )}
        <button
          onClick={refresh}
          disabled={loading}
          className="panel-btn panel-btn-sm panel-btn-primary"
        >
          {loading ? "Refreshing…" : "Refresh"}
        </button>
      </div>

      <div
        className="panel-body"
        style={{
          overflowY: "auto",
          padding: 16,
          fontSize: "var(--font-size-base)",
          color: "var(--text-primary)",
        }}
      >
        <MemoryErrorCard error={error} />
        {!error && activeBody && <MarkdownPreview content={activeBody} />}
        {!error && !activeBody && !loading && (
          <div className="panel-empty" style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {tier === "user" && !canShowUser ? (
              <>
                <div>No home directory available — USER.md not generated.</div>
                <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
                  USER.md projections require a writable HOME directory. Set the HOME environment variable and restart the daemon.
                </div>
              </>
            ) : (
              <>
                <div>{tier === "user" ? "USER.md is empty." : "No project memory yet."}</div>
                <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
                  {tier === "user"
                    ? "Cross-project facts will appear here once agents extract them. To seed manually, add to ~/.vibecli/USER.md."
                    : "Project-scoped facts populate here automatically as agents work. To seed manually, add a memory via the OpenMemory panel or the /openmemory REPL command."}
                </div>
              </>
            )}
          </div>
        )}
        {activePath && (
          <div
            style={{
              marginTop: 16,
              paddingTop: 8,
              borderTop: "1px solid var(--border-color)",
              fontSize: "var(--font-size-sm)",
              opacity: 0.6,
              fontFamily: "var(--font-mono)",
              wordBreak: "break-all",
            }}
          >
            {activePath}
          </div>
        )}
      </div>
    </div>
  );
}

export default MemoryProjectionsPanel;
