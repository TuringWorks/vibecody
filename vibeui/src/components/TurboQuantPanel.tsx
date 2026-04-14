/**
 * TurboQuantPanel — Vector compression dashboard using TurboQuant
 * (PolarQuant + QJL two-stage quantization from Google Research).
 *
 * Tabs:
 * Overview : compression stats, bits/dimension, ratio, vector count
 * Compress : input vectors, compress & add to index
 * Search   : query the compressed index via cosine similarity
 * Benchmark: compare compressed vs uncompressed recall & storage
 *
 * Backed by Tauri commands: turboquant_stats, turboquant_insert,
 * turboquant_search, turboquant_benchmark, turboquant_clear.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types ─────────────────────────────────────────────────────────────────────

interface TurboQuantStats {
  num_vectors: number;
  dimension: number;
  compressed_bytes: number;
  uncompressed_bytes: number;
  compression_ratio: number;
  bits_per_dimension: number;
}

interface SearchResult {
  id: string;
  score: number;
  metadata: Record<string, string>;
}

interface BenchmarkResult {
  num_vectors: number;
  dimension: number;
  compressed_bytes: number;
  uncompressed_bytes: number;
  compression_ratio: number;
  recall_at_10: number;
  avg_query_ms: number;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

const fmtBytes = (n: number) =>
  n >= 1_048_576 ? `${(n / 1_048_576).toFixed(1)} MB` :
  n >= 1_024 ? `${(n / 1_024).toFixed(1)} KB` : `${n} B`;

const fmtRatio = (r: number) => r > 0 ? `${r.toFixed(1)}×` : "—";

const statBox = (label: string, value: string, color = "var(--text-secondary)") => (
  <div style={{ textAlign: "center", padding: "12px 16px", background: "var(--bg-secondary)", borderRadius: "var(--radius-sm-alt)", minWidth: 100 }}>
    <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-tertiary)", marginBottom: 4 }}>{label}</div>
    <div style={{ fontSize: 18, fontWeight: 600, color }}>{value}</div>
  </div>
);

const btn = (label: string, onClick: () => void, disabled = false, accent = false) => (
  <button
    onClick={onClick}
    disabled={disabled}
    style={{
      padding: "6px 14px",
      borderRadius: "var(--radius-sm)",
      border: accent ? "none" : "1px solid var(--border-color)",
      background: accent ? "var(--accent-color)" : "var(--bg-secondary)",
      color: accent ? "#fff" : "var(--text-primary)",
      cursor: disabled ? "not-allowed" : "pointer",
      opacity: disabled ? 0.5 : 1,
      fontSize: "var(--font-size-base)",
      fontFamily: "inherit",
    }}
  >
    {label}
  </button>
);

// ── Tabs ──────────────────────────────────────────────────────────────────────

type Tab = "overview" | "compress" | "search" | "benchmark";

export function TurboQuantPanel() {
  const [tab, setTab] = useState<Tab>("overview");
  const [stats, setStats] = useState<TurboQuantStats | null>(null);
  const [loading, setLoading] = useState(false);

  // ── Compress state
  const [compressId, setCompressId] = useState("");
  const [compressVector, setCompressVector] = useState("");
  const [compressStatus, setCompressStatus] = useState("");

  // ── Search state
  const [searchVector, setSearchVector] = useState("");
  const [searchK, setSearchK] = useState(10);
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [searching, setSearching] = useState(false);

  // ── Benchmark state
  const [benchResult, setBenchResult] = useState<BenchmarkResult | null>(null);
  const [benchN, setBenchN] = useState(500);
  const [benchDim, setBenchDim] = useState(128);
  const [benchRunning, setBenchRunning] = useState(false);

  const loadStats = useCallback(async () => {
    setLoading(true);
    try {
      const s = await invoke<TurboQuantStats>("turboquant_stats");
      setStats(s);
    } catch { /* ignore */ } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadStats(); }, [loadStats]);

  // ── Compress handler
  const handleCompress = async () => {
    try {
      const vector = compressVector.split(",").map(s => parseFloat(s.trim()));
      if (vector.some(isNaN)) { setCompressStatus("Invalid vector (comma-separated floats)"); return; }
      await invoke("turboquant_insert", { id: compressId || `v${Date.now()}`, vector });
      setCompressStatus(`Inserted "${compressId || "auto"}" (${vector.length}-dim)`);
      setCompressId("");
      setCompressVector("");
      await loadStats();
    } catch (e) {
      setCompressStatus(`Error: ${e}`);
    }
  };

  // ── Search handler
  const handleSearch = async () => {
    setSearching(true);
    try {
      const vector = searchVector.split(",").map(s => parseFloat(s.trim()));
      const results = await invoke<SearchResult[]>("turboquant_search", { vector, topK: searchK });
      setSearchResults(results);
    } catch { /* ignore */ } finally {
      setSearching(false);
    }
  };

  // ── Benchmark handler
  const handleBenchmark = async () => {
    setBenchRunning(true);
    try {
      const result = await invoke<BenchmarkResult>("turboquant_benchmark", { numVectors: benchN, dimension: benchDim });
      setBenchResult(result);
    } catch { /* ignore */ } finally {
      setBenchRunning(false);
    }
  };

  // ── Clear handler
  const handleClear = async () => {
    await invoke("turboquant_clear");
    await loadStats();
  };

  const tabBtn = (t: Tab, label: string) => (
    <button
      onClick={() => setTab(t)}
      className={`panel-tab ${tab === t ? "active" : ""}`}
    >
      {label}
    </button>
  );

  return (
    <div className="panel-container" style={{ fontFamily: "var(--font-mono, monospace)", fontSize: "var(--font-size-md)" }}>
      <div className="panel-header">
        <h3>TurboQuant</h3>
        <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-tertiary)" }}>PolarQuant + QJL ~3 bits/dim</span>
      </div>

      {/* Tab bar */}
      <div className="panel-tab-bar">
        {tabBtn("overview", "Overview")}
        {tabBtn("compress", "Compress")}
        {tabBtn("search", "Search")}
        {tabBtn("benchmark", "Benchmark")}
      </div>

      <div className="panel-body">
      {/* ── Overview ──────────────────────────────────────────────────── */}
      {tab === "overview" && (
        <div>
          {loading && <div style={{ color: "var(--text-tertiary)" }}>Loading...</div>}
          {stats && (
            <>
              <div style={{ display: "flex", gap: 12, flexWrap: "wrap", marginBottom: 16 }}>
                {statBox("Vectors", String(stats.num_vectors), "var(--accent-color)")}
                {statBox("Dimension", String(stats.dimension))}
                {statBox("Compressed", fmtBytes(stats.compressed_bytes), "var(--success-color)")}
                {statBox("Uncompressed", fmtBytes(stats.uncompressed_bytes))}
                {statBox("Ratio", fmtRatio(stats.compression_ratio), "var(--success-color)")}
                {statBox("Bits/Dim", stats.bits_per_dimension > 0 ? stats.bits_per_dimension.toFixed(1) : "—")}
              </div>

              {/* Visual bar */}
              {stats.num_vectors > 0 && (
                <div style={{ marginBottom: 16 }}>
                  <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-tertiary)", marginBottom: 4 }}>Storage comparison</div>
                  <div style={{ display: "flex", height: 20, borderRadius: "var(--radius-xs-plus)", overflow: "hidden", background: "var(--bg-tertiary)" }}>
                    <div style={{
                      width: `${Math.min(100 / stats.compression_ratio, 100)}%`,
                      background: "var(--success-color)",
                      display: "flex", alignItems: "center", justifyContent: "center",
                      fontSize: "var(--font-size-xs)", color: "var(--btn-primary-fg, #fff)", fontWeight: 600,
                    }}>
                      TQ
                    </div>
                    <div style={{
                      flex: 1,
                      background: "var(--warning-color)",
                      display: "flex", alignItems: "center", justifyContent: "center",
                      fontSize: "var(--font-size-xs)", color: "var(--btn-primary-fg, #fff)", fontWeight: 600, opacity: 0.6,
                    }}>
                      f32
                    </div>
                  </div>
                </div>
              )}

              <div style={{ display: "flex", gap: 8 }}>
                {btn("Refresh", loadStats)}
                {btn("Clear Index", handleClear, stats.num_vectors === 0)}
              </div>
            </>
          )}
        </div>
      )}

      {/* ── Compress ──────────────────────────────────────────────────── */}
      {tab === "compress" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
          <label style={{ fontSize: "var(--font-size-sm)", color: "var(--text-tertiary)" }}>ID (optional)</label>
          <input
            value={compressId}
            onChange={e => setCompressId(e.target.value)}
            placeholder="auto-generated if empty"
            style={{ padding: "6px 10px", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontFamily: "inherit", fontSize: "var(--font-size-base)" }}
          />
          <label style={{ fontSize: "var(--font-size-sm)", color: "var(--text-tertiary)" }}>Vector (comma-separated floats)</label>
          <textarea
            value={compressVector}
            onChange={e => setCompressVector(e.target.value)}
            rows={4}
            placeholder="0.12, -0.45, 0.78, ..."
            style={{ padding: "6px 10px", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontFamily: "inherit", fontSize: "var(--font-size-base)", resize: "vertical" }}
          />
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            {btn("Compress & Insert", handleCompress, !compressVector.trim(), true)}
            {compressStatus && <span style={{ fontSize: "var(--font-size-sm)", color: compressStatus.startsWith("Error") ? "var(--error-color)" : "var(--success-color)" }}>{compressStatus}</span>}
          </div>
        </div>
      )}

      {/* ── Search ────────────────────────────────────────────────────── */}
      {tab === "search" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
          <label style={{ fontSize: "var(--font-size-sm)", color: "var(--text-tertiary)" }}>Query vector (comma-separated)</label>
          <textarea
            value={searchVector}
            onChange={e => setSearchVector(e.target.value)}
            rows={3}
            placeholder="query: 0.12, -0.45, 0.78, ..."
            style={{ padding: "6px 10px", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontFamily: "inherit", fontSize: "var(--font-size-base)", resize: "vertical" }}
          />
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <label style={{ fontSize: "var(--font-size-sm)", color: "var(--text-tertiary)" }}>Top-K:</label>
            <input type="number" value={searchK} onChange={e => setSearchK(Number(e.target.value))} min={1} max={100}
              style={{ width: 60, padding: "4px 8px", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontFamily: "inherit", fontSize: "var(--font-size-base)" }}
            />
            {btn("Search", handleSearch, !searchVector.trim() || searching, true)}
          </div>

          {searchResults.length > 0 && (
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "var(--font-size-base)" }}>
              <thead>
                <tr style={{ borderBottom: "1px solid var(--border-color)", color: "var(--text-tertiary)" }}>
                  <th style={{ textAlign: "left", padding: "6px 8px" }}>Rank</th>
                  <th style={{ textAlign: "left", padding: "6px 8px" }}>ID</th>
                  <th style={{ textAlign: "right", padding: "6px 8px" }}>Score</th>
                </tr>
              </thead>
              <tbody>
                {searchResults.map((r, i) => (
                  <tr key={r.id} style={{ borderBottom: "1px solid var(--border-color)" }}>
                    <td style={{ padding: "6px 8px", color: "var(--text-tertiary)" }}>{i + 1}</td>
                    <td style={{ padding: "6px 8px" }}>{r.id}</td>
                    <td style={{ padding: "6px 8px", textAlign: "right", color: r.score > 0.8 ? "var(--success-color)" : "var(--text-secondary)" }}>
                      {r.score.toFixed(4)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}

      {/* ── Benchmark ─────────────────────────────────────────────────── */}
      {tab === "benchmark" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
          <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-tertiary)" }}>
            Generate random vectors, compress with TurboQuant, and measure recall + compression.
          </div>
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <label style={{ fontSize: "var(--font-size-sm)" }}>Vectors:</label>
            <input type="number" value={benchN} onChange={e => setBenchN(Number(e.target.value))} min={10} max={10000}
              style={{ width: 80, padding: "4px 8px", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontFamily: "inherit", fontSize: "var(--font-size-base)" }}
            />
            <label style={{ fontSize: "var(--font-size-sm)" }}>Dimension:</label>
            <input type="number" value={benchDim} onChange={e => setBenchDim(Number(e.target.value))} min={8} max={4096}
              style={{ width: 80, padding: "4px 8px", borderRadius: "var(--radius-sm)", border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-primary)", fontFamily: "inherit", fontSize: "var(--font-size-base)" }}
            />
            {btn("Run Benchmark", handleBenchmark, benchRunning, true)}
          </div>

          {benchResult && (
            <div style={{ display: "flex", gap: 12, flexWrap: "wrap", marginTop: 8 }}>
              {statBox("Vectors", String(benchResult.num_vectors))}
              {statBox("Dimension", String(benchResult.dimension))}
              {statBox("Compressed", fmtBytes(benchResult.compressed_bytes), "var(--success-color)")}
              {statBox("Uncompressed", fmtBytes(benchResult.uncompressed_bytes))}
              {statBox("Ratio", fmtRatio(benchResult.compression_ratio), "var(--success-color)")}
              {statBox("Recall@10", `${(benchResult.recall_at_10 * 100).toFixed(0)}%`,
                benchResult.recall_at_10 >= 0.7 ? "var(--success-color)" : "var(--warning-color)")}
              {statBox("Avg Query", `${benchResult.avg_query_ms.toFixed(1)}ms`)}
            </div>
          )}
        </div>
      )}
      </div>
    </div>
  );
}
