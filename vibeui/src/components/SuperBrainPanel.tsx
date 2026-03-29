import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Compass, Handshake, Link, Trophy, BrainCircuit,
  Play, Loader2, ToggleLeft, ToggleRight,
  Clock, Zap, Hash, Layers, Gauge, ArrowDown, Sparkles,
  type LucideIcon,
} from "lucide-react";

// ── Types ────────────────────────────────────────────────────────────────────

interface ModelContribution {
  provider: string;
  model: string;
  role: string;
  content: string;
  duration_ms: number;
  tokens?: number;
}

interface SuperBrainResult {
  mode: string;
  final_response: string;
  model_responses: ModelContribution[];
  routing_reason?: string;
  total_duration_ms: number;
  total_tokens: number;
}

interface RoutingDecision {
  provider: string;
  model: string;
  category: string;
  reason: string;
  confidence: number;
}

interface ProviderConfig {
  enabled: boolean;
  provider: string;
  model: string;
}

// ── Constants ────────────────────────────────────────────────────────────────

const AVAILABLE_PROVIDERS: ProviderConfig[] = [
  { enabled: true, provider: "claude", model: "claude-3.5-sonnet" },
  { enabled: true, provider: "openai", model: "gpt-4o" },
  { enabled: false, provider: "gemini", model: "gemini-2.0-flash" },
  { enabled: false, provider: "grok", model: "grok-2" },
  { enabled: false, provider: "groq", model: "llama-3.3-70b-versatile" },
  { enabled: false, provider: "ollama", model: "llama3.2" },
];

interface ModeInfo {
  id: string;
  name: string;
  description: string;
  icon: LucideIcon;
  color: string;
}

const MODES: ModeInfo[] = [
  { id: "router", name: "Smart Router", description: "Routes to best model for the task", icon: Compass, color: "var(--accent-green)" },
  { id: "consensus", name: "Consensus", description: "Multiple models vote on the answer", icon: Handshake, color: "var(--accent-blue)" },
  { id: "chain", name: "Chain Relay", description: "Models refine each other's thinking", icon: Link, color: "var(--accent-gold)" },
  { id: "bestofn", name: "Best-of-N", description: "A judge picks the best response", icon: Trophy, color: "var(--accent-rose)" },
  { id: "specialist", name: "Specialist", description: "Breaks problem into subtasks for experts", icon: BrainCircuit, color: "var(--accent-purple)" },
];

const CHAIN_STEP_COLORS = ["var(--accent-blue)", "var(--accent-gold)", "var(--accent-green)"];

// ── Styles ───────────────────────────────────────────────────────────────────

