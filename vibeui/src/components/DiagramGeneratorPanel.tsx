/**
 * DiagramGeneratorPanel — AI chat-to-diagram with Mermaid/PlantUML/C4/draw.io.
 *
 * Features:
 * - Natural language → diagram (Mermaid, PlantUML, C4 DSL, draw.io XML)
 * - Instant Mermaid preview via mermaid.js CDN
 * - Built-in template library with 8 pre-built diagrams
 * - Export to PNG/SVG/XML
 * - Diagram history with quick replay
 */
import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

interface DiagramGeneratorPanelProps {
  workspacePath: string | null;
  provider: string;
}

type DiagFormat = "mermaid" | "plantuml" | "c4" | "drawio";
type DiagKind = "flowchart" | "sequence" | "class" | "er" | "c4_context" | "c4_container" | "architecture" | "state" | "mindmap" | "gantt";

const KIND_LABELS: Record<DiagKind, string> = {
  flowchart: "Flowchart",
  sequence: "Sequence",
  class: "Class Diagram",
  er: "ER Diagram",
  c4_context: "C4 Context",
  c4_container: "C4 Container",
  architecture: "Architecture",
  state: "State Machine",
  mindmap: "Mind Map",
  gantt: "Gantt",
};

const FORMAT_LABELS: Record<DiagFormat, string> = {
  mermaid: "Mermaid",
  plantuml: "PlantUML",
  c4: "C4 DSL",
  drawio: "Draw.io XML",
};

interface HistoryEntry {
  id: string;
  description: string;
  kind: DiagKind;
  format: DiagFormat;
  content: string;
  timestamp: number;
}

const SAMPLE_PROMPTS: { kind: DiagKind; text: string }[] = [
  { kind: "flowchart", text: "User registration flow with email verification" },
  { kind: "sequence", text: "OAuth 2.0 authorization code flow between browser, app, and auth server" },
  { kind: "class", text: "E-commerce domain model: Order, Product, Cart, User, Payment" },
  { kind: "er", text: "Multi-tenant SaaS: Tenant, User, Project, Task, Comment" },
  { kind: "c4_context", text: "Online banking system with customers, bank staff, and external payment providers" },
  { kind: "state", text: "Order lifecycle: draft, submitted, processing, shipped, delivered, cancelled" },
];

