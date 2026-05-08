import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// -- Types --------------------------------------------------------------------

type TabName = "Sessions" | "Replay" | "Stats";

interface SessionEntry {
  id: string;
  timestamp: number;
  message_count: number;
  file_size: number;
  has_messages: boolean;
  has_context: boolean;
}

interface SessionMessage {
  role: string;
  content: string;
}

// -- Helpers ------------------------------------------------------------------

const formatTimestamp = (ts: number): string => {
  if (ts === 0) return "Unknown";
  const d = new Date(ts * 1000);
  return d.toLocaleString();
};

const formatFileSize = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};

const roleColor = (r: string): string => {
  switch (r) {
    case "user":
      return "var(--accent-color)";
    case "assistant":
      return "var(--success-color)";
    case "system":
      return "var(--warning-color)";
    default:
      return "var(--text-secondary)";
  }
};

// -- Component ----------------------------------------------------------------

const SessionBrowserPanel: React.FC = () => {
  const [tab, setTab] = useState<TabName>("Sessions");
  const [search, setSearch] = useState("");
  const [workspace, setWorkspace] = useState(() => {
    // Default to current working directory or home
    return ".";
  });

  // Sessions list state
  const [sessions, setSessions] = useState<SessionEntry[]>([]);
  const [sessionsLoading, setSessionsLoading] = useState(false);
  const [sessionsError, setSessionsError] = useState<string | null>(null);

  // Session detail / replay state
  const [selectedSession, setSelectedSession] = useState<SessionEntry | null>(null);
  const [messages, setMessages] = useState<SessionMessage[]>([]);
  const [messagesLoading, setMessagesLoading] = useState(false);
  const [messagesError, setMessagesError] = useState<string | null>(null);
  const [replayIndex, setReplayIndex] = useState(0);

  // Status banner — `kind` controls aria-live (errors are role="alert",
  // info is role="status" so AT users hear destructive failures
  // immediately and routine confirmations get a polite announcement).
  const [status, setStatus] = useState<{ message: string; kind: "info" | "error" } | null>(null);

  // Pending delete — second-click confirmation. We don't pop a modal
  // because the panel is already focused and the row is right there;
  // a single re-click within 5s is enough confirmation, the button
  // copy switches to "Confirm delete?" so the intent is unambiguous.
  const [pendingDelete, setPendingDelete] = useState<string | null>(null);

  // -- Data fetching ----------------------------------------------------------

  const loadSessions = useCallback(async () => {
    setSessionsLoading(true);
    setSessionsError(null);
    try {
      const result = await invoke<SessionEntry[]>("list_sessions", { workspace });
      setSessions(result);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setSessionsError(msg);
    } finally {
      setSessionsLoading(false);
    }
  }, [workspace]);

  const loadSessionDetail = useCallback(
    async (sessionId: string) => {
      setMessagesLoading(true);
      setMessagesError(null);
      try {
        const result = await invoke<SessionMessage[]>("get_session_detail", {
          workspace,
          sessionId,
        });
        setMessages(result);
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        setMessagesError(msg);
      } finally {
        setMessagesLoading(false);
      }
    },
    [workspace],
  );

  const performDelete = useCallback(
    async (sessionId: string) => {
      try {
        await invoke("delete_session", { workspace, sessionId });
        setSessions((prev) => prev.filter((s) => s.id !== sessionId));
        if (selectedSession?.id === sessionId) {
          setSelectedSession(null);
          setMessages([]);
          setTab("Sessions");
        }
        setStatus({ message: "Session deleted", kind: "info" });
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        setStatus({ message: `Delete failed: ${msg}`, kind: "error" });
      } finally {
        setPendingDelete(null);
      }
    },
    [workspace, selectedSession],
  );

  // First click arms the delete; second click within 5s commits. Click
  // anywhere else cancels. This is the same pattern Settings → Memory uses
  // for "Disable encryption" and is preferred over a modal in compact panels.
  const requestDelete = useCallback(
    (sessionId: string) => {
      if (pendingDelete === sessionId) {
        performDelete(sessionId);
        return;
      }
      setPendingDelete(sessionId);
      setTimeout(() => {
        setPendingDelete(prev => (prev === sessionId ? null : prev));
      }, 5000);
    },
    [pendingDelete, performDelete],
  );

  const forkSession = useCallback(
    async (sessionId: string) => {
      try {
        const newId = await invoke<string>("fork_session", { workspace, sessionId });
        setStatus({ message: `Forked → ${newId}`, kind: "info" });
        await loadSessions();
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        setStatus({ message: `Fork failed: ${msg}`, kind: "error" });
      }
    },
    [workspace, loadSessions],
  );

  // Load sessions on mount and when workspace changes
  useEffect(() => {
    loadSessions();
  }, [loadSessions]);

  // Cancel pending delete on Escape — backup escape hatch alongside the
  // 5s auto-cancel and the click-elsewhere implicit cancel.
  useEffect(() => {
    if (!pendingDelete) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") setPendingDelete(null);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [pendingDelete]);

  // -- Derived data -----------------------------------------------------------

  const filteredSessions = sessions.filter(
    (s) =>
      s.id.toLowerCase().includes(search.toLowerCase()),
  );

  const totalMessages = sessions.reduce((sum, s) => sum + s.message_count, 0);
  const totalSize = sessions.reduce((sum, s) => sum + s.file_size, 0);

  const tabs: TabName[] = ["Sessions", "Replay", "Stats"];

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h3>Session Browser</h3>
        <input
          value={workspace}
          onChange={(e) => setWorkspace(e.target.value)}
          placeholder="Workspace path..."
          className="panel-input"
          style={{ flex: 1 }}
        />
        <button onClick={loadSessions} className="panel-btn panel-btn-primary">
          Refresh
        </button>
      </div>
      <div className="panel-body">

      {/* Tab bar */}
      <div className="panel-tab-bar" style={{ marginBottom: 12 }}>
        {tabs.map((t) => (
          <button key={t} className={`panel-tab ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>
            {t}
          </button>
        ))}
      </div>

      {status && (
        <div
          role={status.kind === "error" ? "alert" : "status"}
          aria-live={status.kind === "error" ? "assertive" : "polite"}
          style={{
            padding: "8px 12px",
            marginBottom: 10,
            borderRadius: "var(--radius-xs-plus)",
            background: status.kind === "error" ? "var(--error-bg, #5b1a1a)" : "var(--bg-tertiary)",
            color: "var(--text-primary)",
            fontSize: "var(--font-size-base)",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            border: status.kind === "error" ? "1px solid var(--accent-rose, #ef4444)" : "none",
          }}
        >
          <span>{status.message}</span>
          <button
            onClick={() => setStatus(null)}
            aria-label="Dismiss status"
            style={{
              background: "none",
              border: "none",
              color: "var(--text-secondary)",
              cursor: "pointer",
              fontSize: "var(--font-size-lg)",
            }}
          >
            ✕
          </button>
        </div>
      )}

      {/* Sessions Tab */}
      {tab === "Sessions" && (
        <div>
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search sessions by ID..."
            className="panel-input panel-input-full"
            style={{ marginBottom: 10 }}
          />

          {sessionsLoading && (
            <div className="panel-loading">Loading sessions...</div>
          )}

          {sessionsError && (
            <div className="panel-error"><span>Error: {sessionsError}</span></div>
          )}

          {!sessionsLoading &&
            !sessionsError &&
            filteredSessions.map((s) => (
              <div
                key={s.id}
                style={{
                  padding: "8px 12px",
                  marginBottom: 6,
                  borderRadius: "var(--radius-xs-plus)",
                  background: "var(--bg-secondary)",
                  cursor: "pointer",
                  borderLeft: "3px solid var(--accent-color)",
                }}
              >
                <div
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "center",
                  }}
                >
                  <span
                    style={{ fontWeight: 600, fontSize: "var(--font-size-base)", cursor: "pointer" }}
                    onClick={() => {
                      setSelectedSession(s);
                      setReplayIndex(0);
                      setMessages([]);
                      loadSessionDetail(s.id);
                      setTab("Replay");
                    }}
                  >
                    {s.id}
                  </span>
                  <button
                    className="panel-btn panel-btn-secondary panel-btn-xs"
                    onClick={(e) => {
                      e.stopPropagation();
                      forkSession(s.id);
                    }}
                    aria-label={`Fork session ${s.id}`}
                    title="Fork session"
                  >
                    Fork
                  </button>
                  <button
                    className="panel-btn panel-btn-danger panel-btn-xs"
                    onClick={(e) => {
                      e.stopPropagation();
                      requestDelete(s.id);
                    }}
                    aria-label={
                      pendingDelete === s.id
                        ? `Confirm delete session ${s.id} — second click commits`
                        : `Delete session ${s.id} (requires second click to confirm)`
                    }
                    title={pendingDelete === s.id ? "Click again to confirm" : "Delete session"}
                  >
                    {pendingDelete === s.id ? "Confirm?" : "Delete"}
                  </button>
                </div>
                <div role="button" tabIndex={0}
                  aria-label={`Replay session ${s.id} — ${s.message_count} messages, ${formatFileSize(s.file_size)}`}
                  style={{
                    display: "flex",
                    gap: 12,
                    marginTop: 4,
                    fontSize: "var(--font-size-sm)",
                    color: "var(--text-secondary)",
                  }}
                  onClick={() => {
                    setSelectedSession(s);
                    setReplayIndex(0);
                    setMessages([]);
                    loadSessionDetail(s.id);
                    setTab("Replay");
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      setSelectedSession(s);
                      setReplayIndex(0);
                      setMessages([]);
                      loadSessionDetail(s.id);
                      setTab("Replay");
                    }
                  }}
                >
                  <span>{s.message_count} msgs</span>
                  <span>{formatFileSize(s.file_size)}</span>
                  {s.has_messages && (
                    <span style={{ color: "var(--success-color)" }}>messages</span>
                  )}
                  {s.has_context && (
                    <span style={{ color: "var(--warning-color)" }}>context</span>
                  )}
                  <span style={{ marginLeft: "auto" }}>
                    {formatTimestamp(s.timestamp)}
                  </span>
                </div>
              </div>
            ))}

          {!sessionsLoading &&
            !sessionsError &&
            filteredSessions.length === 0 && (
              <div className="panel-empty">
                {sessions.length === 0
                  ? "No sessions found in .vibecli/traces/"
                  : "No sessions match your search."}
              </div>
            )}
        </div>
      )}

      {/* Replay Tab */}
      {tab === "Replay" && (
        <div>
          {selectedSession ? (
            <>
              <div
                style={{
                  marginBottom: 10,
                  fontSize: "var(--font-size-base)",
                  color: "var(--text-secondary)",
                }}
              >
                Replaying:{" "}
                <strong style={{ color: "var(--text-primary)" }}>
                  {selectedSession.id}
                </strong>{" "}
                ({selectedSession.message_count} messages,{" "}
                {formatFileSize(selectedSession.file_size)})
              </div>

              {messagesLoading && (
                <div className="panel-loading">Loading messages...</div>
              )}

              {messagesError && (
                <div className="panel-error"><span>Error: {messagesError}</span></div>
              )}

              {!messagesLoading && !messagesError && messages.length > 0 && (
                <>
                  <div style={{ display: "flex", gap: 8, marginBottom: 12, alignItems: "center" }}>
                    <button
                      className="panel-btn panel-btn-secondary panel-btn-xs"
                      onClick={() =>
                        setReplayIndex(Math.max(0, replayIndex - 1))
                      }
                      disabled={replayIndex === 0}
                    >
                      Prev
                    </button>
                    <button
                      className="panel-btn panel-btn-secondary panel-btn-xs"
                      onClick={() =>
                        setReplayIndex(
                          Math.min(messages.length - 1, replayIndex + 1),
                        )
                      }
                      disabled={replayIndex >= messages.length - 1}
                    >
                      Next
                    </button>
                    <span
                      style={{
                        fontSize: "var(--font-size-sm)",
                        color: "var(--text-secondary)",
                      }}
                    >
                      Step {replayIndex + 1} / {messages.length}
                    </span>
                  </div>
                  {messages.slice(0, replayIndex + 1).map((msg, i) => (
                    <div
                      key={i}
                      style={{
                        padding: "8px 12px",
                        marginBottom: 6,
                        borderRadius: "var(--radius-xs-plus)",
                        background:
                          i === replayIndex
                            ? "var(--bg-tertiary)"
                            : "var(--bg-secondary)",
                        borderLeft: `3px solid ${roleColor(msg.role)}`,
                      }}
                    >
                      <div
                        style={{
                          display: "flex",
                          justifyContent: "space-between",
                          fontSize: "var(--font-size-sm)",
                          marginBottom: 4,
                        }}
                      >
                        <span
                          style={{
                            fontWeight: 600,
                            color: roleColor(msg.role),
                            textTransform: "capitalize",
                          }}
                        >
                          {msg.role}
                        </span>
                      </div>
                      <div
                        style={{
                          fontSize: "var(--font-size-base)",
                          whiteSpace: "pre-wrap",
                          lineHeight: 1.5,
                          maxHeight: 200,
                          overflowY: "auto",
                        }}
                      >
                        {msg.content}
                      </div>
                    </div>
                  ))}
                </>
              )}

              {!messagesLoading &&
                !messagesError &&
                messages.length === 0 && (
                  <div className="panel-empty">No messages found for this session.</div>
                )}
            </>
          ) : (
            <div className="panel-empty">Select a session from the Sessions tab to replay it.</div>
          )}
        </div>
      )}

      {/* Stats Tab */}
      {tab === "Stats" && (
        <div>
          <div className="panel-stats-grid-3" style={{ marginBottom: 16 }}>
            {[
              { label: "Total Sessions", value: String(sessions.length) },
              { label: "Total Messages", value: String(totalMessages) },
              { label: "Total Size", value: formatFileSize(totalSize) },
            ].map(({ label, value }) => (
              <div key={label} className="panel-stat">
                <div className="panel-stat-value">{value}</div>
                <div className="panel-stat-label">{label}</div>
              </div>
            ))}
          </div>

          <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, marginBottom: 8 }}>
            Sessions by Size
          </div>
          {sessions.length === 0 && (
            <div className="panel-empty">No sessions to display.</div>
          )}
          {[...sessions]
            .sort((a, b) => b.file_size - a.file_size)
            .slice(0, 10)
            .map((s) => {
              const maxSize = Math.max(...sessions.map((x) => x.file_size), 1);
              const pct = Math.round((s.file_size / maxSize) * 100);
              return (
                <div
                  key={s.id}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 8,
                    marginBottom: 6,
                  }}
                >
                  <span
                    style={{
                      minWidth: 100,
                      fontSize: "var(--font-size-sm)",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                    title={s.id}
                  >
                    {s.id}
                  </span>
                  <div className="progress-bar" style={{ flex: 1 }}>
                    <div className="progress-bar-fill progress-bar-accent" style={{ width: `${pct}%` }} />
                  </div>
                  <span
                    style={{
                      minWidth: 50,
                      textAlign: "right",
                      fontSize: "var(--font-size-sm)",
                      color: "var(--text-secondary)",
                    }}
                  >
                    {formatFileSize(s.file_size)}
                  </span>
                  <span
                    style={{
                      minWidth: 40,
                      textAlign: "right",
                      fontSize: "var(--font-size-sm)",
                      color: "var(--text-secondary)",
                    }}
                  >
                    {s.message_count} msgs
                  </span>
                </div>
              );
            })}
        </div>
      )}
      </div>
    </div>
  );
};

export default SessionBrowserPanel;
