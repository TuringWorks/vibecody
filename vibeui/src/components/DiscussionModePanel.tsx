import React, { useState } from "react";

type MessageType = "Question" | "Suggestion" | "Concern" | "Decision" | "Action";

interface DiscussionMessage {
  id: string;
  author: string;
  type: MessageType;
  text: string;
  reactions: Record<string, number>;
  timestamp: string;
}

const DiscussionModePanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("discussion");
  const [topic] = useState("API redesign for v3 migration");
  const [newText, setNewText] = useState("");
  const [newType, setNewType] = useState<MessageType>("Question");
  const [buildState, setBuildState] = useState<"Building" | "Discussing" | "Paused">("Discussing");
  const [messages, setMessages] = useState<DiscussionMessage[]>([
    { id: "m1", author: "Alice", type: "Question", text: "Should we use REST or GraphQL for the new endpoints?", reactions: { "\u{1F44D}": 3, "\u{1F914}": 1 }, timestamp: "10:02 AM" },
    { id: "m2", author: "Bob", type: "Suggestion", text: "GraphQL would reduce over-fetching for the mobile clients.", reactions: { "\u{1F44D}": 5 }, timestamp: "10:05 AM" },
    { id: "m3", author: "Carol", type: "Concern", text: "GraphQL adds complexity to the backend. We need to consider caching.", reactions: { "\u{1F44D}": 2 }, timestamp: "10:08 AM" },
    { id: "m4", author: "Alice", type: "Decision", text: "Use GraphQL for read-heavy endpoints, REST for writes.", reactions: { "\u2705": 4 }, timestamp: "10:15 AM" },
    { id: "m5", author: "Bob", type: "Action", text: "Create proof-of-concept GraphQL schema by Friday.", reactions: { "\u{1F44D}": 2 }, timestamp: "10:18 AM" },
  ]);

  const containerStyle: React.CSSProperties = {
    padding: "16px", color: "var(--vscode-foreground)",
    backgroundColor: "var(--vscode-editor-background)",
    fontFamily: "var(--vscode-font-family)", fontSize: "var(--vscode-font-size)",
    height: "100%", overflow: "auto",
  };
  const tabBarStyle: React.CSSProperties = {
    display: "flex", gap: "4px", marginBottom: "16px",
    borderBottom: "1px solid var(--vscode-panel-border)", paddingBottom: "8px",
  };
  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: "6px 14px", cursor: "pointer", border: "none",
    backgroundColor: active ? "var(--vscode-button-background)" : "transparent",
    color: active ? "var(--vscode-button-foreground)" : "var(--vscode-foreground)",
    borderRadius: "4px", fontSize: "var(--vscode-font-size)",
  });
  const inputStyle: React.CSSProperties = {
    width: "100%", padding: "6px 10px", boxSizing: "border-box",
    backgroundColor: "var(--vscode-input-background)", color: "var(--vscode-input-foreground)",
    border: "1px solid var(--vscode-input-border)", borderRadius: "4px",
  };
  const btnStyle: React.CSSProperties = {
    padding: "6px 14px", cursor: "pointer", border: "none", borderRadius: "4px",
    backgroundColor: "var(--vscode-button-background)", color: "var(--vscode-button-foreground)",
  };
  const cardStyle: React.CSSProperties = {
    padding: "10px", marginBottom: "8px", borderRadius: "4px",
    backgroundColor: "var(--vscode-editor-inactiveSelectionBackground)",
    border: "1px solid var(--vscode-panel-border)",
  };

  const typeColors: Record<MessageType, string> = {
    Question: "#1565c0", Suggestion: "#2e7d32", Concern: "#e65100",
    Decision: "#6a1b9a", Action: "#c62828",
  };
  const badgeStyle = (color: string): React.CSSProperties => ({
    display: "inline-block", padding: "2px 8px", borderRadius: "10px",
    fontSize: "11px", fontWeight: 600, backgroundColor: color, color: "#fff",
  });
  const reactionBtnStyle: React.CSSProperties = {
    padding: "2px 6px", border: "1px solid var(--vscode-panel-border)",
    borderRadius: "12px", backgroundColor: "transparent", cursor: "pointer",
    fontSize: "12px", color: "var(--vscode-foreground)",
  };

  const messageTypes: MessageType[] = ["Question", "Suggestion", "Concern", "Decision", "Action"];
  const reactionEmojis = ["\u{1F44D}", "\u{1F44E}", "\u2705", "\u{1F914}", "\u{1F525}"];

  const handleAddMessage = () => {
    if (!newText.trim()) return;
    setMessages(prev => [...prev, {
      id: `m-${Date.now()}`, author: "You", type: newType, text: newText,
      reactions: {}, timestamp: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
    }]);
    setNewText("");
  };

  const handleReaction = (msgId: string, emoji: string) => {
    setMessages(prev => prev.map(m =>
      m.id === msgId ? { ...m, reactions: { ...m.reactions, [emoji]: (m.reactions[emoji] || 0) + 1 } } : m
    ));
  };

  const renderMessageCard = (m: DiscussionMessage) => (
    <div key={m.id} style={cardStyle}>
      <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "6px" }}>
        <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
          <strong>{m.author}</strong>
          <span style={badgeStyle(typeColors[m.type])}>{m.type}</span>
        </div>
        <span style={{ fontSize: "11px", opacity: 0.6 }}>{m.timestamp}</span>
      </div>
      <div style={{ marginBottom: "6px" }}>{m.text}</div>
      <div style={{ display: "flex", gap: "4px", flexWrap: "wrap" }}>
        {Object.entries(m.reactions).map(([emoji, count]) => (
          <button key={emoji} style={reactionBtnStyle} onClick={() => handleReaction(m.id, emoji)}>
            {emoji} {count}
          </button>
        ))}
        {reactionEmojis.filter(e => !(e in m.reactions)).slice(0, 2).map(emoji => (
          <button key={emoji} style={{ ...reactionBtnStyle, opacity: 0.5 }}
            onClick={() => handleReaction(m.id, emoji)}>{emoji}</button>
        ))}
      </div>
    </div>
  );

  const renderDiscussion = () => (
    <div>
      <div style={{ ...cardStyle, marginBottom: "12px" }}>
        <div style={{ fontSize: "11px", opacity: 0.6 }}>Topic</div>
        <div style={{ fontWeight: 600, fontSize: "14px" }}>{topic}</div>
      </div>
      {messages.map(renderMessageCard)}
      <div style={{ marginTop: "12px", display: "flex", gap: "8px" }}>
        <select style={{ ...inputStyle, width: "130px" }} value={newType}
          onChange={e => setNewType(e.target.value as MessageType)}>
          {messageTypes.map(t => <option key={t} value={t}>{t}</option>)}
        </select>
        <input style={{ ...inputStyle, flex: 1 }} placeholder="Add a message..." value={newText}
          onChange={e => setNewText(e.target.value)} onKeyDown={e => e.key === "Enter" && handleAddMessage()} />
        <button style={btnStyle} onClick={handleAddMessage}>Add</button>
      </div>
    </div>
  );

  const decisions = messages.filter(m => m.type === "Decision" || m.type === "Action");
  const renderDecisions = () => (
    <div>
      <h3 style={{ margin: "0 0 12px" }}>Decisions & Actions ({decisions.length})</h3>
      {decisions.length === 0
        ? <div style={{ opacity: 0.6 }}>No decisions or actions recorded yet.</div>
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
        {[["Decisions", decisionCount, "#6a1b9a"], ["Actions", actionCount, "#c62828"], ["Unresolved", unresolvedCount, "#e65100"]].map(([label, count, color]) => (
          <div key={label as string} style={{ ...cardStyle, textAlign: "center" }}>
            <div style={{ fontSize: "24px", fontWeight: 700, color: color as string }}>{count as number}</div>
            <div style={{ fontSize: "12px", opacity: 0.7 }}>{label as string}</div>
          </div>
        ))}
      </div>
      <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <span style={{ fontWeight: 600 }}>Build State</span>
        <div style={{ display: "flex", gap: "4px" }}>
          {(["Building", "Discussing", "Paused"] as const).map(s => (
            <button key={s} onClick={() => setBuildState(s)}
              style={{ ...btnStyle, backgroundColor: buildState === s ? stateColors[s] : "transparent",
                color: buildState === s ? "#fff" : "var(--vscode-foreground)",
                border: `1px solid ${stateColors[s]}`, fontSize: "12px", padding: "4px 10px" }}>
              {s}
            </button>
          ))}
        </div>
      </div>
    </div>
  );

  return (
    <div style={containerStyle}>
      <h2 style={{ margin: "0 0 12px" }}>Discussion Mode</h2>
      <div style={tabBarStyle}>
        {[["discussion", "Discussion"], ["decisions", "Decisions"], ["summary", "Summary"]].map(([id, label]) => (
          <button key={id} style={tabStyle(activeTab === id)} onClick={() => setActiveTab(id)}>{label}</button>
        ))}
      </div>
      {activeTab === "discussion" && renderDiscussion()}
      {activeTab === "decisions" && renderDecisions()}
      {activeTab === "summary" && renderSummary()}
    </div>
  );
};

export default DiscussionModePanel;
