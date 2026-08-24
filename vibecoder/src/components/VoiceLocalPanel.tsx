import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useVoiceInput } from "@vibe/shared/voice/useVoiceInput";
import { tauriTranscriber } from "@vibe/shared/voice/transcribers";
import { ExperimentalBadge } from "./ExperimentalBadge";

/** One whisper model as `voice_list_models` reports it. */
interface VoiceModel {
  id: string;
  label: string;
  /** Size of the GGML file that gets downloaded, not the checkpoint's parameter size. */
  size_mb: number;
  downloaded: boolean;
  selected: boolean;
  path: string | null;
}

/** What the daemon's voice stack can do on this machine (`GET /voice/status`). */
interface VoiceStatus {
  cloud_stt_configured: boolean;
  local_model: string;
  local_model_downloaded: boolean;
  prefer_local: boolean;
  language: string;
  whisper_cpp_installed: boolean;
  whisper_python_installed: boolean;
  ffmpeg_installed: boolean;
}

interface VoiceSettings {
  local_model: string;
  language: string;
  prefer_local: boolean;
}

/** A finished transcript, kept for the History tab. */
interface HistoryEntry {
  text: string;
  time: string;
}

/** Bytes seen so far for a model that is downloading right now. */
interface DownloadProgress {
  downloaded_bytes: number;
  /** `0` when the server sent no Content-Length — an unknown total, not 0%. */
  total_bytes: number;
}

const LANGUAGES = ["en", "es", "fr", "de", "ja", "zh", "ko", "ru"];

const TABS = ["record", "models", "history", "config"] as const;
type Tab = (typeof TABS)[number];

/** `1234567890` → `1.2 GB`. */
function formatBytes(bytes: number): string {
  return bytes >= 1e9 ? `${(bytes / 1e9).toFixed(1)} GB` : `${Math.round(bytes / 1e6)} MB`;
}

/**
 * What is missing before a recording could be transcribed at all, in the order
 * the user should fix it. Empty means at least one engine can run.
 *
 * Deliberately derived from the daemon's own probe rather than guessed from the
 * model list: a downloaded model with no whisper runtime to execute it is not a
 * usable engine, and saying "ready" then would be a lie the first recording
 * exposes.
 */
function blockers(status: VoiceStatus | null): string[] {
  if (!status) return [];
  const hasRuntime = status.whisper_cpp_installed || status.whisper_python_installed;
  const localReady = status.local_model_downloaded && hasRuntime && status.ffmpeg_installed;
  if (status.cloud_stt_configured && !status.prefer_local) return [];
  if (localReady) return [];

  const missing: string[] = [];
  if (!status.local_model_downloaded) {
    missing.push(`the “${status.local_model}” model is not downloaded — pull it on the Models tab`);
  }
  if (!hasRuntime) {
    missing.push("no whisper runtime on PATH — install one with `brew install whisper-cpp`");
  }
  if (!status.ffmpeg_installed) {
    missing.push("ffmpeg is missing — it converts the browser's WebM clip to WAV (`brew install ffmpeg`)");
  }
  if (status.cloud_stt_configured && status.prefer_local) {
    missing.push("prefer-local is on, so the configured Groq key is not being used");
  }
  return missing;
}

