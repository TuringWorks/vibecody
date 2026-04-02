import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

// ── Thin SVG Icons ───────────────────────────────────────────────────────────

const IconPin = ({ active }: { active?: boolean }) => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke={active ? "var(--primary)" : "currentColor"} strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <path d="M12 17v5M9 3h6l-1 7h3l-5 7-5-7h3z" />
  </svg>
);

const IconSettings = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
  </svg>
);

const IconMinus = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
    <path d="M5 12h14" />
  </svg>
);

const IconSend = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M12 19V5M5 12l7-7 7 7" />
  </svg>
);

const IconSparkle = () => (
  <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="var(--primary)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    <path d="M12 2l2.4 7.2L22 12l-7.6 2.8L12 22l-2.4-7.2L2 12l7.6-2.8z" />
  </svg>
);

const IconLoader = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" className="spin-icon">
    <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
  </svg>
);

// ── Types ─────────────────────────────────────────────────────────────────────

interface Message {
  role: "user" | "assistant" | "system";
  content: string;
  streaming?: boolean;
}

// ── Settings ──────────────────────────────────────────────────────────────────

const DAEMON_URL_KEY   = "vibeapp_daemon_url";
const PROVIDER_KEY     = "vibeapp_provider";
const DAEMON_TOKEN_KEY = "vibeapp_daemon_token";
const MODEL_KEY        = "vibeapp_model";
const DEFAULT_URL      = "http://localhost:7878";

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
  const [daemonToken, setDaemonToken] = useState(() => loadSetting(DAEMON_TOKEN_KEY, ""));
  const [daemonOk, setDaemonOk]     = useState<boolean | null>(null);
  const [pinned, setPinned]         = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [availableModels, setAvailableModels] = useState<Array<{ id: string; name?: string; provider?: string }>>([]);
  const [selectedModel, setSelectedModel] = useState(() => loadSetting(MODEL_KEY, ""));

  const bottomRef   = useRef<HTMLDivElement>(null);
  const inputRef    = useRef<HTMLTextAreaElement>(null);

  // ── Daemon health-check + model discovery ───────────────────────────────
  useEffect(() => {
    const check = async () => {
      try {
        await invoke("check_daemon", { url: daemonUrl });
        setDaemonOk(true);
        // Fetch available models
        try {
          const models = await invoke<Array<{ id: string; name?: string; provider?: string }>>(
            "list_daemon_models", { url: daemonUrl }
          );
          setAvailableModels(models);
          if (models.length > 0 && !selectedModel) {
            setSelectedModel(models[0].id);
          }
        } catch { /* daemon may not support /models yet */ }
      } catch {
        setDaemonOk(false);
      }
    };
    check();
    const id = setInterval(check, 30_000);
    return () => clearInterval(id);
  }, [daemonUrl]); // eslint-disable-line react-hooks/exhaustive-deps

  // ── Auto-scroll ──────────────────────────────────────────────────────────
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  // ── Agent stream event listeners ─────────────────────────────────────────
  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    (async () => {
      unlisteners.push(await listen<string>("agent:chunk", (e) => {
        setMessages(prev => {
          const copy = [...prev];
          const last = copy[copy.length - 1];
          if (last?.streaming) {
            copy[copy.length - 1] = { ...last, content: last.content + e.payload };
          }
          return copy;
        });
      }));

      unlisteners.push(await listen("agent:complete", () => {
        setMessages(prev => {
          const copy = [...prev];
          const last = copy[copy.length - 1];
          if (last?.streaming) copy[copy.length - 1] = { ...last, streaming: false };
          return copy;
        });
        setLoading(false);
      }));

      unlisteners.push(await listen<string>("agent:error", (e) => {
        setMessages(prev => {
          const copy = [...prev];
          const last = copy[copy.length - 1];
          if (last?.streaming) {
            copy[copy.length - 1] = {
              ...last,
              streaming: false,
              content: last.content + `\n\nError: ${e.payload}`,
            };
          }
          return copy;
        });
        setLoading(false);
      }));
    })();

    return () => unlisteners.forEach(u => u());
  }, []);

  // ── Always-on-top toggle ─────────────────────────────────────────────────
  const togglePin = async () => {
    const next = !pinned;
    setPinned(next);
    await invoke("set_always_on_top", { alwaysOnTop: next });
  };

  // ── Send message via daemon (proxied through Tauri commands) ─────────────
  const send = useCallback(async () => {
    const text = input.trim();
    if (!text || loading) return;
    setInput("");
    setLoading(true);

    const userMsg: Message = { role: "user", content: text };
    setMessages(prev => [...prev, userMsg]);

    try {
      // Start agent session via Tauri command (bypasses CORS)
      const sessionId = await invoke<string>("start_agent_session", {
        url: daemonUrl,
        task: text,
        provider,
        model: selectedModel || null,
        token: daemonToken || null,
      });

      // Append placeholder streaming message
      setMessages(prev => [
        ...prev,
        { role: "assistant", content: "", streaming: true },
      ]);

      // Start streaming — Tauri backend reads SSE and emits events
      await invoke("stream_agent", {
        url: daemonUrl,
        sessionId,
        token: daemonToken || null,
      });
    } catch (err) {
      setMessages(prev => [
        ...prev,
        { role: "system", content: `Error: ${String(err)}` },
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
    localStorage.setItem(DAEMON_TOKEN_KEY, daemonToken);
    localStorage.setItem(MODEL_KEY, selectedModel);
    setShowSettings(false);
  };

  // ── Render ───────────────────────────────────────────────────────────────
  return (
    <div className="app">
      {/* Title bar (drag handle for frameless window) */}
      <div className="titlebar" onMouseDown={onDragStart}>
        <span className="titlebar-title">Vibe App</span>
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
            <IconPin active={pinned} />
          </button>
          <button
            className="btn-icon"
            onClick={() => setShowSettings(s => !s)}
            title="Settings"
          >
            <IconSettings />
          </button>
          <button
            className="btn-icon"
            onClick={() => invoke("hide_window")}
            title="Send to tray"
          >
            <IconMinus />
          </button>
        </div>
      </div>

      {/* Model selector bar — shown when settings are closed and models are available */}
      {!showSettings && (selectedModel || availableModels.length > 0) && (
        <div className="model-bar">
          {availableModels.length > 0 ? (
            <select
              value={selectedModel}
              onChange={e => { setSelectedModel(e.target.value); localStorage.setItem(MODEL_KEY, e.target.value); }}
              title="Select model"
            >
              {availableModels.map(m => (
                <option key={m.id} value={m.id}>
                  {m.name || m.id}{m.provider ? ` (${m.provider})` : ""}
                </option>
              ))}
            </select>
          ) : (
            <span style={{ fontSize: 12, color: "var(--text-dim)" }}>{selectedModel || "No model selected"}</span>
          )}
        </div>
      )}

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
            API Token
            <input
              type="password"
              value={daemonToken}
              onChange={e => setDaemonToken(e.target.value)}
              placeholder="Bearer token from vibecli --serve output"
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
          <label>
            Model
            {availableModels.length > 0 ? (
              <select value={selectedModel} onChange={e => setSelectedModel(e.target.value)}>
                {availableModels.map(m => (
                  <option key={m.id} value={m.id}>
                    {m.name || m.id}{m.provider ? ` (${m.provider})` : ""}
                  </option>
                ))}
              </select>
            ) : (
              <input
                value={selectedModel}
                onChange={e => setSelectedModel(e.target.value)}
                placeholder="e.g. llama3.2, gpt-4o, claude-sonnet-4-6"
              />
            )}
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
            <div className="empty-icon"><IconSparkle /></div>
            <div className="empty-title">Vibe App</div>
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
          {loading ? <IconLoader /> : <IconSend />}
        </button>
      </div>
    </div>
  );
}
