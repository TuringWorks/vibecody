/**
 * DocumentIngestPanel — file/directory ingestion with chunking configuration.
 *
 * Tabs: Ingest (file path + format + actions), Config (chunking parameters)
 */
import { useState } from "react";

type Tab = "ingest" | "config";
type Format = "auto" | "plain" | "markdown" | "html" | "pdf" | "docx" | "csv" | "json" | "code";

interface IngestResult {
  id: string;
  title: string;
  chunks: number;
  tokens: number;
  format: string;
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
  const [isLoading, setIsLoading] = useState(false);
  const [config, setConfig] = useState<ChunkingConfig>({
    maxTokens: 512,
    overlap: 50,
    minChunkSize: 64,
    sentenceBoundary: true,
    sectionTitle: true,
  });

  const handleIngestFile = () => {
    if (!filePath.trim()) return;
    setIsLoading(true);
    // Simulate ingestion
    setTimeout(() => {
      const result: IngestResult = {
        id: crypto.randomUUID().slice(0, 8),
        title: filePath.split("/").pop() || filePath,
        chunks: Math.floor(Math.random() * 40) + 5,
        tokens: Math.floor(Math.random() * 8000) + 500,
        format: format === "auto" ? (filePath.split(".").pop() || "plain") : format,
      };
      setResults((prev) => [result, ...prev]);
      setIsLoading(false);
    }, 600);
  };

  const handleIngestDirectory = () => {
    if (!filePath.trim()) return;
    setIsLoading(true);
    setTimeout(() => {
      const count = Math.floor(Math.random() * 8) + 2;
      const newResults: IngestResult[] = Array.from({ length: count }, (_, i) => ({
        id: crypto.randomUUID().slice(0, 8),
        title: `${filePath.split("/").pop()}/file_${i + 1}`,
        chunks: Math.floor(Math.random() * 30) + 3,
        tokens: Math.floor(Math.random() * 5000) + 300,
        format: format === "auto" ? "mixed" : format,
      }));
      setResults((prev) => [...newResults, ...prev]);
      setIsLoading(false);
    }, 900);
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
                style={{ width: "100%", background: "var(--bg-secondary)", border: "1px solid var(--border)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", padding: "6px 8px", fontSize: "var(--font-size-base)", boxSizing: "border-box" }}
              />
            </div>

            {/* Format dropdown */}
            <div>
              <label style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>Format</label>
              <select
                value={format}
                onChange={(e) => setFormat(e.target.value as Format)}
                style={{ width: "100%", background: "var(--bg-secondary)", border: "1px solid var(--border)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", padding: "6px 8px", fontSize: "var(--font-size-base)", boxSizing: "border-box" }}
              >
                <option value="auto">Auto-detect</option>
                <option value="plain">Plain Text</option>
                <option value="markdown">Markdown</option>
                <option value="html">HTML</option>
                <option value="pdf">PDF</option>
                <option value="docx">DOCX</option>
                <option value="csv">CSV</option>
                <option value="json">JSON</option>
                <option value="code">Source Code</option>
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

            {/* Results */}
            {results.length > 0 && (
              <div style={{ marginTop: 8 }}>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 8 }}>{results.length} document(s) ingested</div>
                {results.map((r) => (
                  <div
                    key={r.id}
                    className="panel-card"
                    style={{ marginBottom: 6 }}
                  >
                    <div style={{ fontWeight: 600, fontSize: "var(--font-size-base)", marginBottom: 4 }}>{r.title}</div>
                    <div style={{ display: "flex", gap: 16, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
                      <span>{r.chunks} chunks</span>
                      <span>{r.tokens.toLocaleString()} tokens</span>
                      <span>{r.format}</span>
                      <span style={{ opacity: 0.5 }}>id: {r.id}</span>
                    </div>
                  </div>
                ))}
              </div>
            )}
            {results.length === 0 && !isLoading && (
              <div className="panel-empty-state">No documents ingested yet. Enter a path and click Ingest.</div>
            )}
          </div>
        )}

        {tab === "config" && (
          <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600, marginBottom: 4 }}>Chunking Configuration</div>

            {/* Max tokens slider */}
            <div>
              <label style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
                <span>Max Tokens per Chunk</span>
                <span style={{ fontFamily: "var(--font-mono)" }}>{config.maxTokens}</span>
              </label>
              <input
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
              <label style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
                <span>Overlap (tokens)</span>
                <span style={{ fontFamily: "var(--font-mono)" }}>{config.overlap}</span>
              </label>
              <input
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
              <label style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", display: "block", marginBottom: 4 }}>Min Chunk Size (tokens)</label>
              <input
                type="number"
                min={1}
                max={512}
                value={config.minChunkSize}
                onChange={(e) => setConfig((c) => ({ ...c, minChunkSize: Number(e.target.value) }))}
                style={{ width: 120, background: "var(--bg-secondary)", border: "1px solid var(--border)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", padding: "6px 8px", fontSize: "var(--font-size-base)" }}
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
                  background: config.sentenceBoundary ? "var(--accent)" : "var(--bg-secondary)",
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
                  background: config.sectionTitle ? "var(--accent)" : "var(--bg-secondary)",
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
            <div style={{ background: "var(--bg-secondary)", border: "1px solid var(--border)", borderRadius: "var(--radius-xs-plus)", padding: 12, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 8 }}>
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
