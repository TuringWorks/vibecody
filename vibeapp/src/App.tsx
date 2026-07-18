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

// ── Static model catalog (Option B) ──────────────────────────────────────────
// The daemon's /models endpoint only reports what a local runtime advertises
// (primarily ollama's pulled models) — never ollama *-cloud models nor the
// cloud providers' catalogs, and nothing at all while the daemon is offline.
// So VibeApp carries a full static catalog and shows it regardless, unioning
// the daemon's live ollama list on top when it's reachable. Mirrors the
// desktop registry (vibeui/src/hooks/useModelRegistry.ts).

// Ollama Cloud / Turbo (*-cloud) — datacenter-hosted, never in a local /api/tags.
const OLLAMA_CLOUD_MODELS: string[] = [
  "glm-5.2:cloud",
  "deepseek-v3.1:671b-cloud",
  "kimi-k2:1t-cloud",
  "gpt-oss:120b-cloud",
  "gpt-oss:20b-cloud",
  "glm-4.6:cloud",
  "minimax-m2:cloud",
];

// Ollama chat catalog (cloud rows first, then pull-able local models).
const OLLAMA_CHAT_MODELS: string[] = [
  ...OLLAMA_CLOUD_MODELS,
  "devstral-2",
  "devstral-small-2",
  "nemotron-3-super",
  "cogito-2.1",
  "gemma4",
  "ministral-3",
  "qwen3-coder",
  "qwen3.6",
  "qwen3.5",
  "qwen3",
  "deepseek-v4-pro",
  "deepseek-v4-flash",
  "deepseek-v3.2",
  "deepseek-r1",
  "llama4",
  "llama3.3",
  "llama3.2",
  "gemma3",
  "phi4",
  "phi4-mini",
  "mistral-large-3",
  "mistral-small3.2",
  "glm-5.1",
  "glm-5",
  "codellama",
  "codegemma",
  "starcoder2",
];

// Provider ids the picker offers (sent verbatim to the daemon, which supports
// all of them). Order = display order.
const PROVIDER_OPTIONS: Array<{ id: string; label: string }> = [
  { id: "claude", label: "Claude" },
  { id: "openai", label: "OpenAI" },
  { id: "ollama", label: "Ollama (local + cloud)" },
  { id: "gemini", label: "Gemini" },
  { id: "grok", label: "Grok (xAI)" },
  { id: "groq", label: "Groq" },
  { id: "mistral", label: "Mistral" },
  { id: "deepseek", label: "DeepSeek" },
  { id: "openrouter", label: "OpenRouter" },
  { id: "zhipu", label: "Zhipu (GLM)" },
  { id: "cerebras", label: "Cerebras" },
  { id: "perplexity", label: "Perplexity" },
  { id: "together", label: "Together" },
  { id: "fireworks", label: "Fireworks" },
  { id: "minimax", label: "MiniMax" },
  { id: "sambanova", label: "SambaNova" },
  { id: "azure_openai", label: "Azure OpenAI" },
];

// Build {id,name} rows from bare model ids (name = id unless overridden).
const asRows = (ids: string[]): Array<{ id: string; name: string }> =>
  ids.map((id) => ({ id, name: id }));

