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

  // Status banner
  const [status, setStatus] = useState<string | null>(null);

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

  const deleteSession = useCallback(
    async (sessionId: string) => {
      try {
        await invoke("delete_session", { workspace, sessionId });
        setSessions((prev) => prev.filter((s) => s.id !== sessionId));
        if (selectedSession?.id === sessionId) {
          setSelectedSession(null);
          setMessages([]);
          setTab("Sessions");
        }
        setStatus("Session deleted");
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        setStatus(`Delete failed: ${msg}`);
      }
    },
    [workspace, selectedSession],
  );

  const forkSession = useCallback(
    async (sessionId: string) => {
      try {
        const newId = await invoke<string>("fork_session", { workspace, sessionId });
        setStatus(`Forked → ${newId}`);
        await loadSessions();
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err);
        setStatus(`Fork failed: ${msg}`);
      }
    },
    [workspace, loadSessions],
  );

  // Load sessions on mount and when workspace changes
  useEffect(() => {
    loadSessions();
  }, [loadSessions]);

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
          style={{
            padding: "8px 12px",
            marginBottom: 10,
            borderRadius: "var(--radius-xs-plus)",
            background: "var(--bg-tertiary)",
            color: "var(--text-primary)",
            fontSize: "var(--font-size-base)",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <span>{status}</span>
          <button
            onClick={() => setStatus(null)}
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
            <div
              style={{
                textAlign: "center",
                padding: 30,
                color: "var(--text-secondary)",
              }}
            >
              Loading sessions...
            </div>
          )}

          {sessionsError && (
            <div
              style={{
                padding: "8px 12px",
                marginBottom: 10,
                borderRadius: "var(--radius-xs-plus)",
                background: "var(--bg-secondary)",
                borderLeft: "3px solid var(--error-color)",
                color: "var(--error-color)",
                fontSize: "var(--font-size-base)",
              }}
            >
              Error: {sessionsError}
            </div>
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
                    onClick={(e) => {
                      e.stopPropagation();
                      forkSession(s.id);
                    }}
                    aria-label={`Fork session ${s.id}`}
                    title="Fork session"
                    style={{
                      background: "none",
                      border: "none",
                      color: "var(--accent-color)",
                      cursor: "pointer",
                      fontSize: "var(--font-size-base)",
                      padding: "2px 8px",
                    }}
                  >
                    Fork
                  </button>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      deleteSession(s.id);
                    }}
                    aria-label={`Delete session ${s.id}`}
                    title="Delete session"
                    style={{
                      background: "none",
                      border: "none",
                      color: "var(--error-color)",
                      cursor: "pointer",
                      fontSize: "var(--font-size-base)",
                      padding: "2px 8px",
                    }}
                  >
                    Delete
                  </button>
                </div>
                <div role="button" tabIndex={0}
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
              <div
                className="panel-empty"
                style={{
                  textAlign: "center",
                  padding: 30,
                  color: "var(--text-secondary)",
                }}
              >
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
                <div
                  style={{
                    textAlign: "center",
                    padding: 30,
                    color: "var(--text-secondary)",
                  }}
                >
                  Loading messages...
                </div>
              )}

              {messagesError && (
                <div
                  style={{
                    padding: "8px 12px",
                    marginBottom: 10,
                    borderRadius: "var(--radius-xs-plus)",
                    background: "var(--bg-secondary)",
                    borderLeft: "3px solid var(--error-color)",
                    color: "var(--error-color)",
                    fontSize: "var(--font-size-base)",
                  }}
                >
                  Error: {messagesError}
                </div>
              )}

              {!messagesLoading && !messagesError && messages.length > 0 && (
                <>
                  <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
                    <button
                      onClick={() =>
                        setReplayIndex(Math.max(0, replayIndex - 1))
                      }
                      disabled={replayIndex === 0}
                      style={{
                        padding: "4px 12px",
                        fontSize: "var(--font-size-sm)",
                        borderRadius: "var(--radius-xs-plus)",
                        border: "1px solid var(--border-color)",
                        background: "none",
                        color: "var(--text-primary)",
                        cursor: replayIndex === 0 ? "not-allowed" : "pointer",
                      }}
                    >
                      Prev
                    </button>
                    <button
                      onClick={() =>
                        setReplayIndex(
                          Math.min(messages.length - 1, replayIndex + 1),
                        )
                      }
                      disabled={replayIndex >= messages.length - 1}
                      style={{
                        padding: "4px 12px",
                        fontSize: "var(--font-size-sm)",
                        borderRadius: "var(--radius-xs-plus)",
                        border: "1px solid var(--border-color)",
                        background: "none",
                        color: "var(--text-primary)",
                        cursor:
                          replayIndex >= messages.length - 1
                            ? "not-allowed"
                            : "pointer",
                      }}
                    >
                      Next
                    </button>
                    <span
                      style={{
                        fontSize: "var(--font-size-sm)",
                        color: "var(--text-secondary)",
                        lineHeight: "28px",
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
                  <div className="panel-empty"
                    style={{
                      textAlign: "center",
                      padding: 30,
                      color: "var(--text-secondary)",
                    }}
                  >
                    No messages found for this session.
                  </div>
                )}
            </>
          ) : (
            <div
              style={{
                textAlign: "center",
                padding: 30,
                color: "var(--text-secondary)",
              }}
            >
              Select a session from the Sessions tab to replay it.
            </div>
          )}
        </div>
      )}

      {/* Stats Tab */}
      {tab === "Stats" && (
        <div>
          <div
            style={{
              display: "flex",
              gap: 12,
              flexWrap: "wrap",
              marginBottom: 16,
            }}
          >
            {[
              { label: "Total Sessions", value: String(sessions.length) },
              { label: "Total Messages", value: String(totalMessages) },
              { label: "Total Size", value: formatFileSize(totalSize) },
            ].map(({ label, value }) => (
              <div
                key={label}
                style={{
                  background: "var(--bg-secondary)",
                  padding: "12px 16px",
                  borderRadius: "var(--radius-sm)",
                  textAlign: "center",
                  minWidth: 90,
                }}
              >
                <div
                  style={{
                    fontSize: 20,
                    fontWeight: 700,
                    color: "var(--accent-color)",
                  }}
                >
                  {value}
                </div>
                <div
                  style={{
                    fontSize: "var(--font-size-sm)",
                    color: "var(--text-secondary)",
                    marginTop: 2,
                  }}
                >
                  {label}
                </div>
              </div>
            ))}
          </div>

          <div style={{ fontSize: "var(--font-size-base)", fontWeight: 600, marginBottom: 8 }}>
            Sessions by Size
          </div>
          {sessions.length === 0 && (
            <div
              className="panel-empty"
              style={{
                textAlign: "center",
                padding: 20,
                color: "var(--text-secondary)",
              }}
            >
              No sessions to display.
            </div>
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
                  <div
                    style={{
                      flex: 1,
                      background: "var(--bg-secondary)",
                      borderRadius: 3,
                      height: 10,
                      overflow: "hidden",
                    }}
                  >
                    <div
                      style={{
                        width: `${pct}%`,
                        height: "100%",
                        background: "var(--accent-color)",
                        borderRadius: 3,
                      }}
                    />
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