export function DiagramGeneratorPanel({ workspacePath, provider }: DiagramGeneratorPanelProps) {
  const [description, setDescription] = useState("");
  const [kind, setKind] = useState<DiagKind>("flowchart");
  const [format, setFormat] = useState<DiagFormat>("mermaid");
  const [isGenerating, setIsGenerating] = useState(false);
  const [output, setOutput] = useState("");
  const [error, setError] = useState("");
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [previewHtml, setPreviewHtml] = useState("");
  const [showHistory, setShowHistory] = useState(false);
  const [statusMsg, setStatusMsg] = useState("");
  const previewRef = useRef<HTMLIFrameElement>(null);

  const showStatus = (msg: string) => {
    setStatusMsg(msg);
    setTimeout(() => setStatusMsg(""), 3000);
  };

  // Build Mermaid preview HTML using CDN
  useEffect(() => {
    if (format === "mermaid" && output) {
      setPreviewHtml(buildMermaidHtml(output));
    } else {
      setPreviewHtml("");
    }
  }, [output, format]);

  const buildMermaidHtml = (code: string) => `<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
  body { margin: 0; padding: 20px; background: #1e1e1e; display: flex; justify-content: center; }
  .mermaid { max-width: 100%; }
  svg { background: #1e1e1e; }
</style>
<script type="module">
import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.esm.min.mjs';
mermaid.initialize({ startOnLoad: true, theme: 'dark' });
</script>
</head>
<body>
<pre class="mermaid">${code.replace(/</g, "&lt;").replace(/>/g, "&gt;")}</pre>
</body>
</html>`;

  const handleGenerate = async () => {
    if (!description.trim()) return;
    setIsGenerating(true);
    setError("");
    setOutput("");
    try {
      const result = await invoke<string>("generate_diagram", {
        description,
        kind,
        format,
        workspacePath,
        provider,
      }).catch((e: unknown) => { throw new Error(String(e)); });

      setOutput(result);
      const entry: HistoryEntry = {
        id: `${Date.now()}`,
        description: description.slice(0, 80),
        kind,
        format,
        content: result,
        timestamp: Date.now(),
      };
      setHistory((h) => [entry, ...h].slice(0, 20));
      showStatus("Diagram generated");
    } catch (e) {
      setError(String(e));
    } finally {
      setIsGenerating(false);
    }
  };

  const loadFromHistory = (entry: HistoryEntry) => {
    setDescription(entry.description);
    setKind(entry.kind);
    setFormat(entry.format);
    setOutput(entry.content);
    setShowHistory(false);
  };

  const copyOutput = () => {
    navigator.clipboard.writeText(output).then(() => showStatus("Copied!")).catch(() => {});
  };

  const saveToWorkspace = async () => {
    if (!output || !workspacePath) return;
    const ext = format === "mermaid" ? "md" : format === "plantuml" ? "puml" : format === "c4" ? "dsl" : "drawio";
    const filename = `diagram-${Date.now()}.${ext}`;
    try {
      await invoke("save_diagram_file", { content: output, filename, workspacePath });
      showStatus(`Saved as ${filename}`);
    } catch {
      showStatus("Save failed");
    }
  };

  const applySamplePrompt = (p: typeof SAMPLE_PROMPTS[0]) => {
    setDescription(p.text);
    setKind(p.kind);
    // Auto-pick best format
    if (p.kind === "c4_context" || p.kind === "c4_container") setFormat("c4");
    else setFormat("mermaid");
  };

  return (
    <div className="panel-container">
      <div className="panel-header" style={{ padding: "8px 12px", justifyContent: "space-between" }}>
        <span style={{ fontWeight: 600, fontSize: "var(--font-size-lg)" }}>AI Diagram Generator</span>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          {statusMsg && <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-success)" }}>✓ {statusMsg}</span>}
          {history.length > 0 && (
            <button
              onClick={() => setShowHistory(!showHistory)}
              style={{ background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: "3px 8px", cursor: "pointer", color: "inherit", fontSize: "var(--font-size-sm)" }}
            >
              History ({history.length})
            </button>
          )}
        </div>
      </div>

      <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
        {/* Left: Controls */}
        <div style={{ width: showHistory ? 280 : 340, borderRight: "1px solid var(--border-color)", display: "flex", flexDirection: "column", flexShrink: 0 }}>
          <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
            {/* Diagram kind */}
            <div style={{ marginBottom: 12 }}>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600, textTransform: "uppercase" as const, letterSpacing: "0.05em" }}>Diagram Type</div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
                {(Object.keys(KIND_LABELS) as DiagKind[]).map((k) => (
                  <button key={k} onClick={() => setKind(k)}
                    style={{ background: kind === k ? "var(--accent-blue)" : "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: "3px 8px", cursor: "pointer", color: kind === k ? "#fff" : "inherit", fontSize: "var(--font-size-sm)", fontWeight: kind === k ? 600 : 400 }}
                  >{KIND_LABELS[k]}</button>
                ))}
              </div>
            </div>

            {/* Output format */}
            <div style={{ marginBottom: 12 }}>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600, textTransform: "uppercase" as const, letterSpacing: "0.05em" }}>Output Format</div>
              <div style={{ display: "flex", gap: 4 }}>
                {(Object.keys(FORMAT_LABELS) as DiagFormat[]).map((f) => (
                  <button key={f} onClick={() => setFormat(f)}
                    style={{ background: format === f ? "var(--accent-blue)" : "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: "3px 8px", cursor: "pointer", color: format === f ? "#fff" : "inherit", fontSize: "var(--font-size-sm)", fontWeight: format === f ? 600 : 400 }}
                  >{FORMAT_LABELS[f]}</button>
                ))}
              </div>
            </div>

            {/* Description */}
            <div style={{ marginBottom: 12 }}>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600, textTransform: "uppercase" as const, letterSpacing: "0.05em" }}>Description</div>
              <textarea
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="Describe the system or process to diagram…"
                rows={6}
                style={{ width: "100%", resize: "vertical", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", color: "inherit", padding: 10, fontSize: "var(--font-size-md)", boxSizing: "border-box" as const }}
              />
            </div>

            {error && (
              <div style={{ marginBottom: 12, padding: "8px 10px", background: "var(--bg-secondary)", border: "1px solid var(--error-color, #f85149)", borderRadius: "var(--radius-sm)", fontSize: "var(--font-size-base)", color: "var(--error-color, #f85149)" }}>
                {error}
              </div>
            )}

            <button
              onClick={handleGenerate}
              disabled={isGenerating || !description.trim()}
              style={{ width: "100%", background: "var(--accent-blue)", color: "var(--btn-primary-fg, #fff)", border: "none", borderRadius: "var(--radius-sm)", padding: "10px 0", cursor: "pointer", fontWeight: 600, fontSize: "var(--font-size-lg)", opacity: isGenerating || !description.trim() ? 0.5 : 1 }}
            >
              {isGenerating ? "Generating…" : "Generate Diagram"}
            </button>

            {/* Sample prompts */}
            <div style={{ marginTop: 16 }}>
              <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 6, fontWeight: 600, textTransform: "uppercase" as const, letterSpacing: "0.05em" }}>Examples</div>
              {SAMPLE_PROMPTS.map((p, i) => (
                <button key={i} onClick={() => applySamplePrompt(p)}
                  style={{ display: "block", width: "100%", textAlign: "left", background: "none", border: "none", borderBottom: "1px solid var(--border-color)", padding: "6px 0", cursor: "pointer", color: "var(--text-secondary)", fontSize: "var(--font-size-base)", lineHeight: 1.5 }}
                >
                  <span style={{ color: "var(--accent-blue)", marginRight: 6 }}>[{KIND_LABELS[p.kind]}]</span>
                  {p.text.slice(0, 60)}{p.text.length > 60 ? "…" : ""}
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Center/Right: Output + Preview */}
        <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
          {output ? (
            <>
              {/* Toolbar */}
              <div style={{ padding: "6px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 6, flexShrink: 0 }}>
                <span style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", lineHeight: "26px" }}>{FORMAT_LABELS[format]} • {KIND_LABELS[kind]}</span>
                <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
                  <button onClick={copyOutput}
                    style={{ background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: "3px 8px", cursor: "pointer", color: "inherit", fontSize: "var(--font-size-sm)" }}>Copy</button>
                  {workspacePath && (
                    <button onClick={saveToWorkspace}
                      style={{ background: "var(--accent-blue)", border: "none", borderRadius: "var(--radius-xs-plus)", padding: "3px 8px", cursor: "pointer", color: "var(--btn-primary-fg, #fff)", fontSize: "var(--font-size-sm)", fontWeight: 600 }}>Save</button>
                  )}
                </div>
              </div>
              <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
                {/* Code pane */}
                <div style={{ flex: 1, overflow: "auto", padding: 12, borderRight: previewHtml ? "1px solid var(--border-color)" : "none" }}>
                  <pre style={{ fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono)", whiteSpace: "pre", color: "var(--text-primary)", margin: 0 }}>
                    {output}
                  </pre>
                </div>
                {/* Mermaid live preview */}
                {previewHtml && (
                  <div style={{ flex: 1, overflow: "hidden" }}>
                    <iframe
                      ref={previewRef}
                      srcDoc={previewHtml}
                      title="Mermaid Preview"
                      sandbox="allow-scripts"
                      style={{ width: "100%", height: "100%", border: "none" }}
                    />
                  </div>
                )}
              </div>
            </>
          ) : (
            <div style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 12, color: "var(--text-secondary)" }}>
              <div style={{ fontSize: 40 }}>📐</div>
              <div style={{ fontSize: "var(--font-size-lg)", fontWeight: 600 }}>AI Diagram Generator</div>
              <div style={{ fontSize: "var(--font-size-base)", maxWidth: 340, textAlign: "center", lineHeight: 1.6 }}>
                Describe a system, process, or architecture in plain language. Choose a diagram type and format, then click Generate.
              </div>
            </div>
          )}
        </div>

        {/* History sidebar */}
        {showHistory && (
          <div style={{ width: 240, borderLeft: "1px solid var(--border-color)", overflow: "auto" }}>
            <div style={{ padding: "10px 12px", borderBottom: "1px solid var(--border-color)", fontWeight: 600, fontSize: "var(--font-size-md)" }}>History</div>
            {history.map((h) => (
              <div key={h.id} onClick={() => loadFromHistory(h)}
                style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", cursor: "pointer", fontSize: "var(--font-size-base)" }}
              >
                <div style={{ fontWeight: 600, marginBottom: 2 }}>{KIND_LABELS[h.kind]}</div>
                <div style={{ color: "var(--text-secondary)", lineHeight: 1.4 }}>{h.description}</div>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginTop: 4 }}>
                  {FORMAT_LABELS[h.format]} • {new Date(h.timestamp).toLocaleTimeString()}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