const PROVIDER_MODELS: Record<string, Array<{ id: string; name: string }>> = {
  claude: [
    { id: "claude-opus-4-8", name: "Claude Opus 4.8" },
    { id: "claude-opus-4-7", name: "Claude Opus 4.7" },
    { id: "claude-opus-4-6", name: "Claude Opus 4.6" },
    { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6" },
    { id: "claude-haiku-4-5-20251001", name: "Claude Haiku 4.5" },
    { id: "claude-3-5-sonnet-latest", name: "Claude 3.5 Sonnet" },
  ],
  openai: asRows([
    "gpt-5.5",
    "gpt-5.4",
    "gpt-5.3-codex",
    "gpt-5",
    "gpt-4o",
    "gpt-4o-mini",
    "o4-mini",
    "o3",
    "gpt-4.1",
    "gpt-4.1-mini",
  ]),
  gemini: asRows([
    "gemini-3.5-pro",
    "gemini-3.5-flash",
    "gemini-3.1-pro",
    "gemini-3-pro",
    "gemini-2.5-pro",
    "gemini-2.5-flash",
  ]),
  grok: asRows(["grok-3", "grok-3-mini", "grok-2"]),
  groq: asRows(["llama-3.3-70b-versatile", "llama-3.1-8b-instant", "mixtral-8x7b-32768", "gemma2-9b-it"]),
  mistral: asRows(["mistral-large-latest", "mistral-medium-latest", "mistral-small-latest", "codestral-latest"]),
  deepseek: asRows(["deepseek-v4", "deepseek-v4-flash", "deepseek-chat", "deepseek-reasoner", "deepseek-coder"]),
  openrouter: asRows([
    "moonshotai/kimi-k2.7-code",
    "z-ai/glm-5.2",
    "qwen/qwen3.6-coder",
    "deepseek/deepseek-v4",
    "anthropic/claude-3.5-sonnet",
    "openai/gpt-4o",
  ]),
  zhipu: asRows(["glm-5.2", "glm-5.1", "glm-4-plus", "glm-4-flash"]),
  cerebras: asRows(["llama-3.3-70b", "llama-3.1-8b"]),
  perplexity: asRows(["sonar-pro", "sonar", "sonar-reasoning"]),
  together: asRows(["meta-llama/Llama-3.3-70B-Instruct", "mistralai/Mixtral-8x7B-Instruct-v0.1"]),
  fireworks: asRows([
    "accounts/fireworks/models/llama-v3p3-70b-instruct",
    "accounts/fireworks/models/mixtral-8x7b-instruct",
  ]),
  minimax: asRows(["MiniMax-M3", "abab6.5s-chat"]),
  sambanova: asRows(["Meta-Llama-3.3-70B-Instruct"]),
  azure_openai: asRows(["gpt-4o", "gpt-4-turbo"]),
  ollama: asRows(OLLAMA_CHAT_MODELS),
};

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
        // Fetch available models from daemon (primarily Ollama)
        try {
          const models = await invoke<Array<{ id: string; name?: string; provider?: string }>>(
            "list_daemon_models", { url: daemonUrl }
          );
          setAvailableModels(models);
        } catch { /* daemon may not support /models yet */ }
      } catch {
        setDaemonOk(false);
      }
    };
    check();
    const id = setInterval(check, 30_000);
    return () => clearInterval(id);
  }, [daemonUrl]);

  // ── Models filtered by selected provider ────────────────────────────────
  const filteredModels: Array<{ id: string; name: string; provider?: string }> = (() => {
    if (provider === "ollama") {
      // Live local models the daemon actually reports (real, installed) …
      const live = availableModels
        .filter(m => !m.provider || m.provider === "ollama")
        .map(m => ({ id: m.id, name: m.name || m.id, provider: "ollama" as const }));
      // … unioned with the static catalog (cloud + pull-able), so cloud models
      // and the full library are always selectable, daemon up or down. Dedupe
      // by the tag-normalised name (daemon reports "llama3.2:latest").
      const norm = (n: string) => n.replace(/:latest$/, "");
      const seen = new Set(live.map(m => norm(m.name)));
      const staticRows = (PROVIDER_MODELS.ollama ?? [])
        .filter(m => !seen.has(norm(m.name)))
        .map(m => ({ id: m.id, name: m.name, provider: "ollama" as const }));
      return [...live, ...staticRows];
    }
    // Cloud providers: static catalog.
    return PROVIDER_MODELS[provider] ?? [];
  })();

  // Auto-select first model when provider changes and current selection doesn't match
  useEffect(() => {
    if (filteredModels.length > 0 && !filteredModels.some(m => m.id === selectedModel)) {
      setSelectedModel(filteredModels[0].id);
    }
  }, [provider, filteredModels.length]); // eslint-disable-line react-hooks/exhaustive-deps

  // ── Auto-scroll ──────────────────────────────────────────────────────────
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  // ── Agent stream event listeners ─────────────────────────────────────────
  useEffect(() => {
    let cancelled = false;
    const unlisteners: Array<() => void> = [];

    (async () => {
      const u1 = await listen<string>("agent:chunk", (e) => {
        if (cancelled) return;
        setMessages(prev => {
          const copy = [...prev];
          const last = copy[copy.length - 1];
          if (last?.streaming) {
            copy[copy.length - 1] = { ...last, content: last.content + e.payload };
          }
          return copy;
        });
      });
      if (cancelled) { u1(); return; }
      unlisteners.push(u1);

      const u2 = await listen("agent:complete", () => {
        if (cancelled) return;
        setMessages(prev => {
          const copy = [...prev];
          const last = copy[copy.length - 1];
          if (last?.streaming) copy[copy.length - 1] = { ...last, streaming: false };
          return copy;
        });
        setLoading(false);
      });
      if (cancelled) { u2(); return; }
      unlisteners.push(u2);

      const u3 = await listen<string>("agent:error", (e) => {
        if (cancelled) return;
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
      });
      if (cancelled) { u3(); return; }
      unlisteners.push(u3);
    })();

    return () => {
      cancelled = true;
      unlisteners.forEach(u => u());
    };
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
  }, [input, loading, daemonUrl, provider, selectedModel, daemonToken]);

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
      {!showSettings && (selectedModel || filteredModels.length > 0) && (
        <div className="model-bar">
          {filteredModels.length > 0 ? (
            <select
              value={selectedModel}
              onChange={e => { setSelectedModel(e.target.value); localStorage.setItem(MODEL_KEY, e.target.value); }}
              title="Select model"
            >
              {filteredModels.map(m => (
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
              {PROVIDER_OPTIONS.map(p => (
                <option key={p.id} value={p.id}>{p.label}</option>
              ))}
            </select>
          </label>
          <label>
            Model
            {filteredModels.length > 0 ? (
              <select value={selectedModel} onChange={e => setSelectedModel(e.target.value)}>
                {filteredModels.map(m => (
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
