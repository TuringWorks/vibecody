/**
 * GpuTerminalPanel — GPU-accelerated terminal monitoring and configuration.
 *
 * Backend selector, FPS meter, glyph atlas visualization, grid inspector,
 * benchmark runner, config editor, and GPU memory usage indicator.
 */
import { useState } from "react";

interface RenderStats {
 frame_time_us: number;
 gpu_memory_bytes: number;
 cells_rendered: number;
 dirty_cells: number;
}

interface BenchmarkResult {
 avg_fps: number;
 min_frame_us: number;
 max_frame_us: number;
 p99_frame_us: number;
 backend_name: string;
 frames_rendered: number;
}

interface GpuConfig {
 preferred_backend: string;
 font_size: number;
 vsync: boolean;
 max_fps: number;
 enable_ligatures: boolean;
 subpixel_rendering: boolean;
 cell_padding: number;
}

const BACKENDS = ["wgpu", "opengl", "metal", "software"];

const SAMPLE_STATS: RenderStats = {
 frame_time_us: 850,
 gpu_memory_bytes: 4_325_376,
 cells_rendered: 9600,
 dirty_cells: 42,
};

const SAMPLE_BENCHMARK: BenchmarkResult = {
 avg_fps: 1176.5,
 min_frame_us: 720,
 max_frame_us: 1350,
 p99_frame_us: 1280,
 backend_name: "software",
 frames_rendered: 100,
};

const SAMPLE_FPS_HISTORY = [1150, 1200, 1180, 1170, 1190, 1160, 1175, 1185, 1195, 1165, 1180, 1200, 1210, 1190, 1170, 1185, 1195, 1175, 1180, 1200];

const SAMPLE_GLYPHS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789{}[]()<>+-*/=!@#$%^&_|\\;:'\",.<>?";

