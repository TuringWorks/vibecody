import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, RefreshCw, Play } from "lucide-react";
import { PROVIDER_DEFAULT_MODEL, useModelRegistry } from "../hooks/useModelRegistry";

const DEFAULT_PROFILE = "default";
const PANEL_ID = "model-manager";

interface ModelResponse {
  provider: string;
  model: string;
  content: string;
  duration_ms: number;
  tokens?: number | null;
  error?: string | null;
}

interface CompareResult {
  a: ModelResponse;
  b: ModelResponse;
}

function defaultKey(provider: string) {
  return `default:${provider}`;
}

export function ModelManagerPanel() {
  const { providers, modelsForProvider, refresh, loading, lastUpdated } = useModelRegistry();
  const [profileId, setProfileId] = useState(DEFAULT_PROFILE);
  const [defaults, setDefaults] = useState<Record<string, string>>({});
  const [filter, setFilter] = useState("");
  const [expanded, setExpanded] = useState<string | null>(null);
  const [testPrompt, setTestPrompt] = useState("Reply with the single word: ok");
  const [testing, setTesting] = useState<string | null>(null);
  const [lastResult, setLastResult] = useState<Record<string, ModelResponse>>({});
  const [error, setError] = useState<string | null>(null);

  const loadProfile = useCallback(async () => {
    try {
      const id = await invoke<string>("panel_settings_get_default_profile");
      if (id) setProfileId(id);
    } catch {
      /* ignore */
    }
  }, []);

  const loadDefaults = useCallback(async () => {
    try {
      const stored = await invoke<Record<string, string>>("panel_settings_get_all", {
        profileId,
        panel: PANEL_ID,
      });
      const next: Record<string, string> = {};
      for (const p of providers) {
        const saved = stored?.[defaultKey(p)];
        next[p] = (typeof saved === "string" && saved) || PROVIDER_DEFAULT_MODEL[p] || "";
      }
      setDefaults(next);
    } catch {
      const next: Record<string, string> = {};
      for (const p of providers) next[p] = PROVIDER_DEFAULT_MODEL[p] || "";
      setDefaults(next);
    }
  }, [providers, profileId]);

  useEffect(() => {
    loadProfile();
  }, [loadProfile]);

  useEffect(() => {
    loadDefaults();
  }, [loadDefaults]);

  const setDefault = async (provider: string, model: string) => {
    setError(null);
    setDefaults((prev) => ({ ...prev, [provider]: model }));
    try {
      await invoke("panel_settings_set", {
        profileId,
        panel: PANEL_ID,
        key: defaultKey(provider),
        value: model,
      });
    } catch (e) {
      setError(String(e));
    }
  };

  const runTest = async (provider: string, model: string) => {
    setTesting(`${provider}/${model}`);
    setError(null);
    try {
      const out = await invoke<CompareResult>("compare_models", {
        prompt: testPrompt,
        providerA: provider,
        modelA: model,
        providerB: provider,
        modelB: model,
      });
      setLastResult((prev) => ({ ...prev, [`${provider}/${model}`]: out.a }));
    } catch (e) {
      setError(String(e));
    } finally {
      setTesting(null);
    }
  };

  const visibleProviders = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return providers;
    return providers.filter(
      (p) =>
        p.toLowerCase().includes(q) ||
        modelsForProvider(p).some((m) => m.toLowerCase().includes(q)),
    );
  }, [providers, modelsForProvider, filter]);

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        color: "var(--text-primary)",
        background: "var(--bg-primary)",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "var(--space-2)",
          padding: "var(--space-3) var(--space-4)",
          borderBottom: "1px solid var(--border-color)",
        }}
      >
        <h3 style={{ margin: 0, fontSize: "var(--font-size-lg)" }}>Model Manager</h3>
        <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginLeft: "var(--space-2)" }}>
          {lastUpdated ? `updated ${new Date(lastUpdated).toLocaleTimeString()}` : "not loaded"}
        </span>
        <div style={{ flex: 1 }} />
        <label htmlFor="modelmgr-filter" style={{ position: "absolute", left: -10000, width: 1, height: 1, overflow: "hidden" }}>
          Filter providers or models
        </label>
        <input
          id="modelmgr-filter"
          placeholder="Filter…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          style={{
            padding: "var(--space-1) var(--space-2)",
            border: "1px solid var(--border-color)",
            borderRadius: "var(--radius-sm)",
            background: "var(--bg-secondary)",
            color: "var(--text-primary)",
            fontSize: "var(--font-size-base)",
          }}
        />
        <button
          className="panel-btn"
          aria-label="Refresh model registry"
          onClick={refresh}
          disabled={loading}
          style={{ padding: "var(--space-1) var(--space-2)" }}
        >
          {loading ? <Loader2 size={13} className="spin" /> : <RefreshCw size={13} />}
        </button>
      </div>

      <div
        style={{
          padding: "var(--space-2) var(--space-4)",
          borderBottom: "1px solid var(--border-color)",
          display: "flex",
          alignItems: "center",
          gap: "var(--space-2)",
        }}
      >
        <label htmlFor="modelmgr-prompt" style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
          Test prompt
        </label>
        <input
          id="modelmgr-prompt"
          value={testPrompt}
          onChange={(e) => setTestPrompt(e.target.value)}
          style={{
            flex: 1,
            padding: "var(--space-1) var(--space-2)",
            border: "1px solid var(--border-color)",
            borderRadius: "var(--radius-sm)",
            background: "var(--bg-secondary)",
            color: "var(--text-primary)",
            fontSize: "var(--font-size-base)",
          }}
        />
      </div>

      {error && (
        <div
          role="alert"
          style={{
            padding: "var(--space-2) var(--space-4)",
            background: "var(--error-bg)",
            color: "var(--error-color)",
            fontSize: "var(--font-size-base)",
          }}
        >
          {error}
        </div>
      )}

      <div style={{ flex: 1, overflowY: "auto", padding: "var(--space-3)" }}>
        {visibleProviders.map((provider) => {
          const models = modelsForProvider(provider);
          const isOpen = expanded === provider;
          const def = defaults[provider] || "";
          return (
            <div
              key={provider}
              style={{
                marginBottom: "var(--space-2)",
                border: "1px solid var(--border-color)",
                borderRadius: "var(--radius-sm)",
                background: "var(--bg-secondary)",
                overflow: "hidden",
              }}
            >
              <button
                type="button"
                onClick={() => setExpanded(isOpen ? null : provider)}
                aria-expanded={isOpen}
                style={{
                  width: "100%",
                  padding: "var(--space-3)",
                  display: "flex",
                  alignItems: "center",
                  gap: "var(--space-3)",
                  background: "transparent",
                  border: "none",
                  color: "var(--text-primary)",
                  cursor: "pointer",
                  textAlign: "left",
                  fontSize: "var(--font-size-md)",
                }}
              >
                <span style={{ minWidth: 20, color: "var(--text-secondary)" }}>{isOpen ? "▾" : "▸"}</span>
                <strong style={{ minWidth: 120 }}>{provider}</strong>
                <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>
                  {models.length} model{models.length === 1 ? "" : "s"}
                </span>
                <div style={{ flex: 1 }} />
                <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
                  default: <code>{def || "—"}</code>
                </span>
              </button>

              {isOpen && (
                <div style={{ borderTop: "1px solid var(--border-color)" }}>
                  {models.length === 0 ? (
                    <div
                      style={{
                        padding: "var(--space-3)",
                        color: "var(--text-secondary)",
                        fontSize: "var(--font-size-base)",
                      }}
                    >
                      No models registered for this provider.
                    </div>
                  ) : (
                    <table style={{ width: "100%", borderCollapse: "collapse" }}>
                      <thead>
                        <tr style={{ background: "var(--bg-tertiary)" }}>
                          <th
                            scope="col"
                            style={{
                              textAlign: "left",
                              padding: "var(--space-2) var(--space-3)",
                              fontSize: "var(--font-size-xs)",
                              fontWeight: 600,
                              color: "var(--text-secondary)",
                            }}
                          >
                            Model
                          </th>
                          <th
                            scope="col"
                            style={{
                              textAlign: "left",
                              padding: "var(--space-2) var(--space-3)",
                              fontSize: "var(--font-size-xs)",
                              fontWeight: 600,
                              color: "var(--text-secondary)",
                            }}
                          >
                            Default
                          </th>
                          <th
                            scope="col"
                            style={{
                              textAlign: "left",
                              padding: "var(--space-2) var(--space-3)",
                              fontSize: "var(--font-size-xs)",
                              fontWeight: 600,
                              color: "var(--text-secondary)",
                            }}
                          >
                            Test
                          </th>
                          <th
                            scope="col"
                            style={{
                              textAlign: "left",
                              padding: "var(--space-2) var(--space-3)",
                              fontSize: "var(--font-size-xs)",
                              fontWeight: 600,
                              color: "var(--text-secondary)",
                            }}
                          >
                            Last response
                          </th>
                        </tr>
                      </thead>
                      <tbody>
                        {models.map((model) => {
                          const key = `${provider}/${model}`;
                          const resp = lastResult[key];
                          const isDefault = def === model;
                          const isTesting = testing === key;
                          return (
                            <tr key={model} style={{ borderTop: "1px solid var(--border-color)" }}>
                              <td
                                style={{
                                  padding: "var(--space-2) var(--space-3)",
                                  fontFamily: "var(--font-mono, ui-monospace, monospace)",
                                  fontSize: "var(--font-size-base)",
                                }}
                              >
                                {model}
                              </td>
                              <td style={{ padding: "var(--space-2) var(--space-3)" }}>
                                <button
                                  className="panel-btn"
                                  onClick={() => setDefault(provider, model)}
                                  disabled={isDefault}
                                  aria-pressed={isDefault}
                                  aria-label={isDefault ? `${model} is the default` : `Set ${model} as default`}
                                  style={{
                                    padding: "var(--space-1) var(--space-2)",
                                    fontSize: "var(--font-size-xs)",
                                  }}
                                >
                                  {isDefault ? "✓ default" : "Set default"}
                                </button>
                              </td>
                              <td style={{ padding: "var(--space-2) var(--space-3)" }}>
                                <button
                                  className="panel-btn"
                                  onClick={() => runTest(provider, model)}
                                  disabled={isTesting}
                                  aria-label={`Test ${model}`}
                                  style={{
                                    padding: "var(--space-1) var(--space-2)",
                                    fontSize: "var(--font-size-xs)",
                                    display: "inline-flex",
                                    alignItems: "center",
                                    gap: "var(--space-1)",
                                  }}
                                >
                                  {isTesting ? (
                                    <Loader2 size={12} className="spin" />
                                  ) : (
                                    <Play size={12} />
                                  )}
                                  {isTesting ? "Testing…" : "Test"}
                                </button>
                              </td>
                              <td
                                style={{
                                  padding: "var(--space-2) var(--space-3)",
                                  fontSize: "var(--font-size-xs)",
                                  color: resp?.error ? "var(--error-color)" : "var(--text-secondary)",
                                  maxWidth: 400,
                                  overflow: "hidden",
                                  textOverflow: "ellipsis",
                                  whiteSpace: "nowrap",
                                }}
                                title={resp?.error || resp?.content || ""}
                              >
                                {resp
                                  ? resp.error
                                    ? `error: ${resp.error}`
                                    : `${resp.duration_ms}ms · ${resp.content.slice(0, 120)}`
                                  : "—"}
                              </td>
                            </tr>
                          );
                        })}
                      </tbody>
                    </table>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

export default ModelManagerPanel;
