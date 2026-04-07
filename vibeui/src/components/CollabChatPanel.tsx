import { useState, useRef, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Users, Send, LogOut, Copy, Check, Bot, Loader2, WifiOff } from "lucide-react";

// ── Types ─────────────────────────────────────────────────────────────────────

interface CollabSessionInfo {
  room_id: string;
  peer_id: string;
  ws_url: string;
  peers: Array<{ peer_id: string; name: string; color: string }>;
}

// WebSocket message variants broadcast to all peers in the room
type WsMsg =
  | { type: "collab_chat"; sender_id: string; sender_name: string; sender_color: string; content: string; timestamp: number; message_id: string }
  | { type: "collab_ai_chunk"; message_id: string; chunk: string }
  | { type: "collab_ai_complete"; message_id: string; full_content: string }
  | { type: "collab_ai_error"; message_id: string; error: string };

interface DisplayMsg {
  id: string;
  kind: "user" | "ai" | "system";
  senderId: string;
  senderName: string;
  senderColor: string;
  content: string;
  timestamp: number;
  streaming?: boolean;
  isError?: boolean;
}

interface ChatResponse {
  message: string;
  tool_output?: string;
  pending_write?: { path: string; content: string };
}

export interface CollabChatPanelProps {
  provider?: string;
  daemonPort?: number;
}

// ── Constants ─────────────────────────────────────────────────────────────────

const PEER_COLORS = [
  "#4f9cf9", "#f97b22", "var(--success-color)", "var(--error-color)",
  "#9c27b0", "#00bcd4", "var(--warning-color)", "#795548",
];

// ── Component ─────────────────────────────────────────────────────────────────

