import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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
  icon: string;
}

const MODES: ModeInfo[] = [
  { id: "router", name: "Smart Router", description: "Routes to best model for the task", icon: "\u{1F9ED}" },
  { id: "consensus", name: "Consensus", description: "Multiple models vote on the answer", icon: "\u{1F91D}" },
  { id: "chain", name: "Chain Relay", description: "Models refine each other's thinking", icon: "\u{1F517}" },
  { id: "bestofn", name: "Best-of-N", description: "A judge picks the best response", icon: "\u{1F3C6}" },
  { id: "specialist", name: "Specialist", description: "Breaks problem into subtasks for experts", icon: "\u{1F9E0}" },
];

// ── Styles ───────────────────────────────────────────────────────────────────

const S = {
  container: { height: "100%", display: "flex", flexDirection: "column", fontFamily: "var(--font-family, sans-serif)", color: "var(--text-primary, #e0e0e0)", background: "var(--bg-primary, #1e1e1e)", overflow: "hidden" } as const,
  scrollArea: { flex: 1, overflowY: "auto", padding: 20 } as const,
  btn: { padding: "10px 20px", border: "none", borderRadius: 6, cursor: "pointer", fontSize: 14, fontWeight: 600, background: "var(--accent, #4a9eff)", color: "#fff" } as const,
  btnSecondary: { padding: "6px 12px", border: "1px solid var(--border-color, #444)", borderRadius: 6, cursor: "pointer", fontSize: 12, background: "transparent", color: "var(--text-primary, #ccc)" } as const,
  input: { width: "100%", padding: "8px 10px", borderRadius: 6, border: "1px solid var(--border-color, #444)", background: "var(--bg-secondary, #2a2a2a)", color: "var(--text-primary, #e0e0e0)", fontSize: 13, boxSizing: "border-box" } as const,
  textarea: { width: "100%", padding: "12px 14px", borderRadius: 8, border: "1px solid var(--border-color, #444)", background: "var(--bg-secondary, #2a2a2a)", color: "var(--text-primary, #e0e0e0)", fontSize: 14, resize: "vertical", minHeight: 100, boxSizing: "border-box", fontFamily: "inherit" } as const,
  card: { border: "1px solid var(--border-color, #333)", borderRadius: 8, padding: 14, marginBottom: 12, background: "var(--bg-secondary, #252525)" } as const,
  modeCard: (active: boolean) => ({
    border: active ? "2px solid var(--accent, #4a9eff)" : "1px solid var(--border-color, #444)",
    borderRadius: 10,
    padding: 16,
    cursor: "pointer",
    background: active ? "var(--accent, #4a9eff)11" : "var(--bg-secondary, #252525)",
    textAlign: "center" as const,
    transition: "border-color 0.15s",
    minWidth: 120,
  }),
  badge: (color: string) => ({ display: "inline-block", padding: "3px 10px", borderRadius: 10, fontSize: 11, fontWeight: 600, background: color + "22", color }),
  h2: { fontSize: 16, fontWeight: 700, margin: "0 0 16px 0" } as const,
  h3: { fontSize: 14, fontWeight: 600, margin: "16px 0 8px 0" } as const,
  label: { fontSize: 12, color: "var(--text-secondary, #999)", marginBottom: 4, display: "block" } as const,
  metricsBar: { display: "flex", gap: 20, padding: "10px 14px", borderRadius: 8, background: "var(--bg-secondary, #252525)", border: "1px solid var(--border-color, #333)", fontSize: 12, color: "var(--text-secondary, #aaa)" } as const,
  winnerBadge: { display: "inline-block", padding: "4px 12px", borderRadius: 10, fontSize: 12, fontWeight: 700, background: "#ffd70033", color: "#ffd700", marginLeft: 8 } as const,
  arrow: { textAlign: "center" as const, fontSize: 20, color: "var(--text-secondary, #666)", padding: "4px 0" } as const,
  confidenceMeter: (_pct: number) => ({
    height: 4,
    borderRadius: 2,
    background: "var(--border-color, #444)",
    position: "relative" as const,
    overflow: "hidden" as const,
    marginTop: 4,
  }),
  confidenceFill: (pct: number) => ({
    position: "absolute" as const,
    left: 0,
    top: 0,
    height: "100%",
    width: `${Math.round(pct * 100)}%`,
    background: pct > 0.7 ? "#4aff7f" : pct > 0.4 ? "#ffaa33" : "#ff4a4a",
    borderRadius: 2,
  }),
  progressMsg: { fontSize: 12, color: "var(--accent, #4a9eff)", padding: "8px 0", fontStyle: "italic" as const } as const,
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

  // Listen for progress events
  useEffect(() => {
    const unlisten = listen("superbrain:progress", (event: any) => {
      const data = event.payload as Record<string, any>;
      if (data.step === "routing") setProgress(`Routing to ${data.provider}/${data.model}...`);
      else if (data.step === "querying") setProgress(`Querying ${data.provider} (${(data.index ?? 0) + 1})...`);
      else if (data.step === "synthesizing") setProgress("Synthesizing consensus...");
      else if (data.step === "chain") setProgress(`Chain step ${(data.index ?? 0) + 1}/${data.total} (${data.provider})...`);
      else if (data.step === "judging") setProgress(`Judge (${data.provider}) evaluating...`);
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
      // If router mode, also fetch routing decision
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

  // ── Render ───

  return (
    <div style={S.container}>
      <div style={S.scrollArea}>
        {/* Mode selector */}
        <h2 style={S.h2}>SuperBrain Mode</h2>
        <div style={{ display: "flex", gap: 10, marginBottom: 20, flexWrap: "wrap" }}>
          {MODES.map(m => (
            <div
              key={m.id}
              style={S.modeCard(mode === m.id)}
              onClick={() => { setMode(m.id); setResult(null); }}
            >
              <div style={{ fontSize: 24, marginBottom: 6 }}>{m.icon}</div>
              <div style={{ fontSize: 13, fontWeight: 700 }}>{m.name}</div>
              <div style={{ fontSize: 11, color: "var(--text-secondary, #888)", marginTop: 4 }}>{m.description}</div>
            </div>
          ))}
        </div>

        {/* Provider config (not for router mode) */}
        {mode !== "router" && (
          <div style={{ marginBottom: 20 }}>
            <h3 style={S.h3}>Providers</h3>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(240, 1fr))", gap: 8 }}>
              {providers.map((p, i) => (
                <div key={i} style={{ ...S.card, display: "flex", alignItems: "center", gap: 8, opacity: p.enabled ? 1 : 0.5 }}>
                  <input type="checkbox" checked={p.enabled} onChange={() => toggleProvider(i)} />
                  <span style={{ fontSize: 12, fontWeight: 600, minWidth: 50 }}>{p.provider}</span>
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
                <label style={S.label}>Judge</label>
                <div style={{ display: "flex", gap: 8 }}>
                  <select
                    style={{ padding: "6px 8px", borderRadius: 6, border: "1px solid var(--border-color, #444)", background: "var(--bg-secondary, #2a2a2a)", color: "var(--text-primary, #e0e0e0)", fontSize: 13 }}
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

        {/* Query input */}
        <div style={{ marginBottom: 16 }}>
          <label style={S.label}>Query</label>
          <textarea
            style={S.textarea}
            placeholder="Ask anything..."
            value={prompt}
            onChange={e => setPrompt(e.target.value)}
            onKeyDown={e => { if (e.key === "Enter" && e.metaKey) doQuery(); }}
          />
        </div>

        <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 20 }}>
          <button
            style={{ ...S.btn, opacity: (thinking || !prompt.trim()) ? 0.5 : 1 }}
            disabled={thinking || !prompt.trim()}
            onClick={doQuery}
          >
            {thinking ? "Thinking..." : "Think"}
          </button>
          {progress && <span style={S.progressMsg}>{progress}</span>}
        </div>

        {/* Results area */}
        {result && (
          <div>
            {/* Metrics bar */}
            <div style={S.metricsBar}>
              <span>Mode: <strong>{result.mode}</strong></span>
              <span>Time: <strong>{(result.total_duration_ms / 1000).toFixed(1)}s</strong></span>
              <span>Tokens: <strong>{result.total_tokens.toLocaleString()}</strong></span>
              <span>Models: <strong>{result.model_responses.length}</strong></span>
            </div>

            {/* Smart Router result */}
            {mode === "router" && routingInfo && (
              <div style={{ marginTop: 16 }}>
                <div style={{ ...S.card, display: "flex", alignItems: "center", gap: 12 }}>
                  <span style={S.badge("#4aff7f")}>{routingInfo.category}</span>
                  <span style={{ fontSize: 12 }}>{routingInfo.reason}</span>
                  <div style={{ marginLeft: "auto", width: 80 }}>
                    <div style={{ fontSize: 10, textAlign: "right" }}>{Math.round(routingInfo.confidence * 100)}%</div>
                    <div style={S.confidenceMeter(routingInfo.confidence)}>
                      <div style={S.confidenceFill(routingInfo.confidence)} />
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* Consensus result */}
            {mode === "consensus" && (
              <div style={{ marginTop: 16 }}>
                <h3 style={S.h3}>Model Responses</h3>
                <div style={{ display: "grid", gridTemplateColumns: `repeat(${Math.min(result.model_responses.length, 3)}, 1fr)`, gap: 10 }}>
                  {result.model_responses.map((resp, i) => (
                    <div key={i} style={S.card}>
                      <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 6 }}>
                        {resp.provider}/{resp.model}
                        <span style={{ fontSize: 10, color: "var(--text-secondary, #888)", marginLeft: 6 }}>{resp.duration_ms}ms</span>
                      </div>
                      <div style={{ fontSize: 13, lineHeight: 1.5, whiteSpace: "pre-wrap", maxHeight: 250, overflowY: "auto" }}>
                        {resp.content}
                      </div>
                    </div>
                  ))}
                </div>
                <h3 style={S.h3}>Consensus Synthesis</h3>
              </div>
            )}

            {/* Chain Relay result */}
            {mode === "chain" && (
              <div style={{ marginTop: 16 }}>
                <h3 style={S.h3}>Chain Steps</h3>
                {result.model_responses.map((resp, i) => (
                  <div key={i}>
                    {i > 0 && <div style={S.arrow}>&#x2193;</div>}
                    <div style={{ ...S.card, borderLeft: `3px solid ${i === result.model_responses.length - 1 ? "var(--accent, #4a9eff)" : "var(--border-color, #555)"}` }}>
                      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                        <span style={{ fontSize: 12, fontWeight: 600 }}>
                          Step {i + 1}: {resp.provider}/{resp.model}
                        </span>
                        <span style={S.badge(i === 0 ? "#4a9eff" : i === result.model_responses.length - 1 ? "#4aff7f" : "#ff9f43")}>
                          {resp.role}
                        </span>
                      </div>
                      <div style={{ fontSize: 13, lineHeight: 1.5, whiteSpace: "pre-wrap", maxHeight: 300, overflowY: "auto" }}>
                        {resp.content}
                      </div>
                      <div style={{ fontSize: 10, color: "var(--text-secondary, #888)", marginTop: 6 }}>
                        {resp.duration_ms}ms{resp.tokens != null ? ` | ${resp.tokens} tokens` : ""}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}

            {/* Best-of-N result */}
            {mode === "bestofn" && (
              <div style={{ marginTop: 16 }}>
                <h3 style={S.h3}>Candidate Responses</h3>
                <div style={{ display: "grid", gridTemplateColumns: `repeat(${Math.min(result.model_responses.length, 3)}, 1fr)`, gap: 10 }}>
                  {result.model_responses.map((resp, i) => (
                    <div key={i} style={S.card}>
                      <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 6 }}>
                        {resp.provider}/{resp.model}
                        <span style={{ fontSize: 10, color: "var(--text-secondary, #888)", marginLeft: 6 }}>{resp.duration_ms}ms</span>
                      </div>
                      <div style={{ fontSize: 13, lineHeight: 1.5, whiteSpace: "pre-wrap", maxHeight: 200, overflowY: "auto" }}>
                        {resp.content}
                      </div>
                    </div>
                  ))}
                </div>
                <h3 style={S.h3}>Judge Verdict</h3>
              </div>
            )}

            {/* Specialist result */}
            {mode === "specialist" && (
              <div style={{ marginTop: 16 }}>
                {result.routing_reason && (
                  <div style={{ ...S.card, borderLeft: "3px solid #b94aff" }}>
                    <span style={S.badge("#b94aff")}>{result.routing_reason}</span>
                  </div>
                )}
                <h3 style={S.h3}>Specialist Results</h3>
                {result.model_responses.map((resp, i) => (
                  <div key={i} style={S.card}>
                    <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6 }}>
                      <span style={{ fontSize: 12, fontWeight: 600 }}>{resp.provider}/{resp.model}</span>
                      <span style={S.badge("#b94aff")}>{resp.role}</span>
                    </div>
                    <div style={{ fontSize: 13, lineHeight: 1.5, whiteSpace: "pre-wrap", maxHeight: 200, overflowY: "auto" }}>
                      {resp.content}
                    </div>
                  </div>
                ))}
                <h3 style={S.h3}>Merged Response</h3>
              </div>
            )}

            {/* Final response (always shown) */}
            <div style={{ border: "2px solid var(--accent, #4a9eff)", borderRadius: 8, padding: 16, background: "var(--accent, #4a9eff)08", marginTop: mode === "router" ? 16 : 0 }}>
              <div style={{ fontSize: 12, fontWeight: 700, color: "var(--accent, #4a9eff)", marginBottom: 8 }}>
                {mode === "router" ? "Response" : mode === "chain" ? "Final Output" : mode === "bestofn" ? "Judge Decision" : mode === "specialist" ? "Merged Result" : "Synthesized Consensus"}
              </div>
              <div style={{ fontSize: 13, lineHeight: 1.6, whiteSpace: "pre-wrap" }}>
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