const S = {
  container: {
    height: "100%", display: "flex", flexDirection: "column",
    fontFamily: "var(--font-family, system-ui, sans-serif)",
    color: "var(--text-primary)",
    background: "var(--bg-primary)",
    overflow: "hidden",
  } as const,

  scrollArea: {
    flex: 1, overflowY: "auto", padding: 20,
  } as const,

  sectionTitle: {
    fontSize: 11, fontWeight: 700, textTransform: "uppercase",
    letterSpacing: "0.06em", color: "var(--text-secondary)",
    margin: "0 0 12px 0",
    display: "flex", alignItems: "center", gap: 6,
  } as const,

  modeGrid: {
    display: "grid",
    gridTemplateColumns: "repeat(auto-fill, minmax(150px, 1fr))",
    gap: 8, marginBottom: 24,
  } as const,

  modeCard: (active: boolean, color: string) => ({
    border: active ? `1.5px solid ${color}` : "1px solid var(--border-color)",
    borderRadius: "var(--radius-md)",
    padding: "14px 12px",
    cursor: "pointer",
    background: active ? `color-mix(in srgb, ${color} 8%, var(--bg-secondary))` : "var(--bg-secondary)",
    textAlign: "center" as const,
    transition: "all var(--transition-fast)",
    boxShadow: active ? `0 0 16px color-mix(in srgb, ${color} 12%, transparent)` : "none",
  }),

  providerRow: (enabled: boolean) => ({
    display: "flex", alignItems: "center", gap: 10,
    padding: "8px 12px",
    borderRadius: "var(--radius-sm)",
    border: "1px solid var(--border-color)",
    background: "var(--bg-secondary)",
    opacity: enabled ? 1 : 0.45,
    transition: "opacity var(--transition-fast)",
  }),

  btn: {
    padding: "9px 20px", border: "none",
    borderRadius: "var(--radius-sm)",
    cursor: "pointer", fontSize: 13, fontWeight: 600,
    background: "var(--accent-blue)", color: "var(--btn-primary-fg, #fff)",
    display: "inline-flex", alignItems: "center", gap: 6,
    transition: "opacity var(--transition-fast)",
  } as const,

  input: {
    width: "100%", padding: "7px 10px",
    borderRadius: "var(--radius-sm)",
    border: "1px solid var(--border-color)",
    background: "var(--bg-secondary)",
    color: "var(--text-primary)", fontSize: 13,
    boxSizing: "border-box",
  } as const,

  textarea: {
    width: "100%", padding: "12px 14px",
    borderRadius: "var(--radius-md)",
    border: "1px solid var(--border-color)",
    background: "var(--bg-secondary)",
    color: "var(--text-primary)", fontSize: 13,
    resize: "vertical", minHeight: 90,
    boxSizing: "border-box", fontFamily: "inherit",
    lineHeight: 1.5,
  } as const,

  select: {
    padding: "7px 10px",
    borderRadius: "var(--radius-sm)",
    border: "1px solid var(--border-color)",
    background: "var(--bg-secondary)",
    color: "var(--text-primary)", fontSize: 13,
  } as const,

  card: {
    border: "1px solid var(--border-color)",
    borderRadius: "var(--radius-md)",
    padding: 14, marginBottom: 10,
    background: "var(--bg-secondary)",
  } as const,

  metricsBar: {
    display: "flex", gap: 16, flexWrap: "wrap",
    padding: "10px 14px",
    borderRadius: "var(--radius-md)",
    background: "var(--bg-tertiary)",
    border: "1px solid var(--border-color)",
    fontSize: 12, color: "var(--text-secondary)",
    marginBottom: 16,
  } as const,

  metricItem: {
    display: "inline-flex", alignItems: "center", gap: 5,
  } as const,

  badge: (color: string) => ({
    display: "inline-flex", alignItems: "center", gap: 4,
    padding: "3px 10px", borderRadius: 10,
    fontSize: 11, fontWeight: 600,
    background: `color-mix(in srgb, ${color} 15%, transparent)`,
    color,
  }),

  finalBox: {
    border: "1.5px solid var(--accent-blue)",
    borderRadius: "var(--radius-md)",
    padding: 16,
    background: "color-mix(in srgb, var(--accent-blue) 4%, var(--bg-secondary))",
    marginTop: 16,
  } as const,

  finalLabel: {
    fontSize: 11, fontWeight: 700, textTransform: "uppercase",
    letterSpacing: "0.06em",
    color: "var(--accent-blue)", marginBottom: 10,
    display: "flex", alignItems: "center", gap: 5,
  } as const,

  responseText: {
    fontSize: 13, lineHeight: 1.6, whiteSpace: "pre-wrap",
  } as const,

  confidenceBar: {
    height: 4, borderRadius: 2,
    background: "var(--border-color)",
    position: "relative" as const,
    overflow: "hidden" as const, marginTop: 4,
  } as const,

  confidenceFill: (pct: number) => ({
    position: "absolute" as const,
    left: 0, top: 0, height: "100%",
    width: `${Math.round(pct * 100)}%`,
    background: pct > 0.7 ? "var(--accent-green)" : pct > 0.4 ? "var(--accent-gold)" : "var(--accent-rose)",
    borderRadius: 2,
    transition: "width var(--transition-smooth)",
  }),

  progressMsg: {
    fontSize: 12, color: "var(--accent-blue)",
    display: "inline-flex", alignItems: "center", gap: 5,
    fontStyle: "italic" as const,
  } as const,

  arrow: {
    textAlign: "center" as const, padding: "2px 0",
    color: "var(--text-secondary)",
  } as const,

  label: {
    fontSize: 11, fontWeight: 600, textTransform: "uppercase",
    letterSpacing: "0.04em",
    color: "var(--text-secondary)", marginBottom: 6, display: "block",
  } as const,
};

