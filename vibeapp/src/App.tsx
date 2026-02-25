import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

// ── Types ─────────────────────────────────────────────────────────────────────

interface Message {
  role: "user" | "assistant" | "system";
  content: string;
  streaming?: boolean;
}

// ── Settings ──────────────────────────────────────────────────────────────────

const DAEMON_URL_KEY = "vibeapp_daemon_url";
const PROVIDER_KEY   = "vibeapp_provider";
const DEFAULT_URL    = "http://localhost:7878";

function loadSetting(key: string, fallback: string): string {
  return localStorage.getItem(key) ?? fallback;
}

// ── Main App ──────────────────────────────────────────────────────────────────

export default function App() {
  const [messages, setMessages]     = useState<Message[]>([]);
  const [input, setInput]           = useState("");
  const [loading, setLoading]       = useState(false);
  const [daemonUrl, setDaemonUrl]   = useState(() => loadSetting(DAEMON_URL_KEY, DEFAULT_URL));
  const [provider, setProvider]     = useState(() => loadSetting(PROVIDER_KEY, "claude"));
  const [daemonOk, setDaemonOk]     = useState<boolean | null>(null);
  const [pinned, setPinned]         = useState(false);
  const [showSettings, setShowSettings] = useState(false);

  const bottomRef   = useRef<HTMLDivElement>(null);
  const inputRef    = useRef<HTMLTextAreaElement>(null);
  const esRef       = useRef<EventSource | null>(null);

  // ── Daemon health-check ──────────────────────────────────────────────────
  useEffect(() => {
    const check = async () => {
      try {
        await invoke("check_daemon", { url: daemonUrl });
        setDaemonOk(true);
      } catch {
        setDaemonOk(false);
      }
    };
    check();
    const id = setInterval(check, 10_000);
    return () => clearInterval(id);
  }, [daemonUrl]);

  // ── Auto-scroll ──────────────────────────────────────────────────────────
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  // ── Always-on-top toggle ─────────────────────────────────────────────────
  const togglePin = async () => {
    const next = !pinned;
    setPinned(next);
    await invoke("set_always_on_top", { alwaysOnTop: next });
  };

  // ── Send message via daemon SSE ──────────────────────────────────────────
  const send = useCallback(async () => {
    const text = input.trim();
    if (!text || loading) return;
    setInput("");
    setLoading(true);

    // Cancel any in-flight SSE stream
    esRef.current?.close();

    const userMsg: Message = { role: "user", content: text };
    setMessages(prev => [...prev, userMsg]);

    try {
      // POST /agent to start session
      const res = await fetch(`${daemonUrl}/agent`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          task: text,
          provider,
          approval: "full-auto",
        }),
      });

      if (!res.ok) {
        throw new Error(`Daemon returned ${res.status}: ${res.statusText}`);
      }

      const { session_id } = await res.json() as { session_id: string };

      // Append placeholder streaming message
      const streamId = Date.now();
      setMessages(prev => [
        ...prev,
        { role: "assistant", content: "", streaming: true },
      ]);

      // Stream events
      const es = new EventSource(`${daemonUrl}/stream/${session_id}`);
      esRef.current = es;

      const update = (patch: (prev: string) => string) =>
        setMessages(prev => {
          const copy = [...prev];
          const last = copy[copy.length - 1];
          if (last && last.streaming) {
            copy[copy.length - 1] = { ...last, content: patch(last.content) };
          }
          return copy;
        });

      es.onmessage = (e) => {
        try {
          const ev = JSON.parse(e.data) as Record<string, unknown>;
          if (ev.type === "chunk" && typeof ev.text === "string") {
            update(p => p + ev.text);
          } else if (ev.type === "complete") {
            es.close();
            setMessages(prev => {
              const copy = [...prev];
              const last = copy[copy.length - 1];
              if (last?.streaming) copy[copy.length - 1] = { ...last, streaming: false };
              return copy;
            });
            setLoading(false);
          } else if (ev.type === "error") {
            update(p => p + `\n\n⚠️ ${ev.message ?? "unknown error"}`);
            es.close();
            setLoading(false);
          }
        } catch { /* ignore parse errors */ }
      };

      es.onerror = () => {
        es.close();
        setLoading(false);
      };

      void streamId; // suppress lint
    } catch (err) {
      setMessages(prev => [
        ...prev,
        { role: "system", content: `❌ ${String(err)}` },
      ]);
      setLoading(false);
    }
  }, [input, loading, daemonUrl, provider]);

  // ── Keyboard handler ─────────────────────────────────────────────────────
  const onKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
    if (e.key === "Escape") {
      setShowSettings(false);
    }
  };

  // ── Drag-handle for frameless window ────────────────────────────────────
  const onDragStart = async (e: React.MouseEvent) => {
    e.preventDefault();
    await invoke("start_drag");
  };

  // ── Save settings ────────────────────────────────────────────────────────
  const saveSettings = () => {
    localStorage.setItem(DAEMON_URL_KEY, daemonUrl);
    localStorage.setItem(PROVIDER_KEY, provider);
    setShowSettings(false);
  };

  // ── Render ───────────────────────────────────────────────────────────────
  return (
    <div className="app">
      {/* Title bar (drag handle for frameless window) */}
      <div className="titlebar" onMouseDown={onDragStart}>
        <span className="titlebar-title">VibeCLI</span>
        <div className="titlebar-actions">
          <span
            className={`daemon-dot ${daemonOk === true ? "ok" : daemonOk === false ? "err" : "unknown"}`}
            title={daemonOk === true ? "Daemon online" : daemonOk === false ? "Daemon offline" : "Checking..."}
          />
          <button
            className={`btn-icon ${pinned ? "pinned" : ""}`}
            onClick={togglePin}
            title={pinned ? "Unpin window" : "Pin on top"}
          >
            📌
          </button>
          <button
            className="btn-icon"
            onClick={() => setShowSettings(s => !s)}
            title="Settings"
          >
            ⚙️
          </button>
          <button
            className="btn-icon"
            onClick={() => invoke("hide_window")}
            title="Send to tray"
          >
            —
          </button>
        </div>
      </div>

      {/* Settings panel */}
      {showSettings && (
        <div className="settings-panel">
          <label>
            Daemon URL
            <input
              value={daemonUrl}
              onChange={e => setDaemonUrl(e.target.value)}
              placeholder="http://localhost:7878"
            />
          </label>
          <label>
            Provider
            <select value={provider} onChange={e => setProvider(e.target.value)}>
              <option value="claude">Claude</option>
              <option value="openai">OpenAI</option>
              <option value="ollama">Ollama</option>
              <option value="gemini">Gemini</option>
              <option value="grok">Grok</option>
            </select>
          </label>
          <div className="settings-actions">
            <button onClick={saveSettings}>Save</button>
            <button onClick={() => setShowSettings(false)}>Cancel</button>
          </div>
        </div>
      )}

      {/* Message list */}
      <div className="messages">
        {messages.length === 0 && (
          <div className="empty-state">
            <div className="empty-icon">🤖</div>
            <div className="empty-title">VibeCLI</div>
            <div className="empty-subtitle">
              {daemonOk === false
                ? "Start the daemon: vibecli --serve"
                : "Ask anything about your code"}
            </div>
          </div>
        )}
        {messages.map((msg, i) => (
          <div key={i} className={`msg msg-${msg.role}`}>
            {msg.role === "user" && <div className="msg-label">You</div>}
            {msg.role === "assistant" && <div className="msg-label">AI</div>}
            <div className="msg-content">
              {msg.content || (msg.streaming ? <span className="cursor">▋</span> : null)}
            </div>
          </div>
        ))}
        <div ref={bottomRef} />
      </div>

      {/* Input area */}
      <div className="input-area">
        <textarea
          ref={inputRef}
          value={input}
          onChange={e => setInput(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder="Ask anything… (Enter to send, Shift+Enter for newline)"
          rows={2}
          disabled={loading}
        />
        <button
          className="send-btn"
          onClick={send}
          disabled={loading || !input.trim()}
        >
          {loading ? "●" : "↑"}
        </button>
      </div>
    </div>
  );
}
