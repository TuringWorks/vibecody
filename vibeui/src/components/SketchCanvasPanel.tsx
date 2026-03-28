import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface RecognizedComponent {
  shape: string;
  component: string;
  confidence: number;
  x: number;
  y: number;
}

const panelStyle: React.CSSProperties = {
  padding: 16,
  height: "100%",
  overflow: "auto",
  color: "var(--text-primary)",
  background: "var(--bg-primary)",
};

const headingStyle: React.CSSProperties = {
  fontSize: 18,
  fontWeight: 600,
  marginBottom: 12,
  color: "var(--text-primary)",
};

const cardStyle: React.CSSProperties = {
  background: "var(--bg-secondary)",
  borderRadius: 8,
  padding: 12,
  marginBottom: 8,
  border: "1px solid var(--border-color)",
};

const btnStyle: React.CSSProperties = {
  padding: "6px 14px",
  borderRadius: 6,
  border: "1px solid var(--border-color)",
  background: "var(--accent-color)",
  color: "#fff",
  cursor: "pointer",
  fontSize: 13,
  marginRight: 8,
};

const tabStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 16px",
  cursor: "pointer",
  borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
  color: active ? "var(--accent-color)" : "var(--text-secondary)",
  background: "transparent",
  border: "none",
  fontSize: 13,
  fontWeight: active ? 600 : 400,
});

const toolBtnStyle = (active: boolean): React.CSSProperties => ({
  padding: "8px 12px",
  borderRadius: 6,
  border: active ? "2px solid var(--accent-color)" : "1px solid var(--border-color)",
  background: active ? "var(--accent-color)" : "var(--bg-secondary)",
  color: active ? "#fff" : "var(--text-primary)",
  cursor: "pointer",
  fontSize: 12,
  fontWeight: active ? 600 : 400,
});

export function SketchCanvasPanel() {
  const [tab, setTab] = useState("canvas");
  const [activeTool, setActiveTool] = useState("rect");
  const [framework, setFramework] = useState("react");
  const [recognized, setRecognized] = useState<RecognizedComponent[]>([]);
  const [generatedCode, setGeneratedCode] = useState<string>("");
  const [recognizing, setRecognizing] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [exporting, setExporting] = useState(false);

  const tools = [
    { id: "rect", label: "Rect" },
    { id: "circle", label: "Circle" },
    { id: "line", label: "Line" },
    { id: "text", label: "Text" },
    { id: "arrow", label: "Arrow" },
  ];

  const handleRecognize = async () => {
    setRecognizing(true);
    try {
      const result = await invoke<RecognizedComponent[]>("sketch_recognize", { elements: [] });
      setRecognized(result);
    } catch (e) {
      console.error("Failed to recognize shapes:", e);
    }
    setRecognizing(false);
  };

  const handleGenerate = async () => {
    setGenerating(true);
    try {
      const components = recognized.map((r) => r.component);
      const result = await invoke<string>("sketch_generate", { framework, components });
      setGeneratedCode(result);
    } catch (e) {
      console.error("Failed to generate code:", e);
    }
    setGenerating(false);
  };

  const handleExport = async (format: string) => {
    setExporting(true);
    try {
      await invoke("sketch_export", { format });
    } catch (e) {
      console.error("Failed to export:", e);
    }
    setExporting(false);
  };

  const confColor = (c: number) => c >= 85 ? "#22c55e" : c >= 70 ? "#eab308" : "#ef4444";

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Sketch Canvas</h2>
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--border-color)", marginBottom: 16 }}>
        {["canvas", "recognize", "code", "export"].map((t) => (
          <button key={t} style={tabStyle(tab === t)} onClick={() => setTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {tab === "canvas" && (
        <div>
          <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
            {tools.map((t) => (
              <button key={t.id} style={toolBtnStyle(activeTool === t.id)} onClick={() => setActiveTool(t.id)}>
                {t.label}
              </button>
            ))}
          </div>
          <div style={{
            width: "100%", height: 360, borderRadius: 8, border: "2px dashed var(--border-color)",
            background: "var(--bg-secondary)", display: "flex", alignItems: "center", justifyContent: "center",
            color: "var(--text-secondary)", fontSize: 14, cursor: "crosshair",
          }}>
            Draw with {activeTool} tool - click and drag to create shapes
          </div>
        </div>
      )}

      {tab === "recognize" && (
        <div>
          <div style={{ marginBottom: 12 }}>
            <button style={btnStyle} onClick={handleRecognize} disabled={recognizing}>
              {recognizing ? "Recognizing..." : "Recognize"}
            </button>
          </div>
          {recognizing && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Analyzing shapes...</div>}
          {!recognizing && recognized.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>No shapes recognized yet. Draw on the canvas and click Recognize.</div>}
          {recognized.map((r, i) => (
            <div key={i} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <span style={{ fontWeight: 600, fontSize: 13 }}>{r.shape}</span>
                <span style={{ fontSize: 12, color: "var(--text-secondary)", marginLeft: 8 }}>at ({r.x}, {r.y})</span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ fontSize: 12, color: "var(--accent-color)", fontWeight: 600 }}>{r.component}</span>
                <span style={{ fontSize: 11, fontWeight: 600, color: confColor(r.confidence) }}>{r.confidence}%</span>
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "code" && (
        <div>
          <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
            {["react", "html", "swiftui"].map((f) => (
              <button key={f} onClick={() => setFramework(f)}
                style={{ ...btnStyle, background: framework === f ? "var(--accent-color)" : "transparent", color: framework === f ? "#fff" : "var(--text-primary)", border: "1px solid var(--border-color)" }}>
                {f === "swiftui" ? "SwiftUI" : f === "html" ? "HTML" : "React"}
              </button>
            ))}
            <button style={btnStyle} onClick={handleGenerate} disabled={generating}>
              {generating ? "Generating..." : "Generate Code"}
            </button>
          </div>
          {generating && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Generating code...</div>}
          {!generating && !generatedCode && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Recognize shapes first, then click Generate Code.</div>}
          {generatedCode && (
            <pre style={{
              ...cardStyle, fontFamily: "monospace", fontSize: 12, whiteSpace: "pre-wrap",
              lineHeight: 1.5, maxHeight: 400, overflow: "auto",
            }}>
              {generatedCode}
            </pre>
          )}
        </div>
      )}

      {tab === "export" && (
        <div>
          <div style={cardStyle}>
            <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Export Canvas</div>
            <div style={{ display: "flex", gap: 8 }}>
              <button style={btnStyle} onClick={() => handleExport("svg")} disabled={exporting}>
                {exporting ? "Exporting..." : "Export SVG"}
              </button>
              <button style={btnStyle} onClick={() => handleExport("png")} disabled={exporting}>
                {exporting ? "Exporting..." : "Export PNG"}
              </button>
            </div>
          </div>
          <div style={{ ...cardStyle, fontSize: 13, color: "var(--text-secondary)" }}>
            {recognized.length} shapes recognized | Framework: {framework}
          </div>
        </div>
      )}
    </div>
  );
}