// ── Component ────────────────────────────────────────────────────────────────

export function SuperBrainPanel() {
  const [mode, setMode] = useState("router");
  const [providers, setProviders] = useState<ProviderConfig[]>(
    AVAILABLE_PROVIDERS.map(p => ({ ...p }))
  );
  const [judgeProvider, setJudgeProvider] = useState("claude");
  const [judgeModel, setJudgeModel] = useState("claude-3.5-sonnet");
  const [prompt, setPrompt] = useState("");
  const [thinking, setThinking] = useState(false);
  const [progress, setProgress] = useState("");
  const [result, setResult] = useState<SuperBrainResult | null>(null);
  const [routingInfo, setRoutingInfo] = useState<RoutingDecision | null>(null);

  useEffect(() => {
    const unlisten = listen("superbrain:progress", (event: any) => {
      const data = event.payload as Record<string, any>;
      if (data.step === "routing") setProgress(`Routing to ${data.provider}/${data.model}`);
      else if (data.step === "querying") setProgress(`Querying ${data.provider} (${(data.index ?? 0) + 1})`);
      else if (data.step === "synthesizing") setProgress("Synthesizing consensus...");
      else if (data.step === "chain") setProgress(`Chain step ${(data.index ?? 0) + 1}/${data.total} (${data.provider})`);
      else if (data.step === "judging") setProgress(`Judge evaluating (${data.provider})`);
      else if (data.step === "decomposing") setProgress("Decomposing into subtasks...");
      else if (data.step === "specialist") setProgress(`Specialist: ${data.subtask?.slice(0, 40)}...`);
      else if (data.step === "merging") setProgress("Merging specialist results...");
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  const toggleProvider = (idx: number) => {
    setProviders(prev => prev.map((p, i) => i === idx ? { ...p, enabled: !p.enabled } : p));
  };

  const updateProviderModel = (idx: number, model: string) => {
    setProviders(prev => prev.map((p, i) => i === idx ? { ...p, model } : p));
  };

  const doQuery = useCallback(async () => {
    if (!prompt.trim()) return;
    setThinking(true);
    setProgress("Starting...");
    setResult(null);
    setRoutingInfo(null);

    try {
      if (mode === "router") {
        const rd = await invoke<RoutingDecision>("superbrain_route", { prompt: prompt.trim() });
        setRoutingInfo(rd);
      }

      const enabledProviders = providers.filter(p => p.enabled).map(p => ({
        provider: p.provider,
        model: p.model,
      }));

      const res = await invoke<SuperBrainResult>("superbrain_query", {
        prompt: prompt.trim(),
        mode,
        providers: {
          list: enabledProviders,
          judge: mode === "bestofn" ? { provider: judgeProvider, model: judgeModel } : undefined,
        },
      });
      setResult(res);
    } catch (e) {
      console.error("SuperBrain query failed:", e);
      setResult({
        mode,
        final_response: `Error: ${e}`,
        model_responses: [],
        total_duration_ms: 0,
        total_tokens: 0,
      });
    } finally {
      setThinking(false);
      setProgress("");
    }
  }, [prompt, mode, providers, judgeProvider, judgeModel]);

  const finalLabel = mode === "router" ? "Response"
    : mode === "chain" ? "Final Output"
    : mode === "bestofn" ? "Judge Decision"
    : mode === "specialist" ? "Merged Result"
    : "Synthesized Consensus";

  return (
    <div style={S.container}>
      <div style={S.scrollArea}>

        {/* ── Mode Selector ──────────────────────────────────────────── */}
        <div style={S.sectionTitle}>
          <Layers size={12} strokeWidth={2} /> Mode
        </div>
        <div style={S.modeGrid}>
          {MODES.map(m => {
            const Icon = m.icon;
            const active = mode === m.id;
            return (
              <div
                key={m.id}
                style={S.modeCard(active, m.color)}
                onClick={() => { setMode(m.id); setResult(null); }}
              >
                <Icon
                  size={22} strokeWidth={1.5}
                  style={{ color: active ? m.color : "var(--text-secondary)", marginBottom: 6, transition: "color var(--transition-fast)" }}
                />
                <div style={{ fontSize: 12, fontWeight: 700, color: active ? "var(--text-primary)" : "var(--text-secondary)" }}>
                  {m.name}
                </div>
                <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 4, lineHeight: 1.3 }}>
                  {m.description}
                </div>
              </div>
            );
          })}
        </div>

        {/* ── Provider Config (non-router modes) ─────────────────────── */}
        {mode !== "router" && (
          <div style={{ marginBottom: 20 }}>
            <div style={S.sectionTitle}>
              <Zap size={12} strokeWidth={2} /> Providers
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(260px, 1fr))", gap: 8 }}>
              {providers.map((p, i) => (
                <div key={i} style={S.providerRow(p.enabled)}>
                  <span
                    style={{ cursor: "pointer", display: "flex" }}
                    onClick={() => toggleProvider(i)}
                  >
                    {p.enabled
                      ? <ToggleRight size={18} strokeWidth={1.5} style={{ color: "var(--accent-green)" }} />
                      : <ToggleLeft size={18} strokeWidth={1.5} style={{ color: "var(--text-secondary)" }} />
                    }
                  </span>
                  <span style={{ fontSize: 12, fontWeight: 600, minWidth: 52 }}>{p.provider}</span>
                  <input
                    style={{ ...S.input, flex: 1 }}
                    value={p.model}
                    onChange={e => updateProviderModel(i, e.target.value)}
                    disabled={!p.enabled}
                  />
                </div>
              ))}
            </div>

            {mode === "bestofn" && (
              <div style={{ marginTop: 12 }}>
                <span style={S.label}>Judge</span>
                <div style={{ display: "flex", gap: 8 }}>
                  <select
                    style={S.select}
                    value={judgeProvider}
                    onChange={e => setJudgeProvider(e.target.value)}
                  >
                    {["claude", "openai", "gemini", "grok", "groq", "ollama"].map(p => (
                      <option key={p} value={p}>{p}</option>
                    ))}
                  </select>
                  <input
                    style={{ ...S.input, width: 200 }}
                    value={judgeModel}
                    onChange={e => setJudgeModel(e.target.value)}
                    placeholder="Judge model"
                  />
                </div>
              </div>
            )}
          </div>
        )}

        {/* ── Query Input ────────────────────────────────────────────── */}
        <div style={{ marginBottom: 14 }}>
          <span style={S.label}>Query</span>
          <textarea
            style={S.textarea}
            placeholder="Ask anything — SuperBrain will orchestrate multiple models..."
            value={prompt}
            onChange={e => setPrompt(e.target.value)}
            onKeyDown={e => { if (e.key === "Enter" && e.metaKey) doQuery(); }}
          />
        </div>

        <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 24 }}>
          <button
            style={{ ...S.btn, opacity: (thinking || !prompt.trim()) ? 0.45 : 1 }}
            disabled={thinking || !prompt.trim()}
            onClick={doQuery}
          >
            {thinking
              ? <><Loader2 size={14} strokeWidth={2} className="spin" /> Thinking...</>
              : <><Play size={14} strokeWidth={2} /> Think</>
            }
          </button>
          {progress && (
            <span style={S.progressMsg}>
              <Loader2 size={12} strokeWidth={2} className="spin" />
              {progress}
            </span>
          )}
        </div>

        {/* ── Results ────────────────────────────────────────────────── */}
        {result && (
          <div>
            {/* Metrics bar */}
            <div style={S.metricsBar}>
              <span style={S.metricItem}>
                <Layers size={12} strokeWidth={1.5} /> Mode: <strong>{result.mode}</strong>
              </span>
              <span style={S.metricItem}>
                <Clock size={12} strokeWidth={1.5} /> Time: <strong>{(result.total_duration_ms / 1000).toFixed(1)}s</strong>
              </span>
              <span style={S.metricItem}>
                <Hash size={12} strokeWidth={1.5} /> Tokens: <strong>{result.total_tokens.toLocaleString()}</strong>
              </span>
              <span style={S.metricItem}>
                <BrainCircuit size={12} strokeWidth={1.5} /> Models: <strong>{result.model_responses.length}</strong>
              </span>
            </div>

            {/* Smart Router: routing info */}
            {mode === "router" && routingInfo && (
              <div style={{ ...S.card, display: "flex", alignItems: "center", gap: 12 }}>
                <Compass size={16} strokeWidth={1.5} style={{ color: "var(--accent-green)", flexShrink: 0 }} />
                <span style={S.badge("var(--accent-green)")}>{routingInfo.category}</span>
                <span style={{ fontSize: 12, flex: 1 }}>{routingInfo.reason}</span>
                <div style={{ width: 80, flexShrink: 0 }}>
                  <div style={{ fontSize: 10, textAlign: "right", color: "var(--text-secondary)" }}>
                    {Math.round(routingInfo.confidence * 100)}%
                  </div>
                  <div style={S.confidenceBar}>
                    <div style={S.confidenceFill(routingInfo.confidence)} />
                  </div>
                </div>
              </div>
            )}

            {/* Consensus: individual responses */}
            {mode === "consensus" && (
              <div>
                <div style={{ ...S.sectionTitle, marginTop: 4 }}>
                  <Handshake size={12} strokeWidth={2} /> Model Responses
                </div>
                <div style={{ display: "grid", gridTemplateColumns: `repeat(${Math.min(result.model_responses.length, 3)}, 1fr)`, gap: 8 }}>
                  {result.model_responses.map((resp, i) => (
                    <div key={i} style={S.card}>
                      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                        <span style={{ fontSize: 12, fontWeight: 600 }}>{resp.provider}/{resp.model}</span>
                        <span style={{ fontSize: 10, color: "var(--text-secondary)", display: "inline-flex", alignItems: "center", gap: 3 }}>
                          <Clock size={10} strokeWidth={1.5} /> {resp.duration_ms}ms
                        </span>
                      </div>
                      <div style={{ ...S.responseText, maxHeight: 250, overflowY: "auto" }}>
                        {resp.content}
                      </div>
                    </div>
                  ))}
                </div>
                <div style={{ ...S.sectionTitle, marginTop: 16 }}>
                  <Sparkles size={12} strokeWidth={2} /> Consensus Synthesis
                </div>
              </div>
            )}

            {/* Chain Relay: step-by-step */}
            {mode === "chain" && (
              <div>
                <div style={{ ...S.sectionTitle, marginTop: 4 }}>
                  <Link size={12} strokeWidth={2} /> Chain Steps
                </div>
                {result.model_responses.map((resp, i) => {
                  const isFinal = i === result.model_responses.length - 1;
                  const stepColor = isFinal ? "var(--accent-green)" : CHAIN_STEP_COLORS[i % CHAIN_STEP_COLORS.length];
                  return (
                    <div key={i}>
                      {i > 0 && (
                        <div style={S.arrow}>
                          <ArrowDown size={16} strokeWidth={1.5} />
                        </div>
                      )}
                      <div style={{ ...S.card, borderLeft: `3px solid ${stepColor}` }}>
                        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                          <span style={{ fontSize: 12, fontWeight: 600 }}>
                            Step {i + 1}: {resp.provider}/{resp.model}
                          </span>
                          <span style={S.badge(stepColor)}>{resp.role}</span>
                        </div>
                        <div style={{ ...S.responseText, maxHeight: 300, overflowY: "auto" }}>
                          {resp.content}
                        </div>
                        <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 8, display: "flex", alignItems: "center", gap: 8 }}>
                          <span style={{ display: "inline-flex", alignItems: "center", gap: 3 }}>
                            <Clock size={10} strokeWidth={1.5} /> {resp.duration_ms}ms
                          </span>
                          {resp.tokens != null && (
                            <span style={{ display: "inline-flex", alignItems: "center", gap: 3 }}>
                              <Hash size={10} strokeWidth={1.5} /> {resp.tokens} tokens
                            </span>
                          )}
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}

            {/* Best-of-N: candidate responses */}
            {mode === "bestofn" && (
              <div>
                <div style={{ ...S.sectionTitle, marginTop: 4 }}>
                  <Trophy size={12} strokeWidth={2} /> Candidate Responses
                </div>
                <div style={{ display: "grid", gridTemplateColumns: `repeat(${Math.min(result.model_responses.length, 3)}, 1fr)`, gap: 8 }}>
                  {result.model_responses.map((resp, i) => (
                    <div key={i} style={S.card}>
                      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                        <span style={{ fontSize: 12, fontWeight: 600 }}>{resp.provider}/{resp.model}</span>
                        <span style={{ fontSize: 10, color: "var(--text-secondary)", display: "inline-flex", alignItems: "center", gap: 3 }}>
                          <Clock size={10} strokeWidth={1.5} /> {resp.duration_ms}ms
                        </span>
                      </div>
                      <div style={{ ...S.responseText, maxHeight: 200, overflowY: "auto" }}>
                        {resp.content}
                      </div>
                    </div>
                  ))}
                </div>
                <div style={{ ...S.sectionTitle, marginTop: 16 }}>
                  <Gauge size={12} strokeWidth={2} /> Judge Verdict
                </div>
              </div>
            )}

            {/* Specialist: subtask results */}
            {mode === "specialist" && (
              <div>
                {result.routing_reason && (
                  <div style={{ ...S.card, borderLeft: "3px solid var(--accent-purple)", display: "flex", alignItems: "center", gap: 8 }}>
                    <BrainCircuit size={14} strokeWidth={1.5} style={{ color: "var(--accent-purple)", flexShrink: 0 }} />
                    <span style={S.badge("var(--accent-purple)")}>{result.routing_reason}</span>
                  </div>
                )}
                <div style={{ ...S.sectionTitle, marginTop: 8 }}>
                  <Sparkles size={12} strokeWidth={2} /> Specialist Results
                </div>
                {result.model_responses.map((resp, i) => (
                  <div key={i} style={S.card}>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
                      <span style={{ fontSize: 12, fontWeight: 600 }}>{resp.provider}/{resp.model}</span>
                      <span style={S.badge("var(--accent-purple)")}>{resp.role}</span>
                    </div>
                    <div style={{ ...S.responseText, maxHeight: 200, overflowY: "auto" }}>
                      {resp.content}
                    </div>
                  </div>
                ))}
                <div style={{ ...S.sectionTitle, marginTop: 16 }}>
                  <Sparkles size={12} strokeWidth={2} /> Merged Response
                </div>
              </div>
            )}

            {/* ── Final Response (always shown) ──────────────────────── */}
            <div style={S.finalBox}>
              <div style={S.finalLabel}>
                <Sparkles size={12} strokeWidth={2} /> {finalLabel}
              </div>
              <div style={S.responseText}>
                {result.final_response}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default SuperBrainPanel;
