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
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [moveOffset, setMoveOffset] = useState<{ dx: number; dy: number } | null>(null);
  const [textInput, setTextInput] = useState<{ x: number; y: number; canvasX: number; canvasY: number } | null>(null);
  const [textValue, setTextValue] = useState("");

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const canvasWrapRef = useRef<HTMLDivElement>(null);
  const textInputRef = useRef<HTMLInputElement>(null);

  const tools = [
    { id: "move", label: "Move" },
    { id: "rect", label: "Rect" },
    { id: "circle", label: "Circle" },
    { id: "line", label: "Line" },
    { id: "text", label: "Text" },
    { id: "arrow", label: "Arrow" },
  ];

  /* ── Canvas drawing ──────────────────────────────────────────────── */

  const drawShape = useCallback((ctx: CanvasRenderingContext2D, s: DrawnShape, selected?: boolean) => {
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

    // Selection outline
    if (selected) {
      ctx.save();
      ctx.strokeStyle = "#fff";
      ctx.lineWidth = 1;
      ctx.setLineDash([4, 3]);
      const bounds = getShapeBounds(s);
      ctx.strokeRect(bounds.x - 4, bounds.y - 4, bounds.w + 8, bounds.h + 8);
      ctx.restore();
    }
  }, []);

  /** Get the bounding box for any shape */
  const getShapeBounds = (s: DrawnShape) => {
    if (s.tool === "text") {
      return { x: s.x, y: s.y, w: (s.text?.length ?? 4) * 8, h: 20 };
    }
    const x = Math.min(s.x, s.x + s.w);
    const y = Math.min(s.y, s.y + s.h);
    const w = Math.abs(s.w);
    const h = Math.abs(s.h);
    return { x, y, w, h };
  };

  /** Find the topmost shape at a given point (last drawn = on top) */
  const hitTest = useCallback((px: number, py: number): number | null => {
    for (let i = shapes.length - 1; i >= 0; i--) {
      const b = getShapeBounds(shapes[i]);
      if (px >= b.x - 4 && px <= b.x + b.w + 4 && py >= b.y - 4 && py <= b.y + b.h + 4) {
        return i;
      }
    }
    return null;
  }, [shapes]);

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

    shapes.forEach((s, i) => drawShape(ctx, s, i === selectedIndex));
    if (currentShape) drawShape(ctx, currentShape);
  }, [shapes, currentShape, drawShape, selectedIndex]);

  useEffect(() => { redraw(); }, [redraw]);

  const getCanvasPos = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current!;
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    return { x: (e.clientX - rect.left) * scaleX, y: (e.clientY - rect.top) * scaleY };
  };

  /** Convert canvas coords to CSS pixel position for the text overlay */
  const canvasToCssPos = (cx: number, cy: number) => {
    const canvas = canvasRef.current!;
    const rect = canvas.getBoundingClientRect();
    return { x: (cx / canvas.width) * rect.width, y: (cy / canvas.height) * rect.height };
  };

  const commitTextInput = () => {
    if (textInput && textValue.trim()) {
      setShapes((prev) => [...prev, { tool: "text", x: textInput.canvasX, y: textInput.canvasY, w: 0, h: 0, color: activeColor, text: textValue }]);
    }
    setTextInput(null);
    setTextValue("");
  };

  const handleMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    // Commit any pending text input
    if (textInput) { commitTextInput(); return; }

    const pos = getCanvasPos(e);

    if (activeTool === "move") {
      const idx = hitTest(pos.x, pos.y);
      setSelectedIndex(idx);
      if (idx !== null) {
        const s = shapes[idx];
        setMoveOffset({ dx: pos.x - s.x, dy: pos.y - s.y });
        setDrawing(true);
      }
      return;
    }

    if (activeTool === "text") {
      const cssPos = canvasToCssPos(pos.x, pos.y);
      setTextInput({ x: cssPos.x, y: cssPos.y, canvasX: pos.x, canvasY: pos.y });
      setTextValue("");
      setTimeout(() => textInputRef.current?.focus(), 0);
      return;
    }

    setSelectedIndex(null);
    setDrawing(true);
    setStartPos(pos);
    setCurrentShape({ tool: activeTool, x: pos.x, y: pos.y, w: 0, h: 0, color: activeColor });
  };

  const handleMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!drawing) return;
    const pos = getCanvasPos(e);

    if (activeTool === "move" && selectedIndex !== null && moveOffset) {
      setShapes((prev) => prev.map((s, i) =>
        i === selectedIndex ? { ...s, x: pos.x - moveOffset.dx, y: pos.y - moveOffset.dy } : s
      ));
      return;
    }

    if (!startPos) return;
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
    if (activeTool === "move") {
      setDrawing(false);
      setMoveOffset(null);
      return;
    }
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
      const components = JSON.stringify(shapes.map((s) => ({
        type: s.tool,
        x: Math.round(s.x),
        y: Math.round(s.y),
        w: Math.round(s.w),
        h: Math.round(s.h),
        color: s.color,
        text: s.text,
      })));
      const res = await invoke<{ code: string }>("sketch_generate", { framework, components });
      setGeneratedCode(res.code ?? "");
    } catch (e) {
      console.error("Failed to generate code:", e);
    }
    setGenerating(false);
  };

  const buildSvgString = () => {
    const lines: string[] = ['<svg viewBox="0 0 800 400" xmlns="http://www.w3.org/2000/svg" style="background:#1e1e2e">'];
    for (const s of shapes) {
      const fill = s.color + "22";
      switch (s.tool) {
        case "rect":
          lines.push(`  <rect x="${s.x}" y="${s.y}" width="${Math.abs(s.w)}" height="${Math.abs(s.h)}" fill="${fill}" stroke="${s.color}" stroke-width="2" rx="2" />`);
          break;
        case "circle": {
          const rx = Math.abs(s.w) / 2, ry = Math.abs(s.h) / 2;
          lines.push(`  <ellipse cx="${s.x + s.w / 2}" cy="${s.y + s.h / 2}" rx="${rx}" ry="${ry}" fill="${fill}" stroke="${s.color}" stroke-width="2" />`);
          break;
        }
        case "line":
          lines.push(`  <line x1="${s.x}" y1="${s.y}" x2="${s.x + s.w}" y2="${s.y + s.h}" stroke="${s.color}" stroke-width="2" />`);
          break;
        case "arrow": {
          const ex = s.x + s.w, ey = s.y + s.h;
          const angle = Math.atan2(s.h, s.w), hl = 12;
          const ax1 = ex - hl * Math.cos(angle - 0.4), ay1 = ey - hl * Math.sin(angle - 0.4);
          const ax2 = ex - hl * Math.cos(angle + 0.4), ay2 = ey - hl * Math.sin(angle + 0.4);
          lines.push(`  <line x1="${s.x}" y1="${s.y}" x2="${ex}" y2="${ey}" stroke="${s.color}" stroke-width="2" />`);
          lines.push(`  <polygon points="${ex},${ey} ${ax1},${ay1} ${ax2},${ay2}" fill="${s.color}" />`);
          break;
        }
        case "text":
          lines.push(`  <text x="${s.x}" y="${s.y + 16}" fill="${s.color}" font-size="14" font-family="sans-serif">${s.text ?? "Text"}</text>`);
          break;
      }
    }
    lines.push("</svg>");
    return lines.join("\n");
  };

  const downloadFile = (content: string, filename: string, mime: string) => {
    const blob = new Blob([content], { type: mime });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  const handleExport = async (format: string) => {
    if (shapes.length === 0) return;
    setExporting(true);
    try {
      if (format === "svg") {
        downloadFile(buildSvgString(), "sketch.svg", "image/svg+xml");
      } else if (format === "png") {
        // Render SVG to a temporary canvas for PNG export
        const svgStr = buildSvgString();
        const img = new Image();
        const svgBlob = new Blob([svgStr], { type: "image/svg+xml;charset=utf-8" });
        const url = URL.createObjectURL(svgBlob);
        img.onload = () => {
          const offscreen = document.createElement("canvas");
          offscreen.width = 800;
          offscreen.height = 400;
          const ctx = offscreen.getContext("2d")!;
          ctx.drawImage(img, 0, 0);
          URL.revokeObjectURL(url);
          offscreen.toBlob((blob) => {
            if (!blob) return;
            const pngUrl = URL.createObjectURL(blob);
            const a = document.createElement("a");
            a.href = pngUrl;
            a.download = "sketch.png";
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            URL.revokeObjectURL(pngUrl);
          }, "image/png");
        };
        img.src = url;
      }
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
        <div ref={canvasWrapRef} style={{ position: "relative" }}>
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
              background: "var(--bg-secondary)",
              cursor: activeTool === "move" ? (drawing ? "grabbing" : "grab") : activeTool === "text" ? "text" : "crosshair",
            }}
          />
          {textInput && (
            <input
              ref={textInputRef}
              type="text"
              value={textValue}
              onChange={(e) => setTextValue(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") commitTextInput(); if (e.key === "Escape") { setTextInput(null); setTextValue(""); } }}
              onBlur={commitTextInput}
              style={{
                position: "absolute",
                left: textInput.x,
                top: textInput.y,
                background: "var(--bg-primary)",
                color: activeColor,
                border: `1px solid ${activeColor}`,
                borderRadius: 3,
                padding: "2px 6px",
                fontSize: 14,
                fontFamily: "var(--font-family, sans-serif)",
                outline: "none",
                minWidth: 80,
                zIndex: 10,
              }}
              placeholder="Type text..."
            />
          )}
        </div>
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
            <button style={btnStyle} onClick={handleGenerate} disabled={generating || shapes.length === 0}>
              {generating ? "Generating..." : `Generate Code (${shapes.length} shapes)`}
            </button>
          </div>
          {generating && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Generating code...</div>}
          {!generating && !generatedCode && shapes.length === 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Draw shapes on the canvas first.</div>}
          {!generating && !generatedCode && shapes.length > 0 && <div style={{ color: "var(--text-secondary)", fontSize: 13 }}>Click Generate Code to create SVG-based code from your sketch.</div>}
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
            {shapes.length === 0 && <div style={{ fontSize: 13, color: "var(--text-secondary)", marginBottom: 8 }}>Draw shapes on the canvas first.</div>}
            <div style={{ display: "flex", gap: 8 }}>
              <button style={btnStyle} onClick={() => handleExport("svg")} disabled={exporting || shapes.length === 0}>
                {exporting ? "Exporting..." : "Download SVG"}
              </button>
              <button style={btnStyle} onClick={() => handleExport("png")} disabled={exporting || shapes.length === 0}>
                {exporting ? "Exporting..." : "Download PNG"}
              </button>
            </div>
          </div>
          <div style={{ ...cardStyle, fontSize: 13, color: "var(--text-secondary)" }}>
            {shapes.length} shape{shapes.length !== 1 ? "s" : ""} drawn
          </div>
        </div>
      )}
    </div>
  );
}