export default function GpuTerminalPanel() {
 const [config, setConfig] = useState<GpuConfig>({
   preferred_backend: "software",
   font_size: 14,
   vsync: true,
   max_fps: 120,
   enable_ligatures: false,
   subpixel_rendering: true,
   cell_padding: 1.0,
 });
 const [tab, setTab] = useState<"monitor" | "atlas" | "config" | "benchmark">("monitor");
 const [benchmarkResult, setBenchmarkResult] = useState<BenchmarkResult | null>(null);
 const [selectedGlyph, setSelectedGlyph] = useState<string | null>(null);
 const [hoveredCell, setHoveredCell] = useState<{ row: number; col: number } | null>(null);

 const formatBytes = (b: number) => {
   if (b < 1024) return `${b} B`;
   if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
   return `${(b / (1024 * 1024)).toFixed(1)} MB`;
 };

 const maxFps = Math.max(...SAMPLE_FPS_HISTORY);

 return (
   <div style={{ padding: 16, color: "var(--vp-c-text)", background: "var(--vp-c-bg)", minHeight: "100%" }}>
     <h2 style={{ margin: "0 0 12px", fontSize: 18 }}>GPU Terminal</h2>

     {/* Backend selector */}
     <div style={{ display: "flex", gap: 6, marginBottom: 12 }}>
       {BACKENDS.map(b => (
         <button key={b} onClick={() => setConfig({ ...config, preferred_backend: b })} style={{
           padding: "4px 12px", border: "1px solid var(--vp-c-border)", borderRadius: 4, cursor: "pointer",
           background: config.preferred_backend === b ? "var(--vp-c-brand)" : "transparent",
           color: config.preferred_backend === b ? "#fff" : "var(--vp-c-text)",
         }}>{b.charAt(0).toUpperCase() + b.slice(1)}</button>
       ))}
     </div>

     {/* Tabs */}
     <div style={{ display: "flex", gap: 4, marginBottom: 12 }}>
       {(["monitor", "atlas", "config", "benchmark"] as const).map(t => (
         <button key={t} onClick={() => setTab(t)} style={{
           padding: "4px 12px", border: "1px solid var(--vp-c-border)", borderRadius: 4, cursor: "pointer",
           background: tab === t ? "var(--vp-c-brand)" : "transparent", color: tab === t ? "#fff" : "var(--vp-c-text)",
         }}>{t.charAt(0).toUpperCase() + t.slice(1)}</button>
       ))}
     </div>

     {tab === "monitor" && (
       <>
         {/* Stats cards */}
         <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 8, marginBottom: 12 }}>
           <div style={{ padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, textAlign: "center" }}>
             <div style={{ fontSize: 22, fontWeight: 700, color: "var(--vp-c-success)" }}>
               {(1_000_000 / SAMPLE_STATS.frame_time_us).toFixed(0)}
             </div>
             <div style={{ fontSize: 11 }}>FPS</div>
           </div>
           <div style={{ padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, textAlign: "center" }}>
             <div style={{ fontSize: 22, fontWeight: 700, color: "var(--vp-c-brand)" }}>
               {SAMPLE_STATS.frame_time_us}
             </div>
             <div style={{ fontSize: 11 }}>Frame Time (us)</div>
           </div>
           <div style={{ padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, textAlign: "center" }}>
             <div style={{ fontSize: 22, fontWeight: 700, color: "var(--vp-c-warning)" }}>
               {formatBytes(SAMPLE_STATS.gpu_memory_bytes)}
             </div>
             <div style={{ fontSize: 11 }}>Memory</div>
           </div>
           <div style={{ padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, textAlign: "center" }}>
             <div style={{ fontSize: 22, fontWeight: 700, color: "var(--vp-c-danger)" }}>
               {SAMPLE_STATS.dirty_cells}
             </div>
             <div style={{ fontSize: 11 }}>Dirty Cells</div>
           </div>
         </div>

         {/* FPS graph */}
         <div style={{ marginBottom: 12 }}>
           <strong style={{ fontSize: 12 }}>FPS History</strong>
           <svg width="100%" height={80} style={{ border: "1px solid var(--vp-c-border)", borderRadius: 6, background: "#11111b", marginTop: 4 }}>
             {SAMPLE_FPS_HISTORY.map((fps, i) => {
               const x = (i / (SAMPLE_FPS_HISTORY.length - 1)) * 100;
               const y = 75 - (fps / maxFps) * 70;
               return i > 0 ? (
                 <line key={i}
                   x1={`${((i - 1) / (SAMPLE_FPS_HISTORY.length - 1)) * 100}%`}
                   y1={75 - (SAMPLE_FPS_HISTORY[i - 1] / maxFps) * 70}
                   x2={`${x}%`} y2={y}
                   stroke="var(--vp-c-success)" strokeWidth={2} />
               ) : null;
             })}
           </svg>
         </div>

         {/* Grid inspector */}
         <strong style={{ fontSize: 12 }}>Grid Inspector (120x40)</strong>
         <div style={{ marginTop: 4, overflow: "auto", maxHeight: 120 }}>
           <div style={{ display: "grid", gridTemplateColumns: "repeat(40, 12px)", gap: 0 }}>
             {Array.from({ length: 200 }, (_, i) => {
               const row = Math.floor(i / 40);
               const col = i % 40;
               const ch = String.fromCharCode(65 + (i % 26));
               return (
                 <div key={i}
                   onMouseEnter={() => setHoveredCell({ row, col })}
                   onMouseLeave={() => setHoveredCell(null)}
                   style={{
                     width: 12, height: 14, fontSize: 9, fontFamily: "monospace",
                     textAlign: "center", lineHeight: "14px", cursor: "pointer",
                     background: hoveredCell?.row === row && hoveredCell?.col === col ? "var(--vp-c-brand)" : "transparent",
                     color: i % 7 === 0 ? "var(--vp-c-success)" : "var(--vp-c-text)",
                   }}>{ch}</div>
               );
             })}
           </div>
         </div>
         {hoveredCell && (
           <div style={{ fontSize: 11, marginTop: 4, color: "var(--vp-c-border)" }}>
             Cell [{hoveredCell.row}, {hoveredCell.col}] | fg: #cdd6f4 | bg: #1e1e2e | bold: false
           </div>
         )}
       </>
     )}

     {tab === "atlas" && (
       <>
         <strong style={{ fontSize: 12 }}>Glyph Atlas (1024x1024, {SAMPLE_GLYPHS.length} glyphs cached)</strong>
         <div style={{
           marginTop: 8, padding: 8, background: "#11111b", border: "1px solid var(--vp-c-border)",
           borderRadius: 6, display: "flex", flexWrap: "wrap", gap: 2,
         }}>
           {SAMPLE_GLYPHS.split("").map((ch, i) => (
             <div key={i} onClick={() => setSelectedGlyph(ch)} style={{
               width: 22, height: 24, display: "flex", alignItems: "center", justifyContent: "center",
               fontFamily: "monospace", fontSize: 13, cursor: "pointer", borderRadius: 2,
               background: selectedGlyph === ch ? "var(--vp-c-brand)" : "#313244",
               color: selectedGlyph === ch ? "#fff" : "var(--vp-c-text)",
             }}>{ch}</div>
           ))}
         </div>
         {selectedGlyph && (
           <div style={{ marginTop: 8, padding: 8, border: "1px solid var(--vp-c-border)", borderRadius: 6, fontSize: 12 }}>
             <strong style={{ fontSize: 24, fontFamily: "monospace" }}>{selectedGlyph}</strong>
             <div style={{ marginTop: 4 }}>
               Codepoint: U+{selectedGlyph.charCodeAt(0).toString(16).toUpperCase().padStart(4, "0")} |
               Width: 8px | Height: 21px | Advance: 8.4
             </div>
           </div>
         )}
         <div style={{ marginTop: 8, fontSize: 12 }}>
           Atlas utilization: <strong>{((SAMPLE_GLYPHS.length * 8 * 21) / (1024 * 1024) * 100).toFixed(2)}%</strong>
         </div>
       </>
     )}

     {tab === "config" && (
       <div style={{ display: "grid", gap: 10, maxWidth: 400 }}>
         <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
           <span>Font Size</span>
           <input type="number" value={config.font_size} min={8} max={32}
             onChange={e => setConfig({ ...config, font_size: parseFloat(e.target.value) })}
             style={{ width: 70, padding: 4, background: "var(--vp-c-bg)", color: "var(--vp-c-text)", border: "1px solid var(--vp-c-border)", borderRadius: 4 }} />
         </label>
         <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
           <span>Max FPS</span>
           <input type="number" value={config.max_fps} min={30} max={240}
             onChange={e => setConfig({ ...config, max_fps: parseFloat(e.target.value) })}
             style={{ width: 70, padding: 4, background: "var(--vp-c-bg)", color: "var(--vp-c-text)", border: "1px solid var(--vp-c-border)", borderRadius: 4 }} />
         </label>
         <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
           <span>Cell Padding</span>
           <input type="number" value={config.cell_padding} min={0} max={4} step={0.5}
             onChange={e => setConfig({ ...config, cell_padding: parseFloat(e.target.value) })}
             style={{ width: 70, padding: 4, background: "var(--vp-c-bg)", color: "var(--vp-c-text)", border: "1px solid var(--vp-c-border)", borderRadius: 4 }} />
         </label>
         <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
           <span>VSync</span>
           <input type="checkbox" checked={config.vsync} onChange={e => setConfig({ ...config, vsync: e.target.checked })} />
         </label>
         <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
           <span>Ligatures</span>
           <input type="checkbox" checked={config.enable_ligatures} onChange={e => setConfig({ ...config, enable_ligatures: e.target.checked })} />
         </label>
         <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
           <span>Subpixel Rendering</span>
           <input type="checkbox" checked={config.subpixel_rendering} onChange={e => setConfig({ ...config, subpixel_rendering: e.target.checked })} />
         </label>
       </div>
     )}

     {tab === "benchmark" && (
       <>
         <button onClick={() => setBenchmarkResult(SAMPLE_BENCHMARK)} style={{
           padding: "8px 16px", background: "var(--vp-c-brand)", color: "#fff", border: "none",
           borderRadius: 4, cursor: "pointer", marginBottom: 12,
         }}>Run Benchmark (100 frames)</button>

         {benchmarkResult && (
           <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 8 }}>
             <div style={{ padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, textAlign: "center" }}>
               <div style={{ fontSize: 22, fontWeight: 700, color: "var(--vp-c-success)" }}>{benchmarkResult.avg_fps.toFixed(1)}</div>
               <div style={{ fontSize: 11 }}>Avg FPS</div>
             </div>
             <div style={{ padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, textAlign: "center" }}>
               <div style={{ fontSize: 22, fontWeight: 700, color: "var(--vp-c-brand)" }}>{benchmarkResult.min_frame_us}</div>
               <div style={{ fontSize: 11 }}>Min Frame (us)</div>
             </div>
             <div style={{ padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, textAlign: "center" }}>
               <div style={{ fontSize: 22, fontWeight: 700, color: "var(--vp-c-warning)" }}>{benchmarkResult.max_frame_us}</div>
               <div style={{ fontSize: 11 }}>Max Frame (us)</div>
             </div>
             <div style={{ padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, textAlign: "center" }}>
               <div style={{ fontSize: 22, fontWeight: 700, color: "var(--vp-c-danger)" }}>{benchmarkResult.p99_frame_us}</div>
               <div style={{ fontSize: 11 }}>P99 Frame (us)</div>
             </div>
             <div style={{ padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, textAlign: "center" }}>
               <div style={{ fontSize: 22, fontWeight: 700 }}>{benchmarkResult.frames_rendered}</div>
               <div style={{ fontSize: 11 }}>Frames</div>
             </div>
             <div style={{ padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, textAlign: "center" }}>
               <div style={{ fontSize: 22, fontWeight: 700 }}>{benchmarkResult.backend_name}</div>
               <div style={{ fontSize: 11 }}>Backend</div>
             </div>
           </div>
         )}
       </>
     )}
   </div>
 );
}