export function CollabChatPanel({ provider = "claude", daemonPort = 7878 }: CollabChatPanelProps) {
  // Room state
  const [connected, setConnected] = useState(false);
  const [roomId, setRoomId] = useState<string | null>(null);
  const [myPeerId, setMyPeerId] = useState<string | null>(null);
  const [myColor] = useState(() => PEER_COLORS[Math.floor(Math.random() * PEER_COLORS.length)]);
  const [peers, setPeers] = useState<Array<{ peerId: string; name: string; color: string }>>([]);
  const [userName, setUserName] = useState("User");
  const [joinRoomId, setJoinRoomId] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  // Chat state
  const [messages, setMessages] = useState<DisplayMsg[]>([]);
  const [input, setInput] = useState("");
  const [aiLoading, setAiLoading] = useState(false);

  const wsRef = useRef<WebSocket | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const unlistenRefs = useRef<Array<() => void>>([]);

  // Auto-scroll on new messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      wsRef.current?.close();
      unlistenRefs.current.forEach((fn) => fn());
    };
  }, []);

  // ── WebSocket setup ────────────────────────────────────────────────────────

  const openWs = useCallback((wsUrl: string) => {
    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onopen = () => { setConnected(true); setError(null); };
    ws.onerror = () => setError("WebSocket connection error. Is the VibeCLI daemon running on port " + daemonPort + "?");
    ws.onclose = () => setConnected(false);

    ws.onmessage = (ev) => {
      let msg: WsMsg;
      try { msg = JSON.parse(ev.data); } catch { return; }

      switch (msg.type) {
        case "collab_chat":
          setMessages((prev) => [...prev, {
            id: msg.message_id,
            kind: "user",
            senderId: msg.sender_id,
            senderName: msg.sender_name,
            senderColor: msg.sender_color,
            content: msg.content,
            timestamp: msg.timestamp,
          }]);
          break;

        case "collab_ai_chunk":
          setMessages((prev) => {
            const exists = prev.some((m) => m.id === `ai-${msg.message_id}`);
            if (!exists) {
              return [...prev, {
                id: `ai-${msg.message_id}`,
                kind: "ai",
                senderId: "ai",
                senderName: "AI",
                senderColor: "#4f9cf9",
                content: msg.chunk,
                timestamp: Date.now(),
                streaming: true,
              }];
            }
            return prev.map((m) =>
              m.id === `ai-${msg.message_id}`
                ? { ...m, content: m.content + msg.chunk, streaming: true }
                : m
            );
          });
          break;

        case "collab_ai_complete":
          setAiLoading(false);
          setMessages((prev) => prev.map((m) =>
            m.id === `ai-${msg.message_id}`
              ? { ...m, content: msg.full_content, streaming: false }
              : m
          ));
          break;

        case "collab_ai_error":
          setAiLoading(false);
          setMessages((prev) => prev.map((m) =>
            m.id === `ai-${msg.message_id}`
              ? { ...m, content: msg.error, streaming: false, isError: true }
              : m
          ));
          break;
      }
    };
  }, [daemonPort]);

  // ── Room actions ───────────────────────────────────────────────────────────

  const addSystem = (content: string) =>
    setMessages((prev) => [...prev, {
      id: `sys-${Date.now()}`,
      kind: "system",
      senderId: "system",
      senderName: "System",
      senderColor: "#888",
      content,
      timestamp: Date.now(),
    }]);

  const handleCreate = async () => {
    setLoading(true);
    setError(null);
    try {
      const session = await invoke<CollabSessionInfo>("create_collab_session", {
        roomId: null,
        userName,
        daemonPort,
      });
      setRoomId(session.room_id);
      setMyPeerId(session.peer_id);
      setPeers((session.peers || []).map((p) => ({ peerId: p.peer_id, name: p.name, color: p.color })));
      openWs(session.ws_url);
      addSystem(`Room created — ID: ${session.room_id}. Share this with teammates.`);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleJoin = async () => {
    const rid = joinRoomId.trim();
    if (!rid) return;
    setLoading(true);
    setError(null);
    try {
      const session = await invoke<CollabSessionInfo>("join_collab_session", {
        roomId: rid,
        userName,
        daemonPort,
      });
      setRoomId(session.room_id);
      setMyPeerId(session.peer_id);
      setPeers((session.peers || []).map((p) => ({ peerId: p.peer_id, name: p.name, color: p.color })));
      openWs(session.ws_url);
      addSystem(`Joined room ${session.room_id}.`);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleLeave = () => {
    wsRef.current?.close();
    unlistenRefs.current.forEach((fn) => fn());
    unlistenRefs.current = [];
    invoke("leave_collab_session").catch(() => {});
    setConnected(false);
    setRoomId(null);
    setMyPeerId(null);
    setPeers([]);
    setMessages([]);
    setAiLoading(false);
  };

  const handleCopy = () => {
    if (!roomId) return;
    navigator.clipboard.writeText(roomId).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };

  // ── Send message ───────────────────────────────────────────────────────────

  const send = useCallback(async () => {
    const content = input.trim();
    if (!content || aiLoading || !wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) return;

    const messageId = `${Date.now()}-${Math.random().toString(36).slice(2)}`;
    const aiMsgId = `ai-${messageId}`;

    // Broadcast user message to all peers
    const chatWsMsg: WsMsg = {
      type: "collab_chat",
      sender_id: myPeerId || "local",
      sender_name: userName,
      sender_color: myColor,
      content,
      timestamp: Date.now(),
      message_id: messageId,
    };
    wsRef.current.send(JSON.stringify(chatWsMsg));

    // Add AI placeholder locally (sender only — peers get it via collab_ai_chunk)
    setMessages((prev) => [...prev, {
      id: aiMsgId,
      kind: "ai",
      senderId: "ai",
      senderName: "AI",
      senderColor: "#4f9cf9",
      content: "",
      timestamp: Date.now(),
      streaming: true,
    }]);

    setInput("");
    setAiLoading(true);

    // Tear down previous listeners
    unlistenRefs.current.forEach((fn) => fn());
    unlistenRefs.current = [];

    let accumulated = "";

    // Build conversation history for AI
    const history = messages
      .filter((m) => m.kind !== "system")
      .map((m) => ({
        role: m.kind === "ai" ? "assistant" as const : "user" as const,
        content: m.kind === "user" ? `[${m.senderName}]: ${m.content}` : m.content,
      }));
    history.push({ role: "user", content: `[${userName}]: ${content}` });

    // Listen for AI stream events and forward to WebSocket
    const unChunk = await listen<string>("chat:chunk", (ev) => {
      const chunk = ev.payload;
      accumulated += chunk;
      setMessages((prev) => prev.map((m) =>
        m.id === aiMsgId ? { ...m, content: accumulated, streaming: true } : m
      ));
      if (wsRef.current?.readyState === WebSocket.OPEN) {
        wsRef.current.send(JSON.stringify({ type: "collab_ai_chunk", message_id: messageId, chunk } satisfies WsMsg));
      }
    });

    const unComplete = await listen<ChatResponse>("chat:complete", (ev) => {
      const full = ev.payload.message;
      accumulated = full;
      setAiLoading(false);
      setMessages((prev) => prev.map((m) =>
        m.id === aiMsgId ? { ...m, content: full, streaming: false } : m
      ));
      if (wsRef.current?.readyState === WebSocket.OPEN) {
        wsRef.current.send(JSON.stringify({ type: "collab_ai_complete", message_id: messageId, full_content: full } satisfies WsMsg));
      }
      unChunk();
      unComplete();
      unError();
    });

    const unError = await listen<string>("chat:error", (ev) => {
      const err = ev.payload;
      setAiLoading(false);
      setMessages((prev) => prev.map((m) =>
        m.id === aiMsgId ? { ...m, content: err, streaming: false, isError: true } : m
      ));
      if (wsRef.current?.readyState === WebSocket.OPEN) {
        wsRef.current.send(JSON.stringify({ type: "collab_ai_error", message_id: messageId, error: err } satisfies WsMsg));
      }
      unChunk();
      unComplete();
      unError();
    });

    unlistenRefs.current = [unChunk, unComplete, unError];

    // Kick off AI stream
    try {
      await invoke("stream_chat_message", {
        request: {
          messages: history,
          provider,
          context: "Collaborative AI chat session — multiple users are participating. Respond clearly and concisely.",
          file_tree: null,
          current_file: null,
          mode: "chat",
          attachments: [],
        },
      });
    } catch (e) {
      setAiLoading(false);
      setMessages((prev) => prev.map((m) =>
        m.id === aiMsgId ? { ...m, content: String(e), streaming: false, isError: true } : m
      ));
      unlistenRefs.current.forEach((fn) => fn());
      unlistenRefs.current = [];
    }
  }, [input, aiLoading, messages, myPeerId, userName, myColor, provider]);

  // ── Render ─────────────────────────────────────────────────────────────────

  if (!connected) {
    return (
      <div style={{ padding: 16, display: "flex", flexDirection: "column", gap: 12, flex: 1, minHeight: 0, overflow: "auto" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
          <Users size={16} style={{ color: "var(--text-secondary)" }} />
          <span style={{ fontWeight: 600, fontSize: 14 }}>Collaborative AI Chat</span>
        </div>
        <p style={{ fontSize: 12, color: "var(--text-secondary)", margin: 0 }}>
          Create or join a room to chat with AI together. All participants see messages and AI responses in real time.
          Requires <code>vibecli --serve --port {daemonPort}</code>.
        </p>

        <label style={{ fontSize: 12, color: "var(--text-secondary)" }}>Your name</label>
        <input
          style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 6, padding: "6px 10px", fontSize: 13, color: "var(--text-primary)" }}
          value={userName}
          onChange={(e) => setUserName(e.target.value)}
          placeholder="Your display name"
        />

        <button
          onClick={handleCreate}
          disabled={loading || !userName.trim()}
          style={{ background: "var(--accent)", color: "#fff", border: "none", borderRadius: 6, padding: "8px 14px", cursor: "pointer", fontSize: 13, fontWeight: 500 }}
        >
          {loading ? "Creating…" : "Create Room"}
        </button>

        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <div style={{ flex: 1, height: 1, background: "var(--border-color)" }} />
          <span style={{ fontSize: 11, color: "var(--text-muted)" }}>or join</span>
          <div style={{ flex: 1, height: 1, background: "var(--border-color)" }} />
        </div>

        <input
          style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 6, padding: "6px 10px", fontSize: 13, color: "var(--text-primary)" }}
          value={joinRoomId}
          onChange={(e) => setJoinRoomId(e.target.value)}
          placeholder="Room ID"
          onKeyDown={(e) => e.key === "Enter" && handleJoin()}
        />
        <button
          onClick={handleJoin}
          disabled={loading || !joinRoomId.trim() || !userName.trim()}
          style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 6, padding: "8px 14px", cursor: "pointer", fontSize: 13 }}
        >
          {loading ? "Joining…" : "Join Room"}
        </button>

        {error && (
          <div className="panel-error">
            {error}
          </div>
        )}
      </div>
    );
  }

  // ── Connected view ─────────────────────────────────────────────────────────

  return (
    <div style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, overflow: "hidden" }}>
      {/* Header */}
      <div className="panel-header">
        <div style={{ width: 8, height: 8, borderRadius: "50%", background: "var(--success-color)", flexShrink: 0 }} />
        <span style={{ fontSize: 12, fontWeight: 500, flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
          Room: {roomId}
        </span>
        <button onClick={handleCopy} title="Copy room ID" style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", padding: 2 }}>
          {copied ? <Check size={14} style={{ color: "var(--success-color)" }} /> : <Copy size={14} />}
        </button>
        <button onClick={handleLeave} title="Leave room" style={{ background: "none", border: "none", cursor: "pointer", color: "var(--text-secondary)", padding: 2 }}>
          <LogOut size={14} />
        </button>
      </div>

      {/* Peer avatars */}
      {peers.length > 0 && (
        <div style={{ display: "flex", alignItems: "center", gap: 6, padding: "6px 12px", borderBottom: "1px solid var(--border-color)", flexShrink: 0 }}>
          <Users size={12} style={{ color: "var(--text-muted)" }} />
          {[{ peerId: myPeerId || "me", name: userName, color: myColor }, ...peers].map((p) => (
            <span key={p.peerId} title={p.name} style={{ width: 22, height: 22, borderRadius: "50%", background: p.color, display: "flex", alignItems: "center", justifyContent: "center", fontSize: 10, color: "#fff", fontWeight: 700, flexShrink: 0 }}>
              {p.name.charAt(0).toUpperCase()}
            </span>
          ))}
        </div>
      )}

      {/* Messages */}
      <div style={{ flex: 1, overflowY: "auto", padding: "12px 12px 0" }}>
        {messages.map((msg) => (
          <MessageBubble key={msg.id} msg={msg} myPeerId={myPeerId} />
        ))}
        {aiLoading && messages.every((m) => !m.streaming) && (
          <div style={{ display: "flex", alignItems: "center", gap: 6, padding: "8px 0", color: "var(--text-secondary)", fontSize: 12 }}>
            <Loader2 size={13} style={{ animation: "spin 1s linear infinite" }} />
            <span>AI is thinking…</span>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div style={{ display: "flex", gap: 8, padding: 12, borderTop: "1px solid var(--border-color)", flexShrink: 0 }}>
        <textarea
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); send(); } }}
          placeholder="Message the AI… (Enter to send)"
          disabled={aiLoading}
          rows={2}
          style={{ flex: 1, resize: "none", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 8, padding: "8px 10px", fontSize: 13, color: "var(--text-primary)", fontFamily: "inherit" }}
        />
        <button
          onClick={send}
          disabled={aiLoading || !input.trim()}
          title="Send"
          style={{ background: "var(--accent)", border: "none", borderRadius: 8, padding: "0 12px", cursor: "pointer", color: "#fff", display: "flex", alignItems: "center" }}
        >
          {aiLoading ? <Loader2 size={15} style={{ animation: "spin 1s linear infinite" }} /> : <Send size={15} />}
        </button>
      </div>

      {!connected && (
        <div style={{ display: "flex", alignItems: "center", gap: 6, padding: "6px 12px", fontSize: 11, color: "#f97b22", background: "rgba(249,123,34,0.1)" }}>
          <WifiOff size={12} /> Disconnected
        </div>
      )}
    </div>
  );
}

