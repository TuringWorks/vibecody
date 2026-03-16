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
    <div
      style={{
        padding: 12,
        fontFamily: "var(--font-family, sans-serif)",
        fontSize: 13,
        height: "100%",
        overflowY: "auto",
        color: "var(--text-primary)",
        background: "var(--bg-primary)",
      }}
    >
      <div style={{ fontWeight: 600, fontSize: 14, marginBottom: 12 }}>
        Session Browser
      </div>

      {/* Workspace input */}
      <div style={{ display: "flex", gap: 8, marginBottom: 10 }}>
        <input
          value={workspace}
          onChange={(e) => setWorkspace(e.target.value)}
          placeholder="Workspace path..."
          style={{
            flex: 1,
            padding: "6px 10px",
            fontSize: 12,
            background: "var(--bg-secondary)",
            color: "var(--text-primary)",
            border: "1px solid var(--border-color)",
            borderRadius: 4,
            boxSizing: "border-box",
          }}
        />
        <button
          onClick={loadSessions}
          style={{
            padding: "6px 12px",
            fontSize: 12,
            borderRadius: 4,
            border: "none",
            background: "var(--accent-color)",
            color: "white",
            cursor: "pointer",
          }}
        >
          Refresh
        </button>
      </div>

      {/* Tab bar */}
      <div
        style={{
          display: "flex",
          gap: 0,
          marginBottom: 12,
          borderBottom: "1px solid var(--border-color)",
        }}
      >
        {tabs.map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            style={{
              padding: "6px 16px",
              fontSize: 12,
              background: "none",
              border: "none",
              borderBottom:
                tab === t
                  ? "2px solid var(--accent-color)"
                  : "2px solid transparent",
              color: tab === t ? "var(--text-primary)" : "var(--text-muted)",
              cursor: "pointer",
              fontWeight: tab === t ? 600 : 400,
            }}
          >
            {t}
          </button>
        ))}
      </div>

      {status && (
        <div
          style={{
            padding: "6px 10px",
            marginBottom: 10,
            borderRadius: 4,
            background: "var(--bg-tertiary)",
            color: "var(--text-primary)",
            fontSize: 12,
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
              color: "var(--text-muted)",
              cursor: "pointer",
              fontSize: 14,
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
            style={{
              width: "100%",
              padding: "6px 10px",
              fontSize: 12,
              background: "var(--bg-secondary)",
              color: "var(--text-primary)",
              border: "1px solid var(--border-color)",
              borderRadius: 4,
              marginBottom: 10,
              boxSizing: "border-box",
            }}
          />

          {sessionsLoading && (
            <div
              style={{
                textAlign: "center",
                padding: 30,
                color: "var(--text-muted)",
              }}
            >
              Loading sessions...
            </div>
          )}

          {sessionsError && (
            <div
              style={{
                padding: "8px 10px",
                marginBottom: 10,
                borderRadius: 4,
                background: "var(--bg-secondary)",
                borderLeft: "3px solid var(--error-color)",
                color: "var(--error-color)",
                fontSize: 12,
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
                  padding: "8px 10px",
                  marginBottom: 6,
                  borderRadius: 4,
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
                    style={{ fontWeight: 600, fontSize: 12, cursor: "pointer" }}
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
                      deleteSession(s.id);
                    }}
                    title="Delete session"
                    style={{
                      background: "none",
                      border: "none",
                      color: "var(--error-color)",
                      cursor: "pointer",
                      fontSize: 12,
                      padding: "2px 6px",
                    }}
                  >
                    Delete
                  </button>
                </div>
                <div
                  style={{
                    display: "flex",
                    gap: 12,
                    marginTop: 4,
                    fontSize: 11,
                    color: "var(--text-muted)",
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
                style={{
                  textAlign: "center",
                  padding: 30,
                  color: "var(--text-muted)",
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
                  fontSize: 12,
                  color: "var(--text-muted)",
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
                    color: "var(--text-muted)",
                  }}
                >
                  Loading messages...
                </div>
              )}

              {messagesError && (
                <div
                  style={{
                    padding: "8px 10px",
                    marginBottom: 10,
                    borderRadius: 4,
                    background: "var(--bg-secondary)",
                    borderLeft: "3px solid var(--error-color)",
                    color: "var(--error-color)",
                    fontSize: 12,
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
                        fontSize: 11,
                        borderRadius: 4,
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
                        fontSize: 11,
                        borderRadius: 4,
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
                        fontSize: 11,
                        color: "var(--text-muted)",
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
                        padding: "8px 10px",
                        marginBottom: 6,
                        borderRadius: 4,
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
                          fontSize: 11,
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
                          fontSize: 12,
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
                  <div
                    style={{
                      textAlign: "center",
                      padding: 30,
                      color: "var(--text-muted)",
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
                color: "var(--text-muted)",
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
                  padding: "10px 16px",
                  borderRadius: 6,
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
                    fontSize: 11,
                    color: "var(--text-muted)",
                    marginTop: 2,
                  }}
                >
                  {label}
                </div>
              </div>
            ))}
          </div>

          <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 8 }}>
            Sessions by Size
          </div>
          {sessions.length === 0 && (
            <div
              style={{
                textAlign: "center",
                padding: 20,
                color: "var(--text-muted)",
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
                      fontSize: 11,
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
                      fontSize: 11,
                      color: "var(--text-muted)",
                    }}
                  >
                    {formatFileSize(s.file_size)}
                  </span>
                  <span
                    style={{
                      minWidth: 40,
                      textAlign: "right",
                      fontSize: 11,
                      color: "var(--text-muted)",
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
  );
};

export default SessionBrowserPanel;
