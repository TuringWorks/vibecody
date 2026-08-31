/**
 * DocumentIngestPanel — file/directory ingestion with chunking configuration.
 *
 * Tabs: Ingest (file path + format + actions), Config (chunking parameters)
 */
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Tab = "ingest" | "config";
type Format = "auto" | "txt" | "md" | "html" | "pdf" | "docx" | "csv" | "json" | "rs";

/** One document the backend actually read and chunked. */
interface IngestResult {
  id: string;
  title: string;
  path: string;
  format: string;
  chunks: number;
  word_count: number;
  char_count: number;
  /** Words × 1.3 — the chunker's own approximation, not a tokenizer count. */
  estimated_tokens: number;
  warnings: string[];
}

/** A file the walk found but could not read. Shown, never silently dropped. */
interface SkippedDoc {
  path: string;
  reason: string;
}

interface DirectoryIngestResult {
  documents: IngestResult[];
  skipped: SkippedDoc[];
  files_seen: number;
}

interface ChunkingConfig {
  maxTokens: number;
  overlap: number;
  minChunkSize: number;
  sentenceBoundary: boolean;
  sectionTitle: boolean;
}

export function DocumentIngestPanel() {
  const [tab, setTab] = useState<Tab>("ingest");
  const [filePath, setFilePath] = useState("");
  const [format, setFormat] = useState<Format>("auto");
  const [results, setResults] = useState<IngestResult[]>([]);
  const [skipped, setSkipped] = useState<SkippedDoc[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [config, setConfig] = useState<ChunkingConfig>({
    maxTokens: 512,
    overlap: 50,
    minChunkSize: 64,
    sentenceBoundary: true,
    sectionTitle: true,
  });

  /** The backend's field names, so the chunker gets what the sliders say. */
  const chunkingArgs = () => ({
    max_tokens: config.maxTokens,
    overlap_tokens: config.overlap,
    min_chunk_size: config.minChunkSize,
    respect_boundaries: config.sentenceBoundary,
    include_metadata: config.sectionTitle,
  });

  const handleIngestFile = async () => {
    if (!filePath.trim()) return;
    setIsLoading(true);
    setError(null);
    try {
      const result = await invoke<IngestResult>("ingest_document", {
        path: filePath.trim(),
        format: format === "auto" ? null : format,
        config: chunkingArgs(),
      });
      setResults((prev) => [result, ...prev]);
      setSkipped([]);
    } catch (e) {
      setError(String(e));
    } finally {
      setIsLoading(false);
    }
  };

  const handleIngestDirectory = async () => {
    if (!filePath.trim()) return;
    setIsLoading(true);
    setError(null);
    try {
      const res = await invoke<DirectoryIngestResult>("ingest_document_directory", {
        path: filePath.trim(),
        // "auto" walks every file the ignore list allows; a chosen format also
        // filters the walk to that extension, which is what picking one means.
        extensions: format === "auto" ? [] : [format],
        format: format === "auto" ? null : format,
        config: chunkingArgs(),
      });
      setResults((prev) => [...res.documents, ...prev]);
      setSkipped(res.skipped);
    } catch (e) {
      setError(String(e));
    } finally {
      setIsLoading(false);
    }
  };

  const tabs: { key: Tab; label: string }[] = [
    { key: "ingest", label: "Ingest" },
    { key: "config", label: "Config" },
  ];

  return (
    <div className="panel-container" style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0 }}>
      {/* Tab bar */}
      <div className="panel-tab-bar">
        {tabs.map((t) => (
          <button
            key={t.key}
            onClick={() => setTab(t.key)}
            className={`panel-tab${tab === t.key ? " active" : ""}`}
          >
            {t.label}
          </button>
        ))}
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
        {tab === "ingest" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            {/* File path input */}
            <div>
              <label htmlFor="doc-ingest-path" style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>File or Directory Path</label>
              <input
                id="doc-ingest-path"
                value={filePath}
                onChange={(e) => setFilePath(e.target.value)}
                placeholder="/path/to/document.pdf or /path/to/directory"
                style={{ width: "100%", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", padding: "8px 8px", fontSize: "var(--font-size-base)", boxSizing: "border-box" }}
              />
            </div>

            {/* Format dropdown */}
            <div>
              <label htmlFor="doc-ingest-format" style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>Format</label>
              <select
                id="doc-ingest-format"
                value={format}
                onChange={(e) => setFormat(e.target.value as Format)}
                style={{ width: "100%", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", padding: "8px 8px", fontSize: "var(--font-size-base)", boxSizing: "border-box" }}
              >
                <option value="auto">Auto-detect</option>
                <option value="txt">Plain Text</option>
                <option value="md">Markdown</option>
                <option value="html">HTML</option>
                <option value="pdf">PDF</option>
                <option value="docx">DOCX</option>
                <option value="csv">CSV</option>
                <option value="json">JSON</option>
                <option value="rs">Source Code</option>
              </select>
            </div>

            {/* Action buttons */}
            <div style={{ display: "flex", gap: 8 }}>
              <button
                onClick={handleIngestFile}
                disabled={isLoading || !filePath.trim()}
                className="panel-btn panel-btn-primary"
                style={{ flex: 1, opacity: isLoading || !filePath.trim() ? 0.5 : 1 }}
              >
                {isLoading ? "Ingesting..." : "Ingest File"}
              </button>
              <button
                onClick={handleIngestDirectory}
                disabled={isLoading || !filePath.trim()}
                className="panel-btn panel-btn-secondary"
                style={{ flex: 1, opacity: isLoading || !filePath.trim() ? 0.5 : 1 }}
              >
                Ingest Directory
              </button>
            </div>

            {error && (
              <div style={{ color: "var(--error-color)", fontSize: "var(--font-size-sm)" }}>{error}</div>
            )}

            {/* Results */}
            {results.length > 0 && (
              <div style={{ marginTop: 8 }}>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 8 }}>
                  {results.length} document(s) ingested · token counts are estimates (words × 1.3), not tokenizer output
                </div>
                {results.map((r) => (
                  <div
                    key={r.id}
                    className="panel-card"
                    style={{ marginBottom: 6 }}
                  >
                    <div style={{ fontWeight: 600, fontSize: "var(--font-size-base)", marginBottom: 2 }}>{r.title}</div>
                    <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", fontFamily: "var(--font-mono)", marginBottom: 4, wordBreak: "break-all" }}>{r.path}</div>
                    <div style={{ display: "flex", gap: 16, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", flexWrap: "wrap" }}>
                      <span>{r.chunks} chunks</span>
                      <span>~{r.estimated_tokens.toLocaleString()} tokens (est.)</span>
                      <span>{r.word_count.toLocaleString()} words</span>
                      <span>{r.format}</span>
                    </div>
                    {r.warnings.length > 0 && (
                      <ul style={{ margin: "6px 0 0 0", paddingLeft: 18, fontSize: "var(--font-size-xs)", color: "var(--warning-color)" }}>
                        {r.warnings.map((w) => <li key={w}>{w}</li>)}
                      </ul>
                    )}
                  </div>
                ))}
              </div>
            )}

            {/* Files the walk could not read. "8 ingested" reads very
                differently next to "and 40 could not be read". */}
            {skipped.length > 0 && (
              <details style={{ marginTop: 4 }}>
                <summary style={{ fontSize: "var(--font-size-sm)", color: "var(--warning-color)", cursor: "pointer" }}>
                  {skipped.length} file(s) could not be ingested
                </summary>
                <ul style={{ margin: "6px 0 0 0", paddingLeft: 18, fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
                  {skipped.map((sk) => (
                    <li key={sk.path} style={{ marginBottom: 2 }}>
                      <span style={{ fontFamily: "var(--font-mono)" }}>{sk.path}</span> — {sk.reason}
                    </li>
                  ))}
                </ul>
              </details>
            )}

            {results.length === 0 && !isLoading && !error && (
              <div className="panel-empty-state">No documents ingested yet. Enter a path and click Ingest.</div>
            )}
          </div>
        )}

        {tab === "config" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600, marginBottom: 4 }}>Chunking Configuration</div>

            {/* Max tokens slider */}
            <div>
              <label htmlFor="doc-ingest-max-tokens" style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
                <span>Max Tokens per Chunk</span>
                <span style={{ fontFamily: "var(--font-mono)" }}>{config.maxTokens}</span>
              </label>
              <input
                id="doc-ingest-max-tokens"
                type="range"
                min={128}
                max={2048}
                step={64}
                value={config.maxTokens}
                onChange={(e) => setConfig((c) => ({ ...c, maxTokens: Number(e.target.value) }))}
                style={{ width: "100%" }}
              />
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
                <span>128</span><span>2048</span>
              </div>
            </div>

            {/* Overlap slider */}
            <div>
              <label htmlFor="doc-ingest-overlap" style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
                <span>Overlap (tokens)</span>
                <span style={{ fontFamily: "var(--font-mono)" }}>{config.overlap}</span>
              </label>
              <input
                id="doc-ingest-overlap"
                type="range"
                min={0}
                max={200}
                step={10}
                value={config.overlap}
                onChange={(e) => setConfig((c) => ({ ...c, overlap: Number(e.target.value) }))}
                style={{ width: "100%" }}
              />
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>
                <span>0</span><span>200</span>
              </div>
            </div>

            {/* Min chunk size */}
            <div>
              <label htmlFor="doc-ingest-min-chunk" style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>Min Chunk Size (tokens)</label>
              <input
                id="doc-ingest-min-chunk"
                type="number"
                min={1}
                max={512}
                value={config.minChunkSize}
                onChange={(e) => setConfig((c) => ({ ...c, minChunkSize: Number(e.target.value) }))}
                style={{ width: 120, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", padding: "8px 8px", fontSize: "var(--font-size-base)" }}
              />
            </div>

            {/* Sentence boundary toggle */}
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontSize: "var(--font-size-base)" }}>Respect Sentence Boundaries</div>
                <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>Avoid splitting mid-sentence when chunking</div>
              </div>
              <button
                onClick={() => setConfig((c) => ({ ...c, sentenceBoundary: !c.sentenceBoundary }))}
                role="switch"
                aria-checked={config.sentenceBoundary}
                aria-label="Respect Sentence Boundaries"
                style={{
                  width: 40,
                  height: 22,
                  borderRadius: 11,
                  border: "none",
                  background: config.sentenceBoundary ? "var(--accent-color)" : "var(--bg-secondary)",
                  cursor: "pointer",
                  position: "relative",
                }}
              >
                <div style={{
                  width: 16,
                  height: 16,
                  borderRadius: "50%",
                  background: "var(--bg-elevated)",
                  position: "absolute",
                  top: 3,
                  left: config.sentenceBoundary ? 21 : 3,
                  transition: "left 0.15s ease",
                }} />
              </button>
            </div>

            {/* Section title toggle */}
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontSize: "var(--font-size-base)" }}>Extract Section Titles</div>
                <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>Attach heading/title metadata to each chunk</div>
              </div>
              <button
                onClick={() => setConfig((c) => ({ ...c, sectionTitle: !c.sectionTitle }))}
                role="switch"
                aria-checked={config.sectionTitle}
                aria-label="Extract Section Titles"
                style={{
                  width: 40,
                  height: 22,
                  borderRadius: 11,
                  border: "none",
                  background: config.sectionTitle ? "var(--accent-color)" : "var(--bg-secondary)",
                  cursor: "pointer",
                  position: "relative",
                }}
              >
                <div style={{
                  width: 16,
                  height: 16,
                  borderRadius: "50%",
                  background: "var(--bg-elevated)",
                  position: "absolute",
                  top: 3,
                  left: config.sectionTitle ? 21 : 3,
                  transition: "left 0.15s ease",
                }} />
              </button>
            </div>

            {/* Summary */}
            <div style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: 12, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 8 }}>
              <div style={{ fontWeight: 600, marginBottom: 4, color: "var(--text-primary)" }}>Current Config Summary</div>
              <div>Chunk size: {config.maxTokens} tokens (min {config.minChunkSize})</div>
              <div>Overlap: {config.overlap} tokens</div>
              <div>Sentence boundary: {config.sentenceBoundary ? "enabled" : "disabled"}</div>
              <div>Section titles: {config.sectionTitle ? "enabled" : "disabled"}</div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