export function VoiceLocalPanel() {
  const [tab, setTab] = useState<Tab>("record");
  const [models, setModels] = useState<VoiceModel[]>([]);
  const [status, setStatus] = useState<VoiceStatus | null>(null);
  const [statusError, setStatusError] = useState<string | null>(null);
  const [settings, setSettings] = useState<VoiceSettings>({
    local_model: "base",
    language: "en",
    prefer_local: false,
  });
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [loadingModels, setLoadingModels] = useState(true);
  const [modelsError, setModelsError] = useState<string | null>(null);
  const [progress, setProgress] = useState<Record<string, DownloadProgress>>({});
  const [busyModel, setBusyModel] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [transcript, setTranscript] = useState("");

  const refreshModels = useCallback(async () => {
    try {
      setModels(await invoke<VoiceModel[]>("voice_list_models"));
      setModelsError(null);
    } catch (e) {
      setModelsError(String(e));
    }
  }, []);

  const refreshStatus = useCallback(async () => {
    try {
      setStatus(await invoke<VoiceStatus>("voice_status", { url: null, token: null }));
      setStatusError(null);
    } catch (e) {
      // The daemon being unreachable is the single most common reason voice
      // does nothing, so it gets its own message instead of a silent console
      // line — that is how this panel hid a 100% failure before.
      setStatus(null);
      setStatusError(String(e));
    }
  }, []);

  useEffect(() => {
    let live = true;
    (async () => {
      const loaded = await invoke<VoiceSettings>("voice_get_settings").catch(() => null);
      if (live && loaded) setSettings(loaded);
      await Promise.all([refreshModels(), refreshStatus()]);
      if (live) setLoadingModels(false);
    })();
    return () => {
      live = false;
    };
  }, [refreshModels, refreshStatus]);

  // Download progress arrives as events because a 3 GB pull is minutes long and
  // the command only resolves at the end.
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let live = true;
    listen<{ id: string; downloaded_bytes: number; total_bytes: number }>(
      "voice-model-progress",
      (event) => {
        const { id, downloaded_bytes, total_bytes } = event.payload;
        setProgress((prev) => ({ ...prev, [id]: { downloaded_bytes, total_bytes } }));
      },
    ).then((fn) => {
      if (live) unlisten = fn;
      else fn();
    });
    return () => {
      live = false;
      unlisten?.();
    };
  }, []);

  const appendTranscript = useCallback((text: string) => {
    const trimmed = text.trim();
    if (!trimmed) return;
    setTranscript((prev) => (prev ? `${prev} ${trimmed}` : trimmed));
    setHistory((prev) => [
      { text: trimmed, time: new Date().toLocaleString() },
      ...prev,
    ]);
  }, []);

  // Recording happens in the webview (MediaRecorder); the clip goes to the
  // daemon's /voice/transcribe through the shared `transcribe_audio` command,
  // which is the same path AIChat's mic button uses.
  const transcribe = useMemo(
    () => tauriTranscriber(undefined, { preferLocal: settings.prefer_local, language: settings.language }),
    [settings.prefer_local, settings.language],
  );
  const {
    toggle,
    supported,
    isListening,
    isTranscribing,
    interimText,
    error: voiceError,
    clearError,
  } = useVoiceInput({ onTranscript: appendTranscript, transcribe, lang: settings.language });

  const saveSettings = useCallback(
    async (next: Partial<VoiceSettings>) => {
      const merged = { ...settings, ...next };
      setSettings(merged);
      try {
        await invoke("voice_set_settings", {
          language: next.language ?? null,
          preferLocal: next.prefer_local ?? null,
        });
        setActionError(null);
        await refreshStatus();
      } catch (e) {
        setActionError(String(e));
      }
    },
    [settings, refreshStatus],
  );

  const download = useCallback(
    async (model: VoiceModel) => {
      setBusyModel(model.id);
      setActionError(null);
      setProgress((prev) => ({ ...prev, [model.id]: { downloaded_bytes: 0, total_bytes: 0 } }));
      try {
        await invoke("voice_download_model", { id: model.id });
        await Promise.all([refreshModels(), refreshStatus()]);
      } catch (e) {
        setActionError(`Could not download ${model.label}: ${e}`);
      } finally {
        setBusyModel(null);
        setProgress((prev) => {
          const { [model.id]: _dropped, ...rest } = prev;
          return rest;
        });
      }
    },
    [refreshModels, refreshStatus],
  );

  const select = useCallback(
    async (model: VoiceModel) => {
      setBusyModel(model.id);
      setActionError(null);
      try {
        await invoke("voice_select_model", { id: model.id });
        setSettings((prev) => ({ ...prev, local_model: model.id }));
        await Promise.all([refreshModels(), refreshStatus()]);
      } catch (e) {
        setActionError(`Could not select ${model.label}: ${e}`);
      } finally {
        setBusyModel(null);
      }
    },
    [refreshModels, refreshStatus],
  );

  const remove = useCallback(
    async (model: VoiceModel) => {
      setBusyModel(model.id);
      setActionError(null);
      try {
        await invoke("voice_delete_model", { id: model.id });
        await Promise.all([refreshModels(), refreshStatus()]);
      } catch (e) {
        setActionError(`Could not delete ${model.label}: ${e}`);
      } finally {
        setBusyModel(null);
      }
    },
    [refreshModels, refreshStatus],
  );

  const setup = blockers(status);
  const recordLabel = isListening
    ? "Recording… click to stop"
    : isTranscribing
      ? "Transcribing…"
      : supported
        ? "Click to start recording"
        : "This webview has no microphone available";

  return (
    <div className="panel-container">
      <ExperimentalBadge
        as="banner"
        feature="Local voice"
        tooltip="Whisper / faster-whisper local STT. Models must be pulled before first use; the surface and model list may change."
      />
      <div className="panel-tab-bar">
        {TABS.map((t) => (
          <button key={t} className={`panel-tab ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      <div className="panel-body">
        {statusError && (
          <div className="panel-card" style={{ borderColor: "var(--error-color)" }}>
            <div style={{ fontSize: "var(--font-size-md)", color: "var(--error-color)", fontWeight: 600 }}>
              Cannot reach the daemon
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginTop: 4 }}>
              {statusError}
            </div>
          </div>
        )}
        {setup.length > 0 && (
          <div className="panel-card" style={{ borderColor: "var(--warning-color)" }}>
            <div style={{ fontSize: "var(--font-size-md)", color: "var(--warning-color)", fontWeight: 600 }}>
              Transcription cannot run yet
            </div>
            <ul style={{ margin: "6px 0 0", paddingLeft: 18, fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
              {setup.map((s) => (
                <li key={s}>{s}</li>
              ))}
            </ul>
          </div>
        )}

        {tab === "record" && (
          <div style={{ textAlign: "center" }}>
            <button
              className="panel-btn"
              onClick={() => {
                clearError();
                toggle();
              }}
              disabled={!supported || isTranscribing}
              aria-label={isListening ? "Stop recording" : "Start recording"}
              aria-pressed={isListening}
              title={supported ? undefined : "No microphone API in this webview"}
              style={{
                width: 72,
                height: 72,
                borderRadius: "50%",
                border: "none",
                cursor: supported && !isTranscribing ? "pointer" : "not-allowed",
                opacity: supported ? 1 : 0.5,
                background: "var(--error-color)",
                boxShadow: isListening
                  ? "0 0 0 8px color-mix(in srgb, var(--error-color) 25%, transparent)"
                  : "none",
                marginBottom: 20,
                transition: "box-shadow 0.3s",
              }}
            >
              <div
                style={{
                  width: 24,
                  height: 24,
                  borderRadius: isListening ? 4 : 12,
                  background: "var(--btn-primary-fg, #fff)",
                  margin: "auto",
                }}
              />
            </button>
            <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 16 }}>
              {recordLabel}
            </div>
            {voiceError && (
              <div className="panel-card" style={{ textAlign: "left", borderColor: "var(--error-color)" }}>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--error-color)" }}>{voiceError}</div>
              </div>
            )}
            <div className="panel-card" style={{ minHeight: 60, textAlign: "left" }}>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: 4 }}>
                Transcription
              </div>
              <div style={{ fontSize: "var(--font-size-lg)" }}>
                {transcript || interimText || "No transcription yet"}
              </div>
              {interimText && transcript && (
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginTop: 4 }}>
                  {interimText}
                </div>
              )}
            </div>
            {transcript && (
              <button className="panel-btn panel-btn-secondary" onClick={() => setTranscript("")}>
                Clear
              </button>
            )}
          </div>
        )}

        {tab === "models" && (
          <div>
            {actionError && (
              <div className="panel-card" style={{ borderColor: "var(--error-color)" }}>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--error-color)" }}>{actionError}</div>
              </div>
            )}
            {loadingModels && <div className="panel-loading">Loading models...</div>}
            {!loadingModels && modelsError && (
              <div className="panel-card" style={{ borderColor: "var(--error-color)" }}>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--error-color)" }}>
                  Could not read the model directory: {modelsError}
                </div>
              </div>
            )}
            {!loadingModels && !modelsError && models.length === 0 && (
              <div className="panel-empty">No voice models available.</div>
            )}
            {models.map((m) => {
              const live = progress[m.id];
              const downloading = busyModel === m.id && !m.downloaded;
              const pct = live && live.total_bytes > 0
                ? Math.round((live.downloaded_bytes / live.total_bytes) * 100)
                : null;
              return (
                <div key={m.id} className="panel-card">
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8 }}>
                    <div>
                      <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{m.label}</span>
                      <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginLeft: 8 }}>
                        {m.size_mb} MB
                      </span>
                      {m.selected && (
                        <span style={{ marginLeft: 8, fontSize: "var(--font-size-sm)", color: "var(--success-color)", fontWeight: 600 }}>
                          SELECTED
                        </span>
                      )}
                    </div>
                    <div style={{ display: "flex", gap: 6 }}>
                      {m.downloaded ? (
                        <>
                          <button
                            className="panel-btn panel-btn-primary"
                            disabled={m.selected || busyModel === m.id}
                            style={{ background: m.selected ? "var(--success-color)" : undefined }}
                            onClick={() => select(m)}
                          >
                            {m.selected ? "Active" : "Select"}
                          </button>
                          <button
                            className="panel-btn panel-btn-secondary"
                            disabled={busyModel === m.id}
                            onClick={() => remove(m)}
                          >
                            Delete
                          </button>
                        </>
                      ) : (
                        <button
                          className="panel-btn panel-btn-secondary"
                          disabled={busyModel !== null}
                          onClick={() => download(m)}
                        >
                          {downloading ? "Downloading…" : "Download"}
                        </button>
                      )}
                    </div>
                  </div>
                  {downloading && (
                    <div style={{ marginTop: 8 }}>
                      <div style={{ height: 6, borderRadius: "var(--radius-xs-plus)", background: "var(--border-color)" }}>
                        <div
                          style={{
                            width: pct === null ? "100%" : `${pct}%`,
                            height: 6,
                            borderRadius: "var(--radius-xs-plus)",
                            background: pct === null ? "var(--text-muted)" : "var(--accent-color, var(--success-color))",
                            transition: "width 0.2s",
                          }}
                        />
                      </div>
                      <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 4 }}>
                        {live
                          ? pct === null
                            ? `${formatBytes(live.downloaded_bytes)} downloaded (total size unknown)`
                            : `${pct}% — ${formatBytes(live.downloaded_bytes)} / ${formatBytes(live.total_bytes)}`
                          : "Starting…"}
                      </div>
                    </div>
                  )}
                  {m.downloaded && m.path && (
                    <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-muted)", marginTop: 6, wordBreak: "break-all" }}>
                      {m.path}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}

        {tab === "history" && (
          <div>
            {history.length === 0 && (
              <div className="panel-empty">No transcription history yet. Record something to get started.</div>
            )}
            {history.map((h, i) => (
              <div key={`${h.time}-${i}`} className="panel-card">
                <div style={{ fontSize: "var(--font-size-md)", marginBottom: 4 }}>{h.text}</div>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{h.time}</div>
              </div>
            ))}
          </div>
        )}

        {tab === "config" && (
          <div>
            {actionError && (
              <div className="panel-card" style={{ borderColor: "var(--error-color)" }}>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--error-color)" }}>{actionError}</div>
              </div>
            )}
            <div className="panel-card">
              <div className="panel-label">Language</div>
              <select
                value={settings.language}
                onChange={(e) => saveSettings({ language: e.target.value })}
                style={{
                  padding: "4px 8px",
                  borderRadius: "var(--radius-xs-plus)",
                  border: "1px solid var(--border-color)",
                  background: "var(--bg-primary)",
                  color: "var(--text-primary)",
                  fontSize: "var(--font-size-md)",
                }}
              >
                {LANGUAGES.map((l) => (
                  <option key={l} value={l}>
                    {l}
                  </option>
                ))}
              </select>
            </div>
            <div className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>Keep audio on this machine</div>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
                  Force the local whisper model even when a cloud key is configured.
                </div>
              </div>
              <button
                className="panel-btn panel-btn-primary"
                style={{ background: settings.prefer_local ? "var(--success-color)" : "var(--border-color)" }}
                onClick={() => saveSettings({ prefer_local: !settings.prefer_local })}
              >
                {settings.prefer_local ? "ON" : "OFF"}
              </button>
            </div>
            {status && (
              <div className="panel-card">
                <div className="panel-label">Engine status</div>
                <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", display: "grid", gap: 2 }}>
                  <span>Cloud STT key: {status.cloud_stt_configured ? "configured" : "not configured"}</span>
                  <span>Selected model: {status.local_model} — {status.local_model_downloaded ? "downloaded" : "not downloaded"}</span>
                  <span>whisper runtime: {status.whisper_cpp_installed ? "whisper-cpp" : status.whisper_python_installed ? "python whisper" : "none on PATH"}</span>
                  <span>ffmpeg: {status.ffmpeg_installed ? "present" : "missing"}</span>
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
