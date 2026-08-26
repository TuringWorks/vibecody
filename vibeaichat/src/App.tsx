import { memo, useState, useEffect, useRef, useCallback, useMemo } from "react";
import { SettingsView, type SettingsTab } from "@vibe/shared/settings/SettingsView";
import { Plug } from "lucide-react";
import { visibleAnswer } from "@vibe/shared/lib/thinking";
import { Markdown } from "@vibe/shared/markdown/Markdown";
import { useVoiceInput } from "@vibe/shared/voice/useVoiceInput";
import { VoiceButton } from "@vibe/shared/voice/VoiceButton";
import { useVoiceDuplex } from "@vibe/shared/voice/useVoiceDuplex";
import { DuplexVoiceButton } from "@vibe/shared/voice/DuplexVoiceButton";
import { useVoiceDuplexPreference } from "@vibe/shared/voice/useVoiceDuplexPreference";
import { tauriTranscriber } from "@vibe/shared/voice/transcribers";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";
// The shared settings screens ship their own CSS; it must come after
// App.css, which imports the design-system tokens its var()s consume.
import "@vibe/shared/settings/settings.css";
import "@vibe/shared/markdown/markdown.css";
import "@vibe/shared/voice/voice.css";

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

// ── Message row ───────────────────────────────────────────────────────────────

/**
 * One message bubble, memoized.
 *
 * `agent:chunk` appends to the last message on every streamed chunk, which
 * replaces the `messages` array and re-rendered every bubble in it. Each of
 * those re-renders ran `visibleAnswer` — twice, once for the condition and
 * once for the value — and a full markdown parse, over the whole transcript.
 * The cost of a reply was therefore O(transcript x chunks) and grew for the
 * whole session.
 *
 * Memoized, a chunk re-renders the one bubble it changed. The message objects
 * are already replaced immutably, so identity is a correct staleness check.
 */
const ChatMessage = memo(function ChatMessage({ msg }: { msg: Message }) {
  const answer = useMemo(
    () => (msg.role === "assistant" ? visibleAnswer(msg.content) : ""),
    [msg.role, msg.content],
  );
  return (
    <div className={`msg msg-${msg.role}`}>
      {msg.role === "user" && <div className="msg-label">You</div>}
      {msg.role === "assistant" && <div className="msg-label">AI</div>}
      <div className="msg-content">
        {/* Assistant turns are Markdown — models answer with headings,
            lists, tables and fenced code whatever the question is, and
            showing the source text makes every one of those unreadable.
            User and system messages stay literal: their text is whatever
            was typed, and rendering it would eat characters like `*`.

            `<thinking>` is transport markup, not prose — chat mode strips
            it server-side, this covers everything else. */}
        {msg.role === "assistant" ? (
          answer ? (
            <Markdown text={answer} />
          ) : msg.streaming ? (
            <span className="cursor">▋</span>
          ) : null
        ) : (
          msg.content
        )}
      </div>
    </div>
  );
});

// ── Settings ──────────────────────────────────────────────────────────────────

const DAEMON_URL_KEY   = "vibeaichat_daemon_url";
const PROVIDER_KEY     = "vibeaichat_provider";
const MODEL_KEY        = "vibeaichat_model";
const DEFAULT_URL      = "http://localhost:7878";

/// Pre-encryption key for the daemon bearer token. Read once so an upgrading
/// install keeps working, then removed — see `migrateDaemonToken` below.
const LEGACY_DAEMON_TOKEN_KEY = "vibeaichat_daemon_token";

function loadSetting(key: string, fallback: string): string {
  return localStorage.getItem(key) ?? fallback;
}

/**
 * The bearer token for a remote daemon, from the encrypted ProfileStore.
 *
 * It used to be saved in `localStorage`, which is a plaintext file in the
 * webview profile directory — a token for a machine's entire daemon API, on
 * disk in the clear. A copy left there is migrated into the store and deleted.
 */
