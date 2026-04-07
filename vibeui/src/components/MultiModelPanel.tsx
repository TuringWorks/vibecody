import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useModelRegistry, PROVIDER_DEFAULT_MODEL } from "../hooks/useModelRegistry";

interface ModelResponse {
 provider: string;
 model: string;
 content: string;
 duration_ms: number;
 tokens: number | null;
 error: string | null;
}

interface CompareResult {
 a: ModelResponse;
 b: ModelResponse;
}

function ResponseCard({ resp, side }: { resp: ModelResponse; side: "A" | "B" }) {
 const isError = !!resp.error;
 return (
 <div style={{
 flex: 1,
 display: "flex",
 flexDirection: "column",
 border: `1px solid ${isError ? "rgba(244,67,54,0.4)" : "var(--border-color)"}`,
 borderRadius: "6px",
 overflow: "hidden",
 minWidth: 0,
 }}>
 <div style={{
 padding: "6px 12px",
 background: "var(--bg-secondary)",
 display: "flex",
 alignItems: "center",
 gap: "8px",
 flexWrap: "wrap",
 }}>
 <span style={{ fontWeight: "bold", color: side === "A" ? "var(--accent-color)" : "var(--accent-color)" }}>
 {side}
 </span>
 <span style={{ color: "var(--text-secondary)" }}>{resp.provider}</span>
 <span style={{ color: "var(--text-secondary)", fontSize: "11px" }}>{resp.model}</span>
 {resp.duration_ms > 0 && (
 <span style={{ marginLeft: "auto", color: "var(--text-secondary)", fontSize: "11px" }}>
 {resp.duration_ms}ms
 {resp.tokens != null && ` · ${resp.tokens} tok`}
 </span>
 )}
 </div>
 <div style={{ flex: 1, overflowY: "auto", padding: "10px 12px" }}>
 {isError ? (
 <span style={{ color: "var(--error-color)" }}>{resp.error}</span>
 ) : (
 <pre style={{ margin: 0, whiteSpace: "pre-wrap", fontFamily: "inherit", fontSize: "13px" }}>
 {resp.content || <span style={{ color: "var(--text-secondary)" }}>(empty response)</span>}
 </pre>
 )}
 </div>
 </div>
 );
}

function ProviderSelector({
 label, provider, model, onProvider, onModel,
}: {
 label: string; provider: string; model: string;
 onProvider: (v: string) => void; onModel: (v: string) => void;
}) {
 const { providers, modelsForProvider } = useModelRegistry();
 const listId = `multi-models-${label}`;
 return (
 <div style={{ display: "flex", alignItems: "center", gap: "8px", flex: 1 }}>
 <span style={{ color: "var(--text-secondary)", fontSize: "12px", minWidth: "14px" }}>{label}</span>
 <select
 value={provider}
 onChange={e => { onProvider(e.target.value); onModel(PROVIDER_DEFAULT_MODEL[e.target.value] ?? ""); }}
 className="panel-select"
 >
 {providers.map(p => <option key={p} value={p}>{p}</option>)}
 </select>
 <datalist id={listId}>
 {modelsForProvider(provider).map(m => <option key={m} value={m} />)}
 </datalist>
 <input
 value={model}
 onChange={e => onModel(e.target.value)}
 list={listId}
 placeholder="model"
 className="panel-input"
 style={{ flex: 1, minWidth: 0 }}
 />
 </div>
 );
}

export function MultiModelPanel() {
 const [prompt, setPrompt] = useState("");
 const [providerA, setProviderA] = useState("ollama");
 const [modelA, setModelA] = useState(PROVIDER_DEFAULT_MODEL.ollama ?? "");
 const [providerB, setProviderB] = useState("claude");
 const [modelB, setModelB] = useState(PROVIDER_DEFAULT_MODEL.claude ?? "");
 const [result, setResult] = useState<CompareResult | null>(null);
 const [loading, setLoading] = useState(false);
 const [error, setError] = useState<string | null>(null);

 const handleCompare = async () => {
 if (!prompt.trim()) return;
 setLoading(true);
 setError(null);
 setResult(null);
 try {
 const r = await invoke<CompareResult>("compare_models", {
 prompt: prompt.trim(),
 providerA, modelA, providerB, modelB,
 });
 setResult(r);
 } catch (e: unknown) {
 setError(String(e));
 } finally {
 setLoading(false);
 }
 };

 const handleKey = (e: React.KeyboardEvent) => {
 if ((e.ctrlKey || e.metaKey) && e.key === "Enter") handleCompare();
 };

 return (
 <div className="panel-container">
 {/* Header */}
 <div className="panel-header">
 <h3>Multi-Model Comparison</h3>
 </div>

 <div className="panel-body" style={{ display: "flex", flexDirection: "column", gap: 10 }}>
 {/* Provider selectors */}
 <div style={{ display: "flex", gap: "10px", flexWrap: "wrap" }}>
 <ProviderSelector label="A" provider={providerA} model={modelA} onProvider={setProviderA} onModel={setModelA} />
 <ProviderSelector label="B" provider={providerB} model={modelB} onProvider={setProviderB} onModel={setModelB} />
 </div>

 {/* Prompt input */}
 <textarea
 value={prompt}
 onChange={e => setPrompt(e.target.value)}
 onKeyDown={handleKey}
 placeholder="Enter a prompt… (Ctrl+Enter to send)"
 rows={4}
 className="panel-input panel-textarea panel-input-full"
 style={{ resize: "vertical" }}
 />

 <button
 onClick={handleCompare}
 disabled={loading || !prompt.trim()}
 className="panel-btn panel-btn-primary"
 style={{ alignSelf: "flex-start" }}
 >
 {loading ? "Comparing…" : "Compare"}
 </button>

 {error && (
 <div style={{ color: "var(--error-color)", fontSize: "12px" }}>{error}</div>
 )}

 {/* Side-by-side responses */}
 {result && (
 <div style={{ display: "flex", gap: "10px", flex: 1, overflow: "hidden", minHeight: "200px" }}>
 <ResponseCard resp={result.a} side="A" />
 <ResponseCard resp={result.b} side="B" />
 </div>
 )}

 {!result && !loading && (
 <div className="panel-empty">
 Enter a prompt above and click Compare<br />
 to see both models' responses side-by-side.
 </div>
 )}
 </div>
 </div>
 );
}
