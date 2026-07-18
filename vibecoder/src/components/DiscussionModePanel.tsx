import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ThumbsUp, ThumbsDown, CheckCircle, HelpCircle, Flame } from "lucide-react";

type MessageType = "Question" | "Suggestion" | "Concern" | "Decision" | "Action";

interface DiscussionMessage {
  id: string;
  author: string;
  type: MessageType;
  text: string;
  reactions: Record<string, number>;
  timestamp: string;
}

interface DiscussionThread {
  id: string;
  topic: string;
  messages: DiscussionMessage[];
  build_state: string;
  created_at: string;
}

const DiscussionModePanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("discussion");
  const [threads, setThreads] = useState<DiscussionThread[]>([]);
  const [activeThread, setActiveThread] = useState<DiscussionThread | null>(null);
  const [newTopic, setNewTopic] = useState("");
  const [newText, setNewText] = useState("");
  const [newType, setNewType] = useState<MessageType>("Question");
  const [buildState, setBuildState] = useState<"Building" | "Discussing" | "Paused">("Discussing");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadThreads = useCallback(async () => {
    try {
      const result = await invoke<DiscussionThread[]>("list_discussion_threads");
      setThreads(result);
      if (result.length > 0 && !activeThread) {
        setActiveThread(result[0]);
        setBuildState((result[0].build_state || "Discussing") as "Building" | "Discussing" | "Paused");
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [activeThread]);

  useEffect(() => { loadThreads(); }, [loadThreads]);

  const inputStyle: React.CSSProperties = {
    width: "100%", padding: "6px 10px", boxSizing: "border-box",
    backgroundColor: "var(--bg-secondary)", color: "var(--text-primary)",
    border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)",
  };

  const typeColors: Record<MessageType, string> = {
    Question: "#1565c0", Suggestion: "#2e7d32", Concern: "#e65100",
    Decision: "var(--accent-purple)", Action: "#c62828",
  };
  const badgeStyle = (color: string): React.CSSProperties => ({
    display: "inline-block", padding: "2px 8px", borderRadius: "var(--radius-md)",
    fontSize: "var(--font-size-sm)", fontWeight: 600, backgroundColor: color, color: "var(--text-primary)",
  });
  const reactionBtnStyle: React.CSSProperties = {
    padding: "2px 6px", border: "1px solid var(--border-color)",
    borderRadius: "12px", backgroundColor: "transparent", cursor: "pointer",
    fontSize: "var(--font-size-base)", color: "var(--text-primary)",
  };

  const messageTypes: MessageType[] = ["Question", "Suggestion", "Concern", "Decision", "Action"];
  const reactionKeys = ["thumbsup", "thumbsdown", "check", "thinking", "fire"];
  const reactionIconMap: Record<string, React.ReactNode> = {
    thumbsup: <ThumbsUp size={14} strokeWidth={1.5} />,
    thumbsdown: <ThumbsDown size={14} strokeWidth={1.5} />,
    check: <CheckCircle size={14} strokeWidth={1.5} />,
    thinking: <HelpCircle size={14} strokeWidth={1.5} />,
    fire: <Flame size={14} strokeWidth={1.5} />,
  };

  const handleCreateThread = async () => {
    if (!newTopic.trim()) return;
    try {
      const thread = await invoke<DiscussionThread>("create_discussion_thread", { topic: newTopic });
      setThreads(prev => [...prev, thread]);
      setActiveThread(thread);
      setBuildState("Discussing");
      setNewTopic("");
    } catch (e) {
      setError(String(e));
    }
  };

  const handleDeleteThread = async (threadId: string) => {
    try {
      await invoke("delete_discussion_thread", { threadId });
      setThreads(prev => prev.filter(t => t.id !== threadId));
      if (activeThread?.id === threadId) {
        setActiveThread(null);
      }
    } catch (e) {
      setError(String(e));
    }
  };

  const handleAddMessage = async () => {
    if (!newText.trim() || !activeThread) return;
    try {
      const msg = await invoke<DiscussionMessage>("add_discussion_message", {
        threadId: activeThread.id,
        author: "You",
        msgType: newType,
        text: newText,
      });
      // The backend returns the message with `type` field via serde rename
      const mapped: DiscussionMessage = {
        id: msg.id,
        author: msg.author,
        type: (msg as unknown as Record<string, unknown>).type as MessageType || newType,
        text: msg.text,
        reactions: msg.reactions || {},
        timestamp: msg.timestamp,
      };
      setActiveThread(prev => prev ? { ...prev, messages: [...prev.messages, mapped] } : null);
      setNewText("");
    } catch (e) {
      setError(String(e));
    }
  };

  const handleReaction = (msgId: string, emoji: string) => {
    if (!activeThread) return;
    setActiveThread(prev => {
      if (!prev) return null;
      return {
        ...prev,
        messages: prev.messages.map(m =>
          m.id === msgId ? { ...m, reactions: { ...m.reactions, [emoji]: (m.reactions[emoji] || 0) + 1 } } : m
        ),
      };
    });
  };

  const handleSelectThread = async (threadId: string) => {
    try {
      const thread = await invoke<DiscussionThread>("get_discussion_thread", { threadId });
      // Map msg_type -> type for each message
      const mappedThread: DiscussionThread = {
        ...thread,
        messages: thread.messages.map(m => ({
          ...m,
          type: ((m as unknown as Record<string, unknown>).type as MessageType) || "Question",
        })),
      };
      setActiveThread(mappedThread);
      setBuildState((mappedThread.build_state || "Discussing") as "Building" | "Discussing" | "Paused");
    } catch (e) {
      setError(String(e));
    }
  };

  const messages = activeThread?.messages || [];

  const renderMessageCard = (m: DiscussionMessage) => (
    <div key={m.id} className="panel-card">
      <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "8px" }}>
        <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
          <strong>{m.author}</strong>
          <span style={badgeStyle(typeColors[m.type] || "var(--text-secondary)")}>{m.type}</span>
        </div>
        <span style={{ fontSize: "var(--font-size-sm)", opacity: 0.6 }}>{m.timestamp}</span>
      </div>
      <div style={{ marginBottom: "8px" }}>{m.text}</div>
      <div style={{ display: "flex", gap: "4px", flexWrap: "wrap" }}>
        {Object.entries(m.reactions).map(([key, count]) => (
          <button className="panel-btn" key={key} style={{ ...reactionBtnStyle, display: "inline-flex", alignItems: "center", gap: "4px" }} onClick={() => handleReaction(m.id, key)}>
            {reactionIconMap[key] ?? key} {count}
          </button>
        ))}
        {reactionKeys.filter(k => !(k in m.reactions)).slice(0, 2).map(key => (
          <button className="panel-btn" key={key} style={{ ...reactionBtnStyle, opacity: 0.5, display: "inline-flex", alignItems: "center" }}
            onClick={() => handleReaction(m.id, key)}>{reactionIconMap[key]}</button>
        ))}
      </div>
    </div>
  );

  const renderThreadList = () => (
    <div style={{ marginBottom: "12px" }}>
      <div style={{ display: "flex", gap: "8px", marginBottom: "8px" }}>
        <input style={{ ...inputStyle, flex: 1 }} placeholder="New thread topic..."
          value={newTopic} onChange={e => setNewTopic(e.target.value)}
          onKeyDown={e => e.key === "Enter" && handleCreateThread()} />
        <button className="panel-btn panel-btn-primary" onClick={handleCreateThread}>New Thread</button>
      </div>
      {threads.map(t => (
        <div role="button" tabIndex={0} key={t.id} className="panel-card" style={{
          cursor: "pointer",
          border: activeThread?.id === t.id ? "1px solid var(--accent-color)" : "1px solid var(--border-color)",
          display: "flex", justifyContent: "space-between", alignItems: "center",
        }} onClick={() => handleSelectThread(t.id)}>
          <div>
            <div style={{ fontWeight: 600 }}>{t.topic}</div>
            <div style={{ fontSize: "var(--font-size-sm)", opacity: 0.6 }}>{t.messages.length} messages</div>
          </div>
          <button className="panel-btn panel-btn-danger" style={{ padding: "4px 8px", fontSize: "var(--font-size-sm)" }}
            onClick={e => { e.stopPropagation(); handleDeleteThread(t.id); }}>Delete</button>
        </div>
      ))}
      {threads.length === 0 && <div style={{ opacity: 0.6 }}>No threads yet. Create one above.</div>}
    </div>
  );

  const renderDiscussion = () => (
    <div>
      {renderThreadList()}
      {activeThread && (
        <>
          <div className="panel-card" style={{ marginBottom: "12px" }}>
            <div style={{ fontSize: "var(--font-size-sm)", opacity: 0.6 }}>Topic</div>
            <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)" }}>{activeThread.topic}</div>
          </div>
          {messages.map(renderMessageCard)}
          <div style={{ marginTop: "12px", display: "flex", gap: "8px" }}>
            <select style={{ ...inputStyle, width: "130px" }} value={newType}
              onChange={e => setNewType(e.target.value as MessageType)}>
              {messageTypes.map(t => <option key={t} value={t}>{t}</option>)}
            </select>
            <input style={{ ...inputStyle, flex: 1 }} placeholder="Add a message..." value={newText}
              onChange={e => setNewText(e.target.value)} onKeyDown={e => e.key === "Enter" && handleAddMessage()} />
            <button className="panel-btn panel-btn-primary" onClick={handleAddMessage}>Add</button>
          </div>
        </>
      )}
    </div>
  );

  const decisions = messages.filter(m => m.type === "Decision" || m.type === "Action");
  const renderDecisions = () => (
    <div>
      <h3 style={{ margin: "0 0 12px" }}>Decisions & Actions ({decisions.length})</h3>
      {decisions.length === 0
        ? <div className="panel-empty">No decisions or actions recorded yet.</div>
        : decisions.map(renderMessageCard)}
    </div>
  );

  const decisionCount = messages.filter(m => m.type === "Decision").length;
  const actionCount = messages.filter(m => m.type === "Action").length;
  const unresolvedCount = messages.filter(m => m.type === "Question" || m.type === "Concern").length;
  const stateColors: Record<string, string> = { Building: "#2e7d32", Discussing: "#1565c0", Paused: "#757575" };

  const renderSummary = () => (
    <div>
      <h3 style={{ margin: "0 0 12px" }}>Discussion Summary</h3>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "8px", marginBottom: "16px" }}>
        {[["Decisions", decisionCount, "var(--accent-purple)"], ["Actions", actionCount, "#c62828"], ["Unresolved", unresolvedCount, "#e65100"]].map(([label, count, color]) => (
          <div key={label as string} className="panel-card" style={{ textAlign: "center" }}>
            <div style={{ fontSize: "24px", fontWeight: 700, color: color as string }}>{count as number}</div>
            <div style={{ fontSize: "var(--font-size-base)", opacity: 0.7 }}>{label as string}</div>
          </div>
        ))}
      </div>
      <div className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <span style={{ fontWeight: 600 }}>Build State</span>
        <div style={{ display: "flex", gap: "4px" }}>
          {(["Building", "Discussing", "Paused"] as const).map(s => (
            <button key={s} onClick={() => setBuildState(s)}
              style={{ padding: "4px 12px", fontSize: "var(--font-size-base)", cursor: "pointer", borderRadius: "var(--radius-xs-plus)",
                backgroundColor: buildState === s ? stateColors[s] : "transparent",
                color: "var(--text-primary)",
                border: `1px solid ${stateColors[s]}` }}>
              {s}
            </button>
          ))}
        </div>
      </div>
    </div>
  );

  if (loading) {
    return <div className="panel-loading"><p>Loading discussion threads...</p></div>;
  }

  return (
    <div className="panel-container">
      <div className="panel-header">
        <h2 style={{ margin: 0 }}>Discussion Mode</h2>
      </div>
      {error && <div className="panel-error" style={{ margin: "0 16px 8px" }}>{error}</div>}
      <div className="panel-tab-bar">
        {[["discussion", "Discussion"], ["decisions", "Decisions"], ["summary", "Summary"]].map(([id, label]) => (
          <button key={id} className={`panel-tab ${activeTab === id ? "active" : ""}`} onClick={() => setActiveTab(id)}>{label}</button>
        ))}
      </div>
      <div className="panel-body">
        {activeTab === "discussion" && renderDiscussion()}
        {activeTab === "decisions" && renderDecisions()}
        {activeTab === "summary" && renderSummary()}
      </div>
    </div>
  );
};

export default DiscussionModePanel;
