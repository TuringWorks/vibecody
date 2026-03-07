/**
 * ColorPalettePanel — Color Palette & Design Token Manager.
 *
 * Create and manage named color palettes, scan CSS/SCSS/Tailwind variables
 * from the workspace, and export to CSS custom properties, SCSS variables,
 * Tailwind config, or JSON.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ColorToken {
 name: string;
 value: string;
 comment?: string;
}

interface ColorPalette {
 id: string;
 name: string;
 tokens: ColorToken[];
}

type ExportFormat = "css" | "scss" | "tailwind" | "json";

const EXPORT_FORMATS: { value: ExportFormat; label: string }[] = [
 { value: "css", label: "CSS Variables" },
 { value: "scss", label: "SCSS Variables" },
 { value: "tailwind", label: "Tailwind Config" },
 { value: "json", label: "JSON" },
];

const STARTER_PALETTES: ColorPalette[] = [
 {
 id: "catppuccin",
 name: "Catppuccin Mocha",
 tokens: [
 { name: "ctp-rosewater", value: "#f5e0dc" },
 { name: "ctp-flamingo", value: "#f2cdcd" },
 { name: "ctp-pink", value: "#f5c2e7" },
 { name: "ctp-mauve", value: "#cba6f7" },
 { name: "ctp-red", value: "#f38ba8" },
 { name: "ctp-maroon", value: "#eba0ac" },
 { name: "ctp-peach", value: "#fab387" },
 { name: "ctp-yellow", value: "#f9e2af" },
 { name: "ctp-green", value: "#a6e3a1" },
 { name: "ctp-teal", value: "#94e2d5" },
 { name: "ctp-sky", value: "#89dceb" },
 { name: "ctp-sapphire", value: "#74c7ec" },
 { name: "ctp-blue", value: "#89b4fa" },
 { name: "ctp-lavender", value: "#b4befe" },
 { name: "ctp-text", value: "var(--text-primary)" },
 { name: "ctp-base", value: "var(--bg-tertiary)" },
 { name: "ctp-mantle", value: "var(--bg-primary)" },
 { name: "ctp-crust", value: "#11111b" },
 ],
 },
 {
 id: "tailwind-core",
 name: "Tailwind Core",
 tokens: [
 { name: "tw-blue-500", value: "#3b82f6" },
 { name: "tw-indigo-500", value: "#6366f1" },
 { name: "tw-violet-500", value: "#8b5cf6" },
 { name: "tw-green-500", value: "#22c55e" },
 { name: "tw-amber-500", value: "#f59e0b" },
 { name: "tw-red-500", value: "#ef4444" },
 { name: "tw-gray-50", value: "#f9fafb" },
 { name: "tw-gray-900", value: "#111827" },
 ],
 },
];

function hexToRgb(hex: string): string {
 const h = hex.replace("#", "");
 if (h.length === 3) {
 const r = parseInt(h[0] + h[0], 16);
 const g = parseInt(h[1] + h[1], 16);
 const b = parseInt(h[2] + h[2], 16);
 return `rgb(${r}, ${g}, ${b})`;
 }
 const r = parseInt(h.slice(0, 2), 16);
 const g = parseInt(h.slice(2, 4), 16);
 const b = parseInt(h.slice(4, 6), 16);
 return `rgb(${r}, ${g}, ${b})`;
}


function Swatch({ token, onEdit, onRemove }: {
 token: ColorToken;
 onEdit: (t: ColorToken) => void;
 onRemove: () => void;
}) {
 const [copied, setCopied] = useState(false);
 const copy = () => {
 navigator.clipboard.writeText(token.value);
 setCopied(true);
 setTimeout(() => setCopied(false), 1200);
 };
 return (
 <div style={{ borderRadius: 8, overflow: "hidden", border: "1px solid var(--border-color)", cursor: "pointer" }} onClick={copy}>
 <div style={{ background: token.value, height: 56, display: "flex", alignItems: "flex-end", justifyContent: "flex-end", padding: "4px 6px", gap: 4 }}>
 <button onClick={e => { e.stopPropagation(); onEdit(token); }} style={{ background: "rgba(0,0,0,0.4)", border: "none", borderRadius: 3, color: "#fff", fontSize: 9, padding: "1px 5px", cursor: "pointer" }}></button>
 <button onClick={e => { e.stopPropagation(); onRemove(); }} style={{ background: "rgba(0,0,0,0.4)", border: "none", borderRadius: 3, color: "var(--text-danger, #f38ba8)", fontSize: 9, padding: "1px 5px", cursor: "pointer" }}>✕</button>
 </div>
 <div style={{ padding: "5px 7px", background: "var(--bg-secondary)" }}>
 <div style={{ fontSize: 9, fontWeight: 700, fontFamily: "monospace", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{copied ? "Copied!" : token.value}</div>
 <div style={{ fontSize: 9, color: "var(--text-muted)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>--{token.name}</div>
 </div>
 </div>
 );
}

export function ColorPalettePanel({ workspacePath }: { workspacePath: string | null }) {
 const [palettes, setPalettes] = useState<ColorPalette[]>([]);
 const [activeId, setActiveId] = useState<string | null>(null);
 const [exported, setExported] = useState("");
 const [exportFmt, setExportFmt] = useState<ExportFormat>("css");
 const [scanning, setScanning] = useState(false);
 const [editToken, setEditToken] = useState<ColorToken | null>(null);
 const [editIdx, setEditIdx] = useState<number | null>(null);
 const [newName, setNewName] = useState<string | null>(null);
 const [showExport, setShowExport] = useState(false);

 useEffect(() => {
 invoke<ColorPalette[]>("get_color_palettes").then(p => {
 const list = p.length > 0 ? p : STARTER_PALETTES;
 setPalettes(list);
 setActiveId(list[0]?.id ?? null);
 }).catch(() => {
 setPalettes(STARTER_PALETTES);
 setActiveId(STARTER_PALETTES[0].id);
 });
 }, []);

 const active = palettes.find(p => p.id === activeId) ?? null;

 const save = useCallback(async (list: ColorPalette[]) => {
 setPalettes(list);
 await invoke("save_color_palettes", { palettes: list }).catch(() => {});
 }, []);

 const updateActive = useCallback((tokens: ColorToken[]) => {
 if (!activeId) return;
 save(palettes.map(p => p.id === activeId ? { ...p, tokens } : p));
 }, [activeId, palettes, save]);

 const addPalette = () => {
 const p: ColorPalette = { id: `pal-${Date.now()}`, name: "New Palette", tokens: [] };
 const next = [...palettes, p];
 save(next);
 setActiveId(p.id);
 };

 const removePalette = (id: string) => {
 const next = palettes.filter(p => p.id !== id);
 save(next);
 if (activeId === id) setActiveId(next[0]?.id ?? null);
 };

 const addToken = () => {
 if (!active) return;
 const t: ColorToken = { name: `color-${active.tokens.length + 1}`, value: "#6366f1" };
 updateActive([...active.tokens, t]);
 };

 const removeToken = (idx: number) => {
 if (!active) return;
 updateActive(active.tokens.filter((_, i) => i !== idx));
 };

 const startEdit = (token: ColorToken, idx: number) => {
 setEditToken({ ...token });
 setEditIdx(idx);
 };

 const commitEdit = () => {
 if (!active || editIdx === null || !editToken) return;
 const tokens = active.tokens.map((t, i) => i === editIdx ? editToken : t);
 updateActive(tokens);
 setEditToken(null); setEditIdx(null);
 };

 const handleScan = async () => {
 if (!workspacePath) return;
 setScanning(true);
 try {
 const tokens = await invoke<ColorToken[]>("scan_css_variables", { workspace: workspacePath });
 if (tokens.length === 0) return;
 const p: ColorPalette = { id: `scan-${Date.now()}`, name: "Scanned from workspace", tokens };
 const next = [...palettes, p];
 await save(next);
 setActiveId(p.id);
 } finally {
 setScanning(false);
 }
 };

 const handleExport = async () => {
 if (!active) return;
 try {
 const out = await invoke<string>("export_color_palette", { palette: active, format: exportFmt });
 setExported(out);
 setShowExport(true);
 } catch (e) {
 setExported(String(e));
 setShowExport(true);
 }
 };

 const copyExport = () => { navigator.clipboard.writeText(exported); };

 return (
 <div style={{ display: "flex", height: "100%", overflow: "hidden" }}>
 {/* Palette sidebar */}
 <div style={{ width: 180, borderRight: "1px solid var(--border-color)", display: "flex", flexDirection: "column", flexShrink: 0 }}>
 <div style={{ padding: "8px 10px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", alignItems: "center", justifyContent: "space-between" }}>
 <span style={{ fontSize: 11, fontWeight: 600 }}>Palettes</span>
 <button onClick={addPalette} style={{ fontSize: 12, background: "none", border: "none", color: "var(--accent-primary, #6366f1)", cursor: "pointer", fontWeight: 700 }}>+</button>
 </div>
 <div style={{ flex: 1, overflowY: "auto" }}>
 {palettes.map(p => (
 <div
 key={p.id}
 onClick={() => setActiveId(p.id)}
 style={{ padding: "7px 10px", borderBottom: "1px solid var(--border-color)", cursor: "pointer", background: activeId === p.id ? "var(--accent-bg, rgba(99,102,241,0.15))" : "transparent", display: "flex", alignItems: "center", gap: 6 }}
 >
 {/* Mini swatch row */}
 <div style={{ display: "flex", gap: 2, flex: 1, minWidth: 0 }}>
 {p.tokens.slice(0, 5).map((t, i) => (
 <div key={i} style={{ width: 10, height: 10, borderRadius: 2, background: t.value, flexShrink: 0 }} />
 ))}
 </div>
 <div style={{ fontSize: 10, fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1 }}>{p.name}</div>
 <button onClick={e => { e.stopPropagation(); removePalette(p.id); }} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-danger, #f38ba8)", cursor: "pointer", flexShrink: 0 }}>✕</button>
 </div>
 ))}
 </div>
 <div style={{ padding: "8px 10px", borderTop: "1px solid var(--border-color)", display: "flex", flexDirection: "column", gap: 5 }}>
 {workspacePath && (
 <button onClick={handleScan} disabled={scanning} style={{ padding: "4px 8px", fontSize: 10, fontWeight: 600, background: scanning ? "var(--bg-secondary)" : "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: scanning ? "not-allowed" : "pointer" }}>
 {scanning ? "Scanning…" : "Scan CSS vars"}
 </button>
 )}
 </div>
 </div>

 {/* Main area */}
 <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
 {!active ? (
 <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", color: "var(--text-muted)", fontSize: 13 }}>
 ← Select or create a palette
 </div>
 ) : (
 <>
 {/* Toolbar */}
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center" }}>
 {newName !== null ? (
 <input
 autoFocus
 value={newName}
 onChange={e => setNewName(e.target.value)}
 onBlur={() => { save(palettes.map(p => p.id === activeId ? { ...p, name: newName } : p)); setNewName(null); }}
 onKeyDown={e => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
 style={{ flex: 1, fontSize: 13, fontWeight: 600, background: "transparent", border: "none", borderBottom: "1px solid #6366f1", outline: "none", color: "var(--text-primary)" }}
 />
 ) : (
 <span style={{ fontSize: 13, fontWeight: 600, cursor: "pointer", flex: 1 }} onClick={() => setNewName(active.name)} title="Click to rename">{active.name}</span>
 )}
 <span style={{ fontSize: 10, color: "var(--text-muted)" }}>{active.tokens.length} tokens</span>
 <button onClick={addToken} style={{ padding: "3px 12px", fontSize: 11, fontWeight: 700, background: "var(--accent-primary, #6366f1)", border: "none", borderRadius: 4, color: "#fff", cursor: "pointer" }}>+ Color</button>
 <select value={exportFmt} onChange={e => setExportFmt(e.target.value as ExportFormat)} style={{ fontSize: 10, padding: "3px 6px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)" }}>
 {EXPORT_FORMATS.map(f => <option key={f.value} value={f.value}>{f.label}</option>)}
 </select>
 <button onClick={handleExport} style={{ padding: "3px 12px", fontSize: 11, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer" }}>↗ Export</button>
 </div>

 {/* Token edit modal */}
 {editToken && editIdx !== null && (
 <div style={{ padding: "10px 14px", borderBottom: "1px solid var(--border-color)", background: "rgba(99,102,241,0.08)", display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
 <input type="color" value={editToken.value.startsWith("#") ? editToken.value.slice(0, 7) : "#6366f1"} onChange={e => setEditToken({ ...editToken, value: e.target.value })} style={{ width: 40, height: 32, border: "none", borderRadius: 4, cursor: "pointer" }} />
 <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
 <label style={{ fontSize: 9, color: "var(--text-muted)", fontWeight: 600 }}>Name</label>
 <input value={editToken.name} onChange={e => setEditToken({ ...editToken, name: e.target.value })} style={{ padding: "3px 8px", fontSize: 11, fontFamily: "monospace", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none", width: 160 }} />
 </div>
 <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
 <label style={{ fontSize: 9, color: "var(--text-muted)", fontWeight: 600 }}>Hex</label>
 <input value={editToken.value} onChange={e => setEditToken({ ...editToken, value: e.target.value })} style={{ padding: "3px 8px", fontSize: 11, fontFamily: "monospace", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none", width: 100 }} />
 </div>
 <div style={{ fontSize: 10, color: "var(--text-muted)" }}>{hexToRgb(editToken.value)}</div>
 <button onClick={commitEdit} style={{ padding: "4px 14px", fontSize: 11, fontWeight: 700, background: "var(--accent-primary, #6366f1)", border: "none", borderRadius: 4, color: "#fff", cursor: "pointer", marginLeft: "auto" }}>✓ Done</button>
 </div>
 )}

 {/* Swatch grid */}
 <div style={{ flex: 1, overflow: "auto", padding: 14 }}>
 {active.tokens.length === 0 ? (
 <div style={{ textAlign: "center", color: "var(--text-muted)", fontSize: 12, paddingTop: 40 }}>
 No colors yet — click <b>+ Color</b> or scan CSS variables
 </div>
 ) : (
 <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(110px, 1fr))", gap: 10 }}>
 {active.tokens.map((t, i) => (
 <Swatch key={i} token={t} onEdit={tok => startEdit(tok, i)} onRemove={() => removeToken(i)} />
 ))}
 </div>
 )}
 </div>

 {/* Export panel */}
 {showExport && (
 <div style={{ height: 200, borderTop: "1px solid var(--border-color)", display: "flex", flexDirection: "column" }}>
 <div style={{ padding: "5px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
 <span style={{ fontSize: 11, fontWeight: 600 }}>{EXPORT_FORMATS.find(f => f.value === exportFmt)?.label} output</span>
 <div style={{ display: "flex", gap: 6 }}>
 <button onClick={copyExport} style={{ fontSize: 10, padding: "2px 10px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer" }}>Copy</button>
 <button onClick={() => setShowExport(false)} style={{ fontSize: 10, padding: "2px 8px", background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}>✕</button>
 </div>
 </div>
 <pre style={{ flex: 1, overflowY: "auto", margin: 0, padding: "10px 14px", fontSize: 11, fontFamily: "monospace", lineHeight: 1.6, color: "var(--text-primary)", whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
 {exported}
 </pre>
 </div>
 )}
 </>
 )}
 </div>
 </div>
 );
}
