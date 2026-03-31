import { useState, useRef, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface RecognizedComponent {
  shape: string;
  component: string;
  confidence: number;
  x: number;
  y: number;
}

interface DrawnShape {
  tool: string;
  x: number;
  y: number;
  w: number;
  h: number;
  color: string;
  text?: string;
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
  color: "var(--btn-primary-fg, #fff)",
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
  color: active ? "var(--btn-primary-fg, #fff)" : "var(--text-primary)",
  cursor: "pointer",
  fontSize: 12,
  fontWeight: active ? 600 : 400,
});

const COLORS = ["#4f8ff7", "#f44336", "#4caf50", "#ff9800", "#9c27b0", "#607d8b"];

export function SketchCanvasPanel() {
  const [tab, setTab] = useState("canvas");
  const [activeTool, setActiveTool] = useState("rect");
  const [activeColor, setActiveColor] = useState(COLORS[0]);
  const [framework, setFramework] = useState("react");
  const [recognized, setRecognized] = useState<RecognizedComponent[]>([]);
  const [generatedCode, setGeneratedCode] = useState<string>("");
  const [recognizing, setRecognizing] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [exporting, setExporting] = useState(false);

  const [shapes, setShapes] = useState<DrawnShape[]>([]);
  const [drawing, setDrawing] = useState(false);
  const [startPos, setStartPos] = useState<{ x: number; y: number } | null>(null);
  const [currentShape, setCurrentShape] = useState<DrawnShape | null>(null);

  const canvasRef = useRef<HTMLCanvasElement>(null);

  const tools = [
    { id: "rect", label: "Rect" },
    { id: "circle", label: "Circle" },
    { id: "line", label: "Line" },
    { id: "text", label: "Text" },
    { id: "arrow", label: "Arrow" },
  ];

  /* ── Canvas drawing ──────────────────────────────────────────────── */

  const drawShape = useCallback((ctx: CanvasRenderingContext2D, s: DrawnShape) => {
    ctx.strokeStyle = s.color;
    ctx.fillStyle = s.color + "22";
    ctx.lineWidth = 2;

    switch (s.tool) {
      case "rect":
        ctx.strokeRect(s.x, s.y, s.w, s.h);
        ctx.fillRect(s.x, s.y, s.w, s.h);
        break;
      case "circle": {
        const rx = Math.abs(s.w) / 2;
        const ry = Math.abs(s.h) / 2;
        const cx = s.x + s.w / 2;
        const cy = s.y + s.h / 2;
        ctx.beginPath();
        ctx.ellipse(cx, cy, rx, ry, 0, 0, Math.PI * 2);
        ctx.fill();
        ctx.stroke();
        break;
      }
      case "line":
        ctx.beginPath();
        ctx.moveTo(s.x, s.y);
        ctx.lineTo(s.x + s.w, s.y + s.h);
        ctx.stroke();
        break;
      case "arrow": {
        const ex = s.x + s.w;
        const ey = s.y + s.h;
        ctx.beginPath();
        ctx.moveTo(s.x, s.y);
        ctx.lineTo(ex, ey);
        ctx.stroke();
        // arrowhead
        const angle = Math.atan2(s.h, s.w);
        const headLen = 12;
        ctx.beginPath();
        ctx.moveTo(ex, ey);
        ctx.lineTo(ex - headLen * Math.cos(angle - 0.4), ey - headLen * Math.sin(angle - 0.4));
        ctx.moveTo(ex, ey);
        ctx.lineTo(ex - headLen * Math.cos(angle + 0.4), ey - headLen * Math.sin(angle + 0.4));
        ctx.stroke();
        break;
      }
      case "text":
        ctx.fillStyle = s.color;
        ctx.font = "14px var(--font-family, sans-serif)";
        ctx.fillText(s.text || "Text", s.x, s.y + 16);
        break;
    }
  }, []);

  const redraw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // grid
    ctx.strokeStyle = "rgba(128,128,128,0.08)";
    ctx.lineWidth = 1;
    for (let x = 0; x < canvas.width; x += 20) { ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, canvas.height); ctx.stroke(); }
    for (let y = 0; y < canvas.height; y += 20) { ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(canvas.width, y); ctx.stroke(); }

    shapes.forEach((s) => drawShape(ctx, s));
    if (currentShape) drawShape(ctx, currentShape);
  }, [shapes, currentShape, drawShape]);

  useEffect(() => { redraw(); }, [redraw]);

  const getCanvasPos = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const rect = canvasRef.current!.getBoundingClientRect();
    return { x: e.clientX - rect.left, y: e.clientY - rect.top };
  };

  const handleMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (activeTool === "text") {
      const pos = getCanvasPos(e);
      const text = prompt("Enter text:");
      if (text) {
        setShapes((prev) => [...prev, { tool: "text", x: pos.x, y: pos.y, w: 0, h: 0, color: activeColor, text }]);
      }
      return;
    }
    const pos = getCanvasPos(e);
    setDrawing(true);
    setStartPos(pos);
    setCurrentShape({ tool: activeTool, x: pos.x, y: pos.y, w: 0, h: 0, color: activeColor });
  };

  const handleMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!drawing || !startPos) return;
    const pos = getCanvasPos(e);
    setCurrentShape({
      tool: activeTool,
      x: startPos.x,
      y: startPos.y,
      w: pos.x - startPos.x,
      h: pos.y - startPos.y,
      color: activeColor,
    });
  };

  const handleMouseUp = () => {
    if (!drawing || !currentShape) { setDrawing(false); return; }
    if (Math.abs(currentShape.w) > 2 || Math.abs(currentShape.h) > 2) {
      setShapes((prev) => [...prev, currentShape]);
    }
    setDrawing(false);
    setStartPos(null);
    setCurrentShape(null);
  };

  const handleClear = () => {
    setShapes([]);
    setRecognized([]);
    setGeneratedCode("");
  };

  const handleUndo = () => {
    setShapes((prev) => prev.slice(0, -1));
  };

  /* ── Backend actions ─────────────────────────────────────────────── */

  const handleRecognize = async () => {
    setRecognizing(true);
    try {
      const elements = JSON.stringify(shapes.map((s) => ({
        type: s.tool, x: Math.round(s.x), y: Math.round(s.y),
        w: Math.round(Math.abs(s.w)), h: Math.round(Math.abs(s.h)),
        bounds: { x: Math.round(s.x), y: Math.round(s.y), w: Math.round(Math.abs(s.w)), h: Math.round(Math.abs(s.h)) },
      })));
      const res = await invoke<{ recognized: Array<{ id: string; type: string; confidence: number; bounds: { x?: number; y?: number } }> }>("sketch_recognize", { elements });
      const recognized = (res.recognized ?? []).map((r) => ({
        shape: r.type,
        component: r.type.replace(/\s+/g, ""),
        confidence: Math.round((r.confidence ?? 0.85) * 100),
        x: r.bounds?.x ?? 0,
        y: r.bounds?.y ?? 0,
      }));
      setRecognized(recognized);
    } catch (e) {
      console.error("Failed to recognize shapes:", e);
    }
    setRecognizing(false);
  };

  const handleGenerate = async () => {
    setGenerating(true);
    try {
      const components = JSON.stringify(recognized.map((r) => r.component));
      const res = await invoke<{ code: string }>("sketch_generate", { framework, components });
      setGeneratedCode(res.code ?? "");
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

  const confColor = (c: number) => c >= 85 ? "var(--success-color)" : c >= 70 ? "var(--warning-color)" : "var(--error-color)";

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

      {/* Canvas is always mounted to preserve drawing; hidden when other tabs active */}
      <div style={{ display: tab === "canvas" ? "block" : "none" }}>
        <div style={{ display: "flex", gap: 8, marginBottom: 12, alignItems: "center", flexWrap: "wrap" }}>
          {tools.map((t) => (
            <button key={t.id} style={toolBtnStyle(activeTool === t.id)} onClick={() => setActiveTool(t.id)}>
              {t.label}
            </button>
          ))}
          <span style={{ width: 1, height: 24, background: "var(--border-color)", margin: "0 4px" }} />
          {COLORS.map((c) => (
            <button
              key={c}
              onClick={() => setActiveColor(c)}
              style={{
                width: 24, height: 24, borderRadius: "50%", border: activeColor === c ? "2px solid var(--text-primary)" : "2px solid transparent",
                background: c, cursor: "pointer", padding: 0,
              }}
            />
          ))}
          <span style={{ width: 1, height: 24, background: "var(--border-color)", margin: "0 4px" }} />
          <button style={{ ...btnStyle, background: "transparent", color: "var(--text-primary)" }} onClick={handleUndo} disabled={shapes.length === 0}>
            Undo
          </button>
          <button style={{ ...btnStyle, background: "transparent", color: "var(--text-primary)" }} onClick={handleClear} disabled={shapes.length === 0}>
            Clear
          </button>
          <span style={{ fontSize: 11, color: "var(--text-secondary)", marginLeft: "auto" }}>
            {shapes.length} shape{shapes.length !== 1 ? "s" : ""}
          </span>
        </div>
        <canvas
          ref={canvasRef}
          width={800}
          height={400}
          onMouseDown={handleMouseDown}
          onMouseMove={handleMouseMove}
          onMouseUp={handleMouseUp}
          onMouseLeave={handleMouseUp}
          style={{
            width: "100%", height: 400, borderRadius: 8, border: "2px solid var(--border-color)",
            background: "var(--bg-secondary)", cursor: activeTool === "text" ? "text" : "crosshair",
          }}
        />
      </div>

      {tab === "recognize" && (
        <div>
          <div style={{ marginBottom: 12 }}>
            <button style={btnStyle} onClick={handleRecognize} disabled={recognizing || shapes.length === 0}>
              {recognizing ? "Recognizing..." : `Recognize (${shapes.length} shapes)`}
            </button>
          </div>
          {recognizing && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Analyzing shapes...</div>}
          {!recognizing && shapes.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Draw shapes on the canvas first.</div>}
          {!recognizing && shapes.length > 0 && recognized.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Click Recognize to analyze your sketch.</div>}
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
                style={{ ...btnStyle, background: framework === f ? "var(--accent-color)" : "transparent", color: framework === f ? "var(--btn-primary-fg, #fff)" : "var(--text-primary)", border: "1px solid var(--border-color)" }}>
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
            {shapes.length} shapes drawn | {recognized.length} recognized | Framework: {framework}
          </div>
        </div>
      )}
    </div>
  );
}