// ── Message bubble ─────────────────────────────────────────────────────────────

function MessageBubble({ msg, myPeerId }: { msg: DisplayMsg; myPeerId: string | null }) {
  if (msg.kind === "system") {
    return (
      <div style={{ textAlign: "center", fontSize: 11, color: "var(--text-muted)", padding: "4px 0 8px" }}>
        {msg.content}
      </div>
    );
  }

  const isMe = msg.senderId === myPeerId;
  const isAi = msg.kind === "ai";

  return (
    <div style={{ marginBottom: 12, display: "flex", flexDirection: "column", alignItems: isMe ? "flex-end" : "flex-start" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
        {isAi && <Bot size={13} style={{ color: "#4f9cf9" }} />}
        {!isAi && (
          <span style={{ width: 18, height: 18, borderRadius: "50%", background: msg.senderColor, display: "inline-flex", alignItems: "center", justifyContent: "center", fontSize: 9, color: "#fff", fontWeight: 700 }}>
            {msg.senderName.charAt(0).toUpperCase()}
          </span>
        )}
        <span style={{ fontSize: 11, color: "var(--text-muted)" }}>
          {isAi ? "AI Assistant" : msg.senderName}
          {isMe && !isAi && " (you)"}
        </span>
        {msg.streaming && <span style={{ fontSize: 10, color: "#4f9cf9" }}>●</span>}
      </div>
      <div style={{
        maxWidth: "85%",
        background: isAi ? "var(--bg-secondary)" : isMe ? "var(--accent)" : "var(--bg-tertiary, var(--bg-secondary))",
        color: isMe && !isAi ? "#fff" : "var(--text-primary)",
        borderRadius: isMe ? "12px 12px 4px 12px" : "12px 12px 12px 4px",
        padding: "8px 12px",
        fontSize: 13,
        lineHeight: 1.5,
        whiteSpace: "pre-wrap",
        wordBreak: "break-word",
        border: isAi ? "1px solid var(--border-color)" : "none",
        ...(msg.isError ? { borderColor: "rgba(239,68,68,0.4)", color: "var(--error-color)" } : {}),
      }}>
        {msg.content || (msg.streaming ? <span style={{ opacity: 0.4 }}>…</span> : "")}
      </div>
    </div>
  );
}
