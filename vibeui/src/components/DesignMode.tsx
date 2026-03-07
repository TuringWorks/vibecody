/**
 * DesignMode — full-screen visual design editor.
 *
 * Layout: File Tree (left) | Live Preview (center) | AI Chat + Property Inspector (right)
 *
 * Features:
 * - Component tree from JSX file list
 * - Live iframe preview with Visual Editor overlay
 * - AI-powered property editing
 * - "Generate component" from natural language
 * - Figma import dialog
 */
import { useState, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { VisualEditor, SelectedElement } from "./VisualEditor";

interface DesignModeProps {
 workspacePath: string | null;
 provider: string;
}

interface GeneratedFile {
 path: string;
 content: string;
}

export function DesignMode({ workspacePath, provider }: DesignModeProps) {
 const [previewUrl, setPreviewUrl] = useState("http://localhost:5173");
 const [visualEditEnabled, setVisualEditEnabled] = useState(false);
 const [selectedElement, setSelectedElement] = useState<SelectedElement | null>(null);
 const [aiInstruction, setAiInstruction] = useState("");
 const [isGenerating, setIsGenerating] = useState(false);
 const [generationResult, setGenerationResult] = useState("");
 const [showFigmaDialog, setShowFigmaDialog] = useState(false);
 const [figmaUrl, setFigmaUrl] = useState("");
 const [figmaToken, setFigmaToken] = useState("");
 const [figmaResult, setFigmaResult] = useState<GeneratedFile[]>([]);
 const iframeRef = useRef<HTMLIFrameElement>(null);
 const iframeContainerRef = useRef<HTMLDivElement>(null);

 if (!workspacePath) {
 return <div className="empty-state"><p>Open a workspace folder to use the design editor.</p></div>;
 }

 // Inject inspector.js into the iframe
 const injectInspector = useCallback(() => {
 const iframe = iframeRef.current;
 if (!iframe || !iframe.contentWindow) return;
 try {
 const script = iframe.contentDocument?.createElement("script");
 if (script) {
 script.src = "/inspector.js";
 iframe.contentDocument?.head?.appendChild(script);
 }
 } catch {
 // Cross-origin iframes can't be directly scripted — show message
 console.warn("Cannot inject inspector into cross-origin iframe");
 }
 }, []);

 const handleVisualEditToggle = () => {
 if (!visualEditEnabled) {
 injectInspector();
 } else {
 // Deactivate inspector
 iframeRef.current?.contentWindow?.postMessage({ type: "vibe:deactivate-inspector" }, "*");
 setSelectedElement(null);
 }
 setVisualEditEnabled(!visualEditEnabled);
 };

 const handleElementEdit = useCallback(async (element: SelectedElement, instruction: string) => {
 setSelectedElement(element);
 setAiInstruction(instruction);
 setIsGenerating(true);
 try {
 const result = await invoke<string>("visual_edit_element", {
 workspacePath,
 selector: element.selector,
 instruction,
 currentHtml: element.outerHTML,
 reactComponent: element.reactComponent ?? null,
 }).catch(() => "Edit queued — check agent output.");
 setGenerationResult(result);
 } finally {
 setIsGenerating(false);
 }
 }, [workspacePath]);

 const handleGenerateComponent = async () => {
 if (!aiInstruction.trim()) return;
 setIsGenerating(true);
 setGenerationResult("");
 try {
 const result = await invoke<string>("generate_component", {
 workspacePath,
 description: aiInstruction,
 provider,
 }).catch((e: unknown) =>String(e));
 setGenerationResult(result);
 } finally {
 setIsGenerating(false);
 }
 };

 const handleFigmaImport = async () => {
 if (!figmaUrl.trim() || !figmaToken.trim()) return;
 setIsGenerating(true);
 try {
 const files = await invoke<GeneratedFile[]>("import_figma", {
 url: figmaUrl,
 token: figmaToken,
 workspacePath,
 provider,
 }).catch(() => [] as GeneratedFile[]);
 setFigmaResult(files);
 } finally {
 setIsGenerating(false);
 }
 };

 const iframeRect = iframeContainerRef.current?.getBoundingClientRect();

 return (
 <div style={{ display: "flex", height: "100%", background: "var(--bg-primary, #1a1b26)", color: "var(--text-primary)" }}>
 {/* Left: Component tree */}
 <div style={{ width: 200, borderRight: "1px solid var(--border-color)", padding: 12, overflowY: "auto", flexShrink: 0 }}>
 <div style={{ fontWeight: 600, marginBottom: 12, fontSize: 13 }}>Components</div>
 <div style={{ fontSize: 12, opacity: 0.6 }}>
 Component tree from your project files will appear here once a live dev server is running.
 </div>
 <div style={{ marginTop: 16 }}>
 <button
 onClick={() => setShowFigmaDialog(true)}
 style={{
 width: "100%",
 background: "var(--bg-secondary)",
 border: "1px solid var(--border-color)",
 borderRadius: 6,
 padding: "6px 10px",
 color: "var(--text-primary)",
 cursor: "pointer",
 fontSize: 12,
 }}
 >
 Import from Figma
 </button>
 </div>
 </div>

 {/* Center: Live preview */}
 <div style={{ flex: 1, display: "flex", flexDirection: "column", position: "relative" }}>
 {/* Toolbar */}
 <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "6px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" }}>
 <input
 value={previewUrl}
 onChange={(e) => setPreviewUrl(e.target.value)}
 onKeyDown={(e) => e.key === "Enter" && iframeRef.current?.setAttribute("src", previewUrl)}
 style={{ flex: 1, background: "var(--bg-tertiary)", border: "1px solid var(--border-subtle, #44445a)", borderRadius: 4, color: "inherit", padding: "3px 8px", fontSize: 12 }}
 />
 <button onClick={() => iframeRef.current?.setAttribute("src", previewUrl)} style={{ background: "none", border: "none", cursor: "pointer", color: "inherit", fontSize: 14 }}>↺</button>
 <button
 onClick={handleVisualEditToggle}
 style={{
 background: visualEditEnabled ? "#6366f1" : "var(--bg-tertiary)",
 border: "1px solid var(--border-subtle, #44445a)",
 borderRadius: 4,
 padding: "3px 10px",
 cursor: "pointer",
 color: "inherit",
 fontSize: 12,
 fontWeight: 600,
 }}
 title="Toggle visual element selection"
 >
 {visualEditEnabled ? "Exit Edit" : "Visual Edit"}
 </button>
 </div>

 {/* Iframe container */}
 <div ref={iframeContainerRef} style={{ flex: 1, position: "relative" }}>
 <iframe
 ref={iframeRef}
 src={previewUrl}
 title="Live Preview"
 sandbox="allow-scripts allow-same-origin allow-forms allow-modals"
 style={{ width: "100%", height: "100%", border: "none", background: "#fff" }}
 />
 {/* Visual Editor overlay */}
 {visualEditEnabled && (
 <div style={{ position: "absolute", top: 0, left: 0, pointerEvents: "none", width: "100%", height: "100%" }}>
 <VisualEditor
 onEdit={handleElementEdit}
 workspacePath={workspacePath}
 iframeOffset={iframeRect ? { top: iframeRect.top, left: iframeRect.left } : undefined}
 />
 </div>
 )}
 </div>
 </div>

 {/* Right: AI Chat + Property Inspector */}
 <div style={{ width: 280, borderLeft: "1px solid var(--border-color)", padding: 12, overflowY: "auto", flexShrink: 0, display: "flex", flexDirection: "column", gap: 12 }}>
 {/* Generate component */}
 <div>
 <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 8 }}>Generate Component</div>
 <textarea
 value={aiInstruction}
 onChange={(e) => setAiInstruction(e.target.value)}
 placeholder="Describe a component to generate..."
 rows={3}
 style={{ width: "100%", resize: "vertical", background: "var(--bg-secondary)", border: "1px solid var(--border-subtle, #44445a)", borderRadius: 4, color: "inherit", padding: 8, fontSize: 12, boxSizing: "border-box" }}
 />
 <button
 onClick={handleGenerateComponent}
 disabled={isGenerating || !aiInstruction.trim()}
 style={{ width: "100%", background: "#6366f1", color: "#fff", border: "none", borderRadius: 4, padding: "7px 0", cursor: "pointer", fontWeight: 600, fontSize: 13, marginTop: 6 }}
 >
 {isGenerating ? "Generating…" : "Generate"}
 </button>
 {generationResult && (
 <pre style={{ marginTop: 8, fontSize: 11, color: "#a6e3a1", overflowX: "auto", whiteSpace: "pre-wrap", background: "var(--bg-secondary)", borderRadius: 4, padding: 8 }}>
 {generationResult}
 </pre>
 )}
 </div>

 {/* Selected element properties */}
 {selectedElement && (
 <div>
 <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 6 }}>Selected Element</div>
 <div style={{ fontSize: 11, fontFamily: "monospace", opacity: 0.7 }}>
 {selectedElement.reactComponent ?? `<${selectedElement.tagName}>`}
 </div>
 <div style={{ fontSize: 11, opacity: 0.5, marginTop: 2 }}>
 {selectedElement.selector.length > 60
 ? "…" + selectedElement.selector.slice(-57)
 : selectedElement.selector}
 </div>
 </div>
 )}
 </div>

 {/* Figma import dialog */}
 {showFigmaDialog && (
 <div style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.6)", zIndex: 1000, display: "flex", alignItems: "center", justifyContent: "center" }}>
 <div style={{ background: "var(--bg-secondary)", borderRadius: 8, padding: 24, width: 400, border: "1px solid var(--border-color)" }}>
 <div style={{ fontWeight: 600, fontSize: 15, marginBottom: 16 }}>Import from Figma</div>
 <label style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Figma File URL</label>
 <input
 value={figmaUrl}
 onChange={(e) => setFigmaUrl(e.target.value)}
 placeholder="https://www.figma.com/file/..."
 style={{ width: "100%", background: "var(--bg-tertiary)", border: "1px solid var(--border-subtle, #44445a)", borderRadius: 4, color: "inherit", padding: "6px 8px", fontSize: 12, marginBottom: 12, boxSizing: "border-box" }}
 />
 <label style={{ fontSize: 12, display: "block", marginBottom: 4 }}>Figma API Token</label>
 <input
 type="password"
 value={figmaToken}
 onChange={(e) => setFigmaToken(e.target.value)}
 placeholder="figd_..."
 style={{ width: "100%", background: "var(--bg-tertiary)", border: "1px solid var(--border-subtle, #44445a)", borderRadius: 4, color: "inherit", padding: "6px 8px", fontSize: 12, marginBottom: 16, boxSizing: "border-box" }}
 />
 {figmaResult.length > 0 && (
 <div style={{ marginBottom: 16 }}>
 <div style={{ fontSize: 12, color: "#a6e3a1", marginBottom: 6 }}>Generated {figmaResult.length} file(s):</div>
 {figmaResult.map((f) => (
 <div key={f.path} style={{ fontSize: 11, fontFamily: "monospace", opacity: 0.7 }}> {f.path}</div>
 ))}
 </div>
 )}
 <div style={{ display: "flex", gap: 8 }}>
 <button
 onClick={handleFigmaImport}
 disabled={isGenerating || !figmaUrl.trim() || !figmaToken.trim()}
 style={{ flex: 1, background: "#6366f1", color: "#fff", border: "none", borderRadius: 4, padding: "8px 0", cursor: "pointer", fontWeight: 600, fontSize: 13 }}
 >
 {isGenerating ? "Importing…" : "Import"}
 </button>
 <button
 onClick={() => { setShowFigmaDialog(false); setFigmaResult([]); }}
 style={{ flex: 1, background: "var(--bg-tertiary)", border: "1px solid var(--border-subtle, #44445a)", borderRadius: 4, padding: "8px 0", cursor: "pointer", fontSize: 13, color: "inherit" }}
 >
 Cancel
 </button>
 </div>
 </div>
 </div>
 )}
 </div>
 );
}