async function loadDaemonToken(): Promise<string> {
  const stored = await invoke<string | null>("daemon_token_get");
  if (typeof stored === "string" && stored.length > 0) {
    localStorage.removeItem(LEGACY_DAEMON_TOKEN_KEY);
    return stored;
  }
  const legacy = localStorage.getItem(LEGACY_DAEMON_TOKEN_KEY) ?? "";
  if (!legacy) return "";
  // Move it rather than drop it: the user would otherwise be 401ing against
  // their remote daemon with no hint that we discarded the token.
  await invoke("daemon_token_set", { token: legacy });
  localStorage.removeItem(LEGACY_DAEMON_TOKEN_KEY);
  return legacy;
}

// ── Model catalog — daemon is the single source of truth ─────────────────────
// The daemon's /models endpoint (vibe-ai/src/catalog.rs) returns the full
// catalog for every provider, including ollama local + cloud. VibeAIChat renders
// exactly that and caches the last success, so the picker survives a brief
// disconnect without carrying its own hardcoded list.

const MODELS_CACHE_KEY = "vibeaichat_models_cache";

type ModelRow = { id: string; name?: string; provider?: string };

function readModelsCache(): ModelRow[] {
  try {
    const raw = localStorage.getItem(MODELS_CACHE_KEY);
    const parsed = raw ? (JSON.parse(raw) as unknown) : [];
    return Array.isArray(parsed) ? (parsed as ModelRow[]) : [];
  } catch {
    return [];
  }
}

// Friendly labels for the provider dropdown; the providers themselves are
// derived from the daemon's model list. Unknown ids fall back to the raw id.
const PROVIDER_LABELS: Record<string, string> = {
  claude: "Claude",
  openai: "OpenAI",
  ollama: "Ollama (local + cloud)",
  vllm: "vLLM (local)",
  lmstudio: "LM Studio (local)",
  gemini: "Gemini",
  grok: "Grok (xAI)",
  groq: "Groq",
  mistral: "Mistral",
  deepseek: "DeepSeek",
  openrouter: "OpenRouter",
  zhipu: "Zhipu (GLM)",
  cerebras: "Cerebras",
  perplexity: "Perplexity",
  together: "Together",
  fireworks: "Fireworks",
  minimax: "MiniMax",
  sambanova: "SambaNova",
  poolside: "Poolside AI",
  azure_openai: "Azure OpenAI",
  bedrock: "AWS Bedrock",
  copilot: "GitHub Copilot",
  "vibecli-mistralrs": "VibeCLI (mistral.rs)",
};

// ── Main App ──────────────────────────────────────────────────────────────────

