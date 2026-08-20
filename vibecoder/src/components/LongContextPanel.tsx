import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

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

/**
 * What the router last said, as one value.
 *
 * A failure carries the token count it was about. "No configured model has a
 * context window large enough for 1,010,000 tokens" is a statement about
 * 1,010,000 tokens and about nothing else: once the slider moves, nobody has
 * checked the new number, and leaving the banner up asserts a result that was
 * never measured. Kept as separate `routing` / `result` / `error` flags it
 * outlived both the input it described and the tab it belonged to.
 */
type RouteState =
  | { kind: "idle" }
  | { kind: "routing" }
  | { kind: "chosen"; result: RouteResult }
  | { kind: "failed"; message: string; tokenCount: number };

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
  const [route, setRoute] = useState<RouteState>({ kind: "idle" });
  const [filePath, setFilePath] = useState("");
  const [ingestProgress, setIngestProgress] = useState<IngestProgress | null>(null);
  const [ingesting, setIngesting] = useState(false);
  const [pickError, setPickError] = useState<string | null>(null);

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
    setRoute({ kind: "routing" });
    try {
      const res = await invoke<RouteResult>("long_context_route", { tokenCount });
      // A command that answers with nothing chose nothing — that is idle, not
      // a choice and not a failure.
      setRoute(res ? { kind: "chosen", result: res } : { kind: "idle" });
    } catch (e) {
      setRoute({ kind: "failed", message: String(e), tokenCount });
    }
  }

  /**
   * Pick the file to ingest from the OS file dialog.
   *
   * Typing an absolute path by hand was the only way in, so a typo surfaced as
   * an ingest failure rather than as a file that was never there.
   */
  async function browseForFile() {
    setPickError(null);
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        title: "Select a file to ingest",
        filters: [
          { name: "All Files", extensions: ["*"] },
          { name: "Documents", extensions: ["txt", "md", "pdf", "csv", "json", "xml", "log", "html"] },
          { name: "Code", extensions: ["rs", "py", "js", "ts", "tsx", "jsx", "go", "java", "c", "cpp", "rb", "swift", "kt", "sql", "yaml", "toml"] },
        ],
      });
      // `null` is a cancelled dialog — leave whatever is already typed alone.
      if (typeof selected === "string") setFilePath(selected);
    } catch (e) {
      setPickError(`Could not open the file dialog: ${e}`);
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
    <div className="panel-container">
      <div className="panel-header"><h3>Long Context Router</h3></div>
      <div className="panel-tab-bar">
        {["routing", "models", "ingest"].map(t => (
          <button className={`panel-tab${tab === t ? " active" : ""}`} key={t} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>
      <div className="panel-body">
      {loading && <div className="panel-loading">Loading...</div>}
      {/* Only the model list's own failure belongs to the whole panel. A
          routing failure shown here followed the reader onto the models and
          ingest tabs, where it described nothing on screen. */}
      {error && <div className="panel-error"><span>{error}</span></div>}

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
          <button className="panel-btn" onClick={runRoute} disabled={route.kind === "routing"}
            style={{ padding: "8px 24px", borderRadius: "var(--radius-sm)", cursor: route.kind === "routing" ? "not-allowed" : "pointer", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", border: "none", fontSize: "var(--font-size-md)", fontWeight: 600, opacity: route.kind === "routing" ? 0.6 : 1, marginBottom: 20 }}>
            {route.kind === "routing" ? "Routing…" : "Find Best Model"}
          </button>
          {/* The failure is shown only while the slider still reads the number
              it was about; move the slider and it is a claim about a count
              nobody routed. */}
          {route.kind === "failed" && route.tokenCount === tokenCount && (
            <div className="panel-error" style={{ marginBottom: 20 }}><span>{route.message}</span></div>
          )}
          {route.kind === "chosen" && (
            <div style={{ background: "var(--bg-secondary)", borderRadius: "var(--radius-md)", border: "1px solid var(--border-color)", padding: 16 }}>
              <div style={{ fontSize: "var(--font-size-md)", fontWeight: 700, color: "var(--accent-color)", marginBottom: 10 }}>{route.result.chosen_model}</div>
              <div style={{ display: "grid", gridTemplateColumns: "130px 1fr", rowGap: 8, fontSize: "var(--font-size-base)" }}>
                {[
                  ["Provider", route.result.provider],
                  ["Input Tokens", formatTokens(route.result.input_tokens)],
                  ["Cost Estimate", `$${route.result.cost_estimate_usd.toFixed(4)}`],
                  ["Reason", route.result.reason],
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
            <label htmlFor="long-context-file-path" style={{ display: "block", fontSize: "var(--font-size-base)", color: "var(--text-muted)", marginBottom: 6 }}>File Path</label>
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <input id="long-context-file-path" value={filePath} onChange={e => setFilePath(e.target.value)}
                placeholder="/path/to/large/document.txt"
                style={{ flex: 1, minWidth: 0, padding: "8px 12px", borderRadius: "var(--radius-sm)", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)", boxSizing: "border-box" }} />
              <button className="panel-btn" onClick={browseForFile} disabled={ingesting}
                style={{ flex: "0 0 auto", padding: "8px 16px", borderRadius: "var(--radius-sm)", cursor: ingesting ? "not-allowed" : "pointer", background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", fontSize: "var(--font-size-base)", opacity: ingesting ? 0.6 : 1 }}>
                Browse…
              </button>
            </div>
            {pickError && (
              <div role="alert" style={{ marginTop: 6, fontSize: "var(--font-size-base)", color: "var(--error-color)" }}>{pickError}</div>
            )}
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
    </div>
  );
}
