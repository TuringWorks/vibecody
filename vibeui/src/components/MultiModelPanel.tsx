import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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

const PROVIDERS = ["ollama", "claude", "openai", "gemini", "grok", "groq"];

const DEFAULT_MODELS: Record<string, string> = {
  ollama: "codellama",
  claude: "claude-sonnet-4-6",
  openai: "gpt-4o",
  gemini: "gemini-2.0-flash",
  grok: "grok-2",
  groq: "llama-3.3-70b-versatile",
};

function ResponseCard({ resp, side }: { resp: ModelResponse; side: "A" | "B" }) {
  const isError = !!resp.error;
  return (
    <div style={{
      flex: 1,
      display: "flex",
      flexDirection: "column",
      border: `1px solid ${isError ? "rgba(244,67,54,0.4)" : "var(--border, #444)"}`,
      borderRadius: "6px",
      overflow: "hidden",
      minWidth: 0,
    }}>
      <div style={{
        padding: "6px 12px",
        background: "var(--bg-secondary, #2d2d2d)",
        display: "flex",
        alignItems: "center",
        gap: "8px",
        flexWrap: "wrap",
      }}>
        <span style={{ fontWeight: "bold", color: side === "A" ? "#4fc3f7" : "#ce93d8" }}>
          {side}
        </span>
        <span style={{ color: "var(--text-secondary, #ccc)" }}>{resp.provider}</span>
        <span style={{ color: "var(--text-muted, #888)", fontSize: "11px" }}>{resp.model}</span>
        {resp.duration_ms > 0 && (
          <span style={{ marginLeft: "auto", color: "var(--text-muted, #888)", fontSize: "11px" }}>
            {resp.duration_ms}ms
            {resp.tokens != null && ` · ${resp.tokens} tok`}
          </span>
        )}
      </div>
      <div style={{ flex: 1, overflowY: "auto", padding: "10px 12px" }}>
        {isError ? (
          <span style={{ color: "#f44336" }}>{resp.error}</span>
        ) : (
          <pre style={{ margin: 0, whiteSpace: "pre-wrap", fontFamily: "inherit", fontSize: "13px" }}>
            {resp.content || <span style={{ color: "var(--text-muted, #888)" }}>(empty response)</span>}
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
  return (
    <div style={{ display: "flex", alignItems: "center", gap: "8px", flex: 1 }}>
      <span style={{ color: "var(--text-muted, #888)", fontSize: "12px", minWidth: "14px" }}>{label}</span>
      <select
        value={provider}
        onChange={e => { onProvider(e.target.value); onModel(DEFAULT_MODELS[e.target.value] ?? ""); }}
        style={{ background: "var(--bg-secondary, #2d2d2d)", color: "var(--text, #fff)", border: "1px solid var(--border, #444)", borderRadius: "4px", padding: "3px 6px", fontSize: "12px" }}
      >
        {PROVIDERS.map(p => <option key={p} value={p}>{p}</option>)}
      </select>
      <input
        value={model}
        onChange={e => onModel(e.target.value)}
        placeholder="model"
        style={{ flex: 1, background: "var(--bg-secondary, #2d2d2d)", color: "var(--text, #fff)", border: "1px solid var(--border, #444)", borderRadius: "4px", padding: "3px 6px", fontSize: "12px", minWidth: 0 }}
      />
    </div>
  );
}

export function MultiModelPanel() {
  const [prompt, setPrompt] = useState("");
  const [providerA, setProviderA] = useState("ollama");
  const [modelA, setModelA] = useState(DEFAULT_MODELS.ollama);
  const [providerB, setProviderB] = useState("claude");
  const [modelB, setModelB] = useState(DEFAULT_MODELS.claude);
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
    <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "12px", gap: "10px", fontFamily: "monospace", fontSize: "13px" }}>
      {/* Header */}
      <div style={{ fontWeight: "bold", marginBottom: "2px" }}>⚖️ Multi-Model Comparison</div>

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
        style={{
          resize: "vertical",
          background: "var(--bg-secondary, #2d2d2d)",
          color: "var(--text, #fff)",
          border: "1px solid var(--border, #444)",
          borderRadius: "4px",
          padding: "8px",
          fontFamily: "inherit",
          fontSize: "13px",
        }}
      />

      <button
        onClick={handleCompare}
        disabled={loading || !prompt.trim()}
        style={{
          alignSelf: "flex-start",
          background: loading ? "var(--bg-secondary, #2d2d2d)" : "var(--accent, #007acc)",
          color: "#fff", border: "none", borderRadius: "4px",
          padding: "6px 18px", cursor: loading ? "default" : "pointer",
        }}
      >
        {loading ? "⏳ Comparing…" : "▶ Compare"}
      </button>

      {error && (
        <div style={{ color: "#f44336", fontSize: "12px" }}>{error}</div>
      )}

      {/* Side-by-side responses */}
      {result && (
        <div style={{ display: "flex", gap: "10px", flex: 1, overflow: "hidden", minHeight: "200px" }}>
          <ResponseCard resp={result.a} side="A" />
          <ResponseCard resp={result.b} side="B" />
        </div>
      )}

      {!result && !loading && (
        <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", color: "var(--text-muted, #888)", textAlign: "center" }}>
          Enter a prompt above and click Compare<br />
          to see both models' responses side-by-side.
        </div>
      )}
    </div>
  );
}
