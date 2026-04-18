import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface RouteResult {
  input_tokens: number;
  chosen_model: string;
  provider: string;
  cost_estimate_usd: number;
  reason: string;
}

interface ModelEntry {
  model_id: string;
  name: string;
  provider: string;
  max_tokens: number;
  cost_per_1k_input: number;
  cost_per_1k_output: number;
  supports_long_context: boolean;
}

interface IngestProgress {
  file_path: string;
  total_chunks: number;
  processed_chunks: number;
  status: string;
  error: string | null;
}

export function LongContextPanel() {
  const [tab, setTab] = useState("routing");
  const [models, setModels] = useState<ModelEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tokenCount, setTokenCount] = useState(32000);
  const [routeResult, setRouteResult] = useState<RouteResult | null>(null);
  const [routing, setRouting] = useState(false);
  const [filePath, setFilePath] = useState("");
  const [ingestProgress, setIngestProgress] = useState<IngestProgress | null>(null);
  const [ingesting, setIngesting] = useState(false);

  useEffect(() => {
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const res = await invoke<ModelEntry[]>("long_context_models");
        setModels(Array.isArray(res) ? res : []);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  async function runRoute() {
    setRouting(true);
    setRouteResult(null);
    try {
      const res = await invoke<RouteResult>("long_context_route", { tokenCount });
      setRouteResult(res ?? null);
    } catch (e) {
      setError(String(e));
    } finally {
      setRouting(false);
    }
  }

  async function runIngest() {
    if (!filePath.trim()) return;
    setIngesting(true);
    setIngestProgress(null);
    try {
      const res = await invoke<IngestProgress>("long_context_ingest", { filePath: filePath.trim() });
      setIngestProgress(res ?? null);
    } catch (e) {
      setIngestProgress({ file_path: filePath, total_chunks: 0, processed_chunks: 0, status: "failed", error: String(e) });
    } finally {
      setIngesting(false);
    }
  }

  const formatTokens = (n: number) => n >= 1000 ? `${(n / 1000).toFixed(0)}k` : String(n);

  return (
    <div className="panel-container" style={{ padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono)", flex: 1, minHeight: 0, overflowY: "auto" }}>
      <div style={{ fontSize: "var(--font-size-xl)", fontWeight: 700, marginBottom: 12 }}>Long Context Router</div>

      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        {["routing", "models", "ingest"].map(t => (
          <button key={t} onClick={() => setTab(t)} style={{ padding: "4px 12px", borderRadius: "var(--radius-sm)", cursor: "pointer", background: tab === t ? "var(--accent-color)" : "var(--bg-secondary)", color: tab === t ? "var(--btn-primary-fg)" : "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)" }}>{t}</button>
        ))}
      </div>

      {loading && <div className="panel-loading" style={{ color: "var(--text-muted)" }}>Loading...</div>}
      {error && <div style={{ color: "var(--error-color)", marginBottom: 8 }}>{error}</div>}

      {!loading && tab === "routing" && (
        <div style={{ maxWidth: 520 }}>
          <div style={{ marginBottom: 20 }}>
            <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 6 }}>
              Input Token Count: <strong style={{ color: "var(--text-primary)" }}>{formatTokens(tokenCount)}</strong>
            </label>
            <input type="range" min={1000} max={2000000} step={1000} value={tokenCount} onChange={e => setTokenCount(Number(e.target.value))}
              style={{ width: "100%", accentColor: "var(--accent-color)" }} />
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-xs)", color: "var(--text-muted)" }}>
              <span>1k</span><span>2M</span>
            </div>
          </div>
          <button className="panel-btn" onClick={runRoute} disabled={routing}
            style={{ padding: "8px 24px", borderRadius: "var(--radius-sm)", cursor: routing ? "not-allowed" : "pointer", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontSize: "var(--font-size-md)", fontWeight: 600, opacity: routing ? 0.6 : 1, marginBottom: 20 }}>
            {routing ? "Routing…" : "Find Best Model"}
          </button>
          {routeResult && (
            <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-md)", border: "1px solid var(--border-color)", padding: 16 }}>
              <div style={{ fontSize: "var(--font-size-md)", fontWeight: 700, color: "var(--accent-color)", marginBottom: 10 }}>{routeResult.chosen_model}</div>
              <div style={{ display: "grid", gridTemplateColumns: "130px 1fr", rowGap: 8, fontSize: "var(--font-size-base)" }}>
                {[
                  ["Provider", routeResult.provider],
                  ["Input Tokens", formatTokens(routeResult.input_tokens)],
                  ["Cost Estimate", `$${routeResult.cost_estimate_usd.toFixed(4)}`],
                  ["Reason", routeResult.reason],
                ].map(([label, value]) => (
                  <>
                    <span key={`l-${label}`} style={{ color: "var(--text-muted)" }}>{label}</span>
                    <span key={`v-${label}`}>{value}</span>
                  </>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {!loading && tab === "models" && (
        <div style={{ overflowX: "auto" }}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" }}>
            <thead>
              <tr style={{ background: "var(--bg-secondary)" }}>
                {["Model", "Provider", "Max Tokens", "$/1k In", "$/1k Out", "Long Ctx"].map(h => (
                  <th key={h} style={{ padding: "8px 12px", textAlign: "left", borderBottom: "1px solid var(--border-color)", color: "var(--text-muted)", fontWeight: 600, whiteSpace: "nowrap" }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {models.length === 0 && (
                <tr><td colSpan={6} style={{ padding: 16, color: "var(--text-muted)", textAlign: "center" }}>No models found.</td></tr>
              )}
              {models.map(m => (
                <tr key={m.model_id} style={{ borderBottom: "1px solid var(--border-color)" }}>
                  <td style={{ padding: "8px 12px", fontWeight: 600 }}>{m.name}</td>
                  <td style={{ padding: "8px 12px", color: "var(--text-muted)" }}>{m.provider}</td>
                  <td style={{ padding: "8px 12px" }}>{formatTokens(m.max_tokens)}</td>
                  <td style={{ padding: "8px 12px" }}>${m.cost_per_1k_input.toFixed(4)}</td>
                  <td style={{ padding: "8px 12px" }}>${m.cost_per_1k_output.toFixed(4)}</td>
                  <td style={{ padding: "8px 12px" }}>
                    <span style={{ fontSize: "var(--font-size-sm)", color: m.supports_long_context ? "var(--success-color)" : "var(--text-muted)" }}>
                      {m.supports_long_context ? "Yes" : "No"}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {!loading && tab === "ingest" && (
        <div style={{ maxWidth: 520 }}>
          <div style={{ marginBottom: 14 }}>
            <label style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 6 }}>File Path</label>
            <input value={filePath} onChange={e => setFilePath(e.target.value)}
              placeholder="/path/to/large/document.txt"
              style={{ width: "100%", padding: "8px 12px", borderRadius: "var(--radius-sm)", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)", boxSizing: "border-box" }} />
          </div>
          <button className="panel-btn" onClick={runIngest} disabled={ingesting || !filePath.trim()}
            style={{ padding: "8px 24px", borderRadius: "var(--radius-sm)", cursor: ingesting || !filePath.trim() ? "not-allowed" : "pointer", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontSize: "var(--font-size-md)", fontWeight: 600, opacity: ingesting || !filePath.trim() ? 0.6 : 1, marginBottom: 20 }}>
            {ingesting ? "Ingesting…" : "Start Ingest"}
          </button>
          {ingestProgress && (
            <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-md)", border: "1px solid var(--border-color)", padding: 16 }}>
              <div style={{ fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 8, wordBreak: "break-all" }}>{ingestProgress.file_path}</div>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-base)", marginBottom: 6 }}>
                <span>Progress</span>
                <span style={{ color: "var(--text-muted)" }}>{ingestProgress.processed_chunks} / {ingestProgress.total_chunks} chunks</span>
              </div>
              <div style={{ height: 8, background: "var(--bg-primary)", borderRadius: "var(--radius-xs-plus)", marginBottom: 10 }}>
                <div style={{
                  flex: 1, minHeight: 0,
                  width: ingestProgress.total_chunks > 0 ? `${(ingestProgress.processed_chunks / ingestProgress.total_chunks) * 100}%` : "0%",
                  background: ingestProgress.status === "failed" ? "var(--error-color)" : "var(--accent-color)",
                  borderRadius: "var(--radius-xs-plus)",
                  transition: "width 0.3s ease"
                }} />
              </div>
              <div style={{ fontSize: "var(--font-size-base)" }}>
                Status: <span style={{ color: ingestProgress.status === "completed" ? "var(--success-color)" : ingestProgress.status === "failed" ? "var(--error-color)" : "var(--warning-color)" }}>{ingestProgress.status}</span>
              </div>
              {ingestProgress.error && <div style={{ fontSize: "var(--font-size-base)", color: "var(--error-color)", marginTop: 6 }}>{ingestProgress.error}</div>}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