export default function App() {
  const [messages, setMessages]     = useState<Message[]>([]);
  const [input, setInput]           = useState("");
  const [loading, setLoading]       = useState(false);
  const [daemonUrl, setDaemonUrl]   = useState(() => loadSetting(DAEMON_URL_KEY, DEFAULT_URL));
  const [provider, setProvider]     = useState(() => loadSetting(PROVIDER_KEY, "claude"));
  // Hydrated from the encrypted store below, never from localStorage.
  const [daemonToken, setDaemonToken] = useState("");
  const [tokenError, setTokenError] = useState<string | null>(null);
  const [daemonOk, setDaemonOk]     = useState<boolean | null>(null);
  const [pinned, setPinned]         = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [availableModels, setAvailableModels] = useState<ModelRow[]>(readModelsCache);
  const [selectedModel, setSelectedModel] = useState(() => loadSetting(MODEL_KEY, ""));

  // Full-duplex conversation. Push-to-talk (`useVoiceInput`, below) stays: it
  // is the right control for dictating a long prompt, and it is the only one
  // that works on a host without echo cancellation.
  const voicePref = useVoiceDuplexPreference();
  const duplex = useVoiceDuplex({
    enabled: voicePref.enabled,
    daemonUrl,
    token: daemonToken,
    provider,
    model: selectedModel,
    language: "en",
    onTurn: turn =>
      setMessages(m => [...m, { role: turn.role === "user" ? "user" : "assistant", content: turn.text }]),
  });

  const bottomRef   = useRef<HTMLDivElement>(null);
  const inputRef    = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    let cancelled = false;
    loadDaemonToken()
      .then(token => { if (!cancelled && token) setDaemonToken(token); })
      .catch(e => { if (!cancelled) setTokenError(`Could not read the saved API token: ${e}`); });
    return () => { cancelled = true; };
  }, []);

  // ── Daemon health-check + model discovery ───────────────────────────────
  useEffect(() => {
    const check = async () => {
      try {
        await invoke("check_daemon", { url: daemonUrl });
        setDaemonOk(true);
        // The daemon is the single source of truth for the model catalog.
        // Keep addressable rows (drop the synthetic active entry), cache them
        // so the picker survives a brief disconnect.
        try {
          const models = await invoke<ModelRow[]>("list_daemon_models", { url: daemonUrl });
          const named = models.filter(m => !!m.name);
          setAvailableModels(named);
          localStorage.setItem(MODELS_CACHE_KEY, JSON.stringify(named));
        } catch { /* daemon may not support /models yet — keep cache */ }
      } catch {
        setDaemonOk(false);
      }
    };
    check();
    const id = setInterval(check, 30_000);
    return () => clearInterval(id);
  }, [daemonUrl]);

  // ── Providers derived from the daemon's catalog ─────────────────────────
  const providerOptions = useMemo(() => {
    const present = Array.from(
      new Set(availableModels.map(m => m.provider).filter((p): p is string => !!p)),
    );
    const ids = present.length > 0 ? present : Object.keys(PROVIDER_LABELS);
    return ids.map(id => ({ id, label: PROVIDER_LABELS[id] ?? id }));
  }, [availableModels]);

  // ── Models filtered by selected provider ────────────────────────────────
  const filteredModels: Array<{ id: string; name: string; provider?: string }> =
    availableModels
      .filter(m => (m.provider ?? "") === provider && !!m.name)
      .map(m => ({ id: m.id, name: m.name as string, provider: m.provider }));

  // Auto-select first model when provider changes and current selection doesn't match
  useEffect(() => {
    if (filteredModels.length > 0 && !filteredModels.some(m => m.name === selectedModel)) {
      setSelectedModel(filteredModels[0].name);
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
        // VibeAIChat is an assistant, not a task runner. The agent loop's system
        // prompt orders the model to answer *only* with a tool call, so "hi"
        // produced pages of the model arguing with that rule instead of a
        // greeting. Chat mode is a single reply, no tools.
        mode: "chat",
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

  // ── Voice input ───────────────────────────────────────────────────────────
  // Dictation extends the draft instead of replacing it — Web Speech delivers
  // an utterance as several final chunks, and each one must append.
  const appendTranscript = useCallback((chunk: string) => {
    const trimmed = chunk.trim();
    if (!trimmed) return;
    setInput(prev => (prev ? `${prev.replace(/\s+$/, "")} ${trimmed}` : trimmed));
  }, []);
  const transcribe = useMemo(
    () => tauriTranscriber(daemonUrl, { token: daemonToken || undefined }),
    [daemonUrl, daemonToken],
  );
  const voice = useVoiceInput({ onTranscript: appendTranscript, transcribe });

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

  // Daemon URL / token / provider / model belong to this shell, not to every
  // shell, so they ride along as an extra tab rather than being pushed into the
  // shared settings screens.
  const connectionTab: SettingsTab = {
    id: "connection",
    label: "Connection",
    icon: Plug,
    render: () => (
      <div className="vx-set-section">
        <h3 className="vx-set-h">Daemon</h3>
        <label className="settings-field">
          Daemon URL
          <input
            value={daemonUrl}
            onChange={e => setDaemonUrl(e.target.value)}
            placeholder="http://localhost:7878"
          />
        </label>
        <label className="settings-field">
          API token — optional
          <input
            type="password"
            value={daemonToken}
            onChange={e => setDaemonToken(e.target.value)}
            placeholder="read from ~/.vibecli/daemon.token when blank"
          />
        </label>
        {tokenError && <p className="settings-hint" role="alert">{tokenError}</p>}
        <p className="settings-hint">
          Leave the token blank unless you are pointing at a daemon on another
          machine. The token is regenerated every time the daemon starts, so a
          value pasted here goes stale on the next restart; blank reads the
          current one from disk.
        </p>

        <h3 className="vx-set-h">Model</h3>
        <label className="settings-field">
          Provider
          <select value={provider} onChange={e => setProvider(e.target.value)}>
            {providerOptions.map(p => (
              <option key={p.id} value={p.id}>{p.label}</option>
            ))}
          </select>
        </label>
        <label className="settings-field">
          Model
          {filteredModels.length > 0 ? (
            <select value={selectedModel} onChange={e => setSelectedModel(e.target.value)}>
              {filteredModels.map(m => (
                <option key={m.id} value={m.name}>
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
        </div>
      </div>
    ),
  };

  // ── Save settings ────────────────────────────────────────────────────────
  const saveSettings = async () => {
    localStorage.setItem(DAEMON_URL_KEY, daemonUrl);
    localStorage.setItem(PROVIDER_KEY, provider);
    localStorage.setItem(MODEL_KEY, selectedModel);
    // The token is the one secret here, so it goes to the encrypted store — and
    // the panel stays open on a failure rather than reporting a save that did
    // not happen.
    try {
      await invoke("daemon_token_set", { token: daemonToken });
      setTokenError(null);
    } catch (e) {
      setTokenError(`Could not save the API token: ${e}`);
      return;
    }
    setShowSettings(false);
  };

  // ── Render ───────────────────────────────────────────────────────────────
  return (
    <div className="app">
      {/* Title bar (drag handle for frameless window) */}
      <div className="titlebar" onMouseDown={onDragStart}>
        <span className="titlebar-title">VibeAIChat</span>
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

      {/* Model selector bar — shown when settings are closed. Provider + model
          pickers live here so you can switch between Claude, Ollama (local +
          cloud), etc. without opening Settings. Both are populated from the
          daemon's /models catalog (the single source of truth). */}
      {!showSettings && (selectedModel || availableModels.length > 0) && (
        <div className="model-bar">
          {availableModels.length > 0 ? (
            <>
              <select
                value={provider}
                onChange={e => { setProvider(e.target.value); localStorage.setItem(PROVIDER_KEY, e.target.value); }}
                title="Select provider"
              >
                {providerOptions.map(p => (
                  <option key={p.id} value={p.id}>{p.label}</option>
                ))}
              </select>
              {filteredModels.length > 0 ? (
                <select
                  value={selectedModel}
                  onChange={e => { setSelectedModel(e.target.value); localStorage.setItem(MODEL_KEY, e.target.value); }}
                  title="Select model"
                >
                  {filteredModels.map(m => (
                    <option key={m.id} value={m.name}>
                      {m.name || m.id}
                    </option>
                  ))}
                </select>
              ) : (
                <span style={{ fontSize: 12, color: "var(--text-dim)" }}>No models for this provider</span>
              )}
            </>
          ) : (
            <span style={{ fontSize: 12, color: "var(--text-dim)" }}>{selectedModel || "No model selected"}</span>
          )}
        </div>
      )}

      {/* Settings panel */}
      {showSettings && (
        <SettingsView
          onClose={() => setShowSettings(false)}
          extraTabs={[connectionTab]}
        />
      )}

      {/* Message list */}
      <div className="messages">
        {messages.length === 0 && (
          <div className="empty-state">
            <div className="empty-icon"><IconSparkle /></div>
            <div className="empty-title">VibeAIChat</div>
            <div className="empty-subtitle">
              {daemonOk === false
                ? "Start the daemon: vibecli --serve"
                : "Ask anything"}
            </div>
          </div>
        )}
        {messages.map((msg, i) => (
          <ChatMessage key={i} msg={msg} />
        ))}
        <div ref={bottomRef} />
      </div>

      {/* Input area */}
      {voice.interimText && (
        <div className="vx-voice-interim" aria-live="polite">
          {voice.interimText}
        </div>
      )}
      {voice.error && (
        <div className="vx-voice-error" role="status">
          {voice.error}
        </div>
      )}
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
        <VoiceButton voice={voice} disabled={loading} />
        <DuplexVoiceButton
          state={duplex.state}
          enabled={voicePref.enabled}
          onEnabledChange={voicePref.setEnabled}
          active={duplex.active}
          supported={duplex.supported && daemonOk === true}
          onStart={duplex.start}
          onStop={duplex.stop}
          unsupportedHint={
            daemonOk === true
              ? "This webview cannot capture audio"
              : "Connect to a daemon to start a voice conversation"
          }
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
