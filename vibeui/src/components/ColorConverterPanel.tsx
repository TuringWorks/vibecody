/**
 * ColorConverterPanel — Color format converter & inspector.
 *
 * • Native color picker + hex text input — all formats sync live.
 * • Conversions: HEX, RGB, RGBA, HSL, HSLA, HSV, CMYK, CSS var.
 * • Tints (+ white) and Shades (+ black) — 11 steps each.
 * • WCAG 2.1 contrast ratio against white, black, and custom background.
 * • CSS usage snippets: background, text, border, box-shadow, gradient.
 *
 * Pure TypeScript + native browser APIs — no Tauri commands required.
 */
import { useState, useMemo, useCallback } from "react";
import { CopyButton as CopyBtn } from "./shared/CopyButton";

// ── Colour math ────────────────────────────────────────────────────────────────

interface RGB { r: number; g: number; b: number } // 0–255
interface HSL { h: number; s: number; l: number } // h 0–360, s/l 0–100
interface HSV { h: number; s: number; v: number } // h 0–360, s/v 0–100
interface CMYK { c: number; m: number; y: number; k: number } // 0–100

function hexToRgb(hex: string): RGB | null {
 const clean = hex.replace(/^#/, "");
 const full = clean.length === 3
 ? clean.split("").map(c => c + c).join("")
 : clean.length === 6 ? clean : null;
 if (!full) return null;
 const n = parseInt(full, 16);
 if (isNaN(n)) return null;
 return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 };
}

function rgbToHex({ r, g, b }: RGB): string {
 return "#" + [r, g, b].map(v =>Math.round(v).toString(16).padStart(2, "0")).join("").toUpperCase();
}

function rgbToHsl({ r, g, b }: RGB): HSL {
 const rn = r / 255, gn = g / 255, bn = b / 255;
 const max = Math.max(rn, gn, bn), min = Math.min(rn, gn, bn);
 const l = (max + min) / 2;
 if (max === min) return { h: 0, s: 0, l: Math.round(l * 100) };
 const d = max - min;
 const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
 let h = 0;
 if (max === rn) h = ((gn - bn) / d + (gn < bn ? 6 : 0)) / 6;
 else if (max === gn) h = ((bn - rn) / d + 2) / 6;
 else h = ((rn - gn) / d + 4) / 6;
 return { h: Math.round(h * 360), s: Math.round(s * 100), l: Math.round(l * 100) };
}


function rgbToHsv({ r, g, b }: RGB): HSV {
 const rn = r / 255, gn = g / 255, bn = b / 255;
 const max = Math.max(rn, gn, bn), min = Math.min(rn, gn, bn);
 const v = max, d = max - min;
 const s = max === 0 ? 0 : d / max;
 let h = 0;
 if (d !== 0) {
 if (max === rn) h = ((gn - bn) / d + (gn < bn ? 6 : 0)) / 6;
 else if (max === gn) h = ((bn - rn) / d + 2) / 6;
 else h = ((rn - gn) / d + 4) / 6;
 }
 return { h: Math.round(h * 360), s: Math.round(s * 100), v: Math.round(v * 100) };
}

function rgbToCmyk({ r, g, b }: RGB): CMYK {
 const rn = r / 255, gn = g / 255, bn = b / 255;
 const k = 1 - Math.max(rn, gn, bn);
 if (k === 1) return { c: 0, m: 0, y: 0, k: 100 };
 return {
 c: Math.round((1 - rn - k) / (1 - k) * 100),
 m: Math.round((1 - gn - k) / (1 - k) * 100),
 y: Math.round((1 - bn - k) / (1 - k) * 100),
 k: Math.round(k * 100),
 };
}

// ── WCAG contrast ──────────────────────────────────────────────────────────────

function luminance({ r, g, b }: RGB): number {
 const lin = (v: number) => { const n = v / 255; return n <= 0.04045 ? n / 12.92 : ((n + 0.055) / 1.055) ** 2.4; };
 return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

function contrastRatio(a: RGB, b: RGB): number {
 const la = luminance(a), lb = luminance(b);
 return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}

function wcagGrade(ratio: number): { aa: boolean; aaa: boolean; aaLg: boolean; aaaLg: boolean } {
 return { aa: ratio >= 4.5, aaa: ratio >= 7, aaLg: ratio >= 3, aaaLg: ratio >= 4.5 };
}

// ── Tint / shade ───────────────────────────────────────────────────────────────

function mixRgb(a: RGB, b: RGB, t: number): RGB {
 return { r: Math.round(a.r + (b.r - a.r) * t), g: Math.round(a.g + (b.g - a.g) * t), b: Math.round(a.b + (b.b - a.b) * t) };
}

const WHITE: RGB = { r: 255, g: 255, b: 255 };
const BLACK: RGB = { r: 0, g: 0, b: 0 };

// ── Closest CSS named color ────────────────────────────────────────────────────

const CSS_COLORS: [string, string][] = [
 ["red","#FF0000"],["green","#008000"],["blue","#0000FF"],["white","#FFFFFF"],["black","#000000"],
 ["yellow","#FFFF00"],["cyan","#00FFFF"],["magenta","#FF00FF"],["orange","#FFA500"],["purple","#800080"],
 ["pink","#FFC0CB"],["brown","#A52A2A"],["gray","#808080"],["silver","#C0C0C0"],["gold","#FFD700"],
 ["coral","#FF7F50"],["salmon","#FA8072"],["khaki","#F0E68C"],["violet","#EE82EE"],["indigo","#4B0082"],
 ["navy","#000080"],["teal","#008080"],["lime","#00FF00"],["maroon","#800000"],["olive","#808000"],
 ["aquamarine","#7FFFD4"],["chartreuse","#7FFF00"],["crimson","#DC143C"],["turquoise","#40E0D0"],
 ["chocolate","#D2691E"],["tomato","#FF6347"],["orchid","#DA70D6"],["plum","#DDA0DD"],["tan","#D2B48C"],
];

function closestNamedColor(rgb: RGB): string {
 let best = CSS_COLORS[0][0], bestDist = Infinity;
 for (const [name, hex] of CSS_COLORS) {
 const c = hexToRgb(hex)!;
 const d = (c.r - rgb.r) ** 2 + (c.g - rgb.g) ** 2 + (c.b - rgb.b) ** 2;
 if (d < bestDist) { bestDist = d; best = name; }
 }
 return best;
}

// ── Component ──────────────────────────────────────────────────────────────────

type SubTab = "convert" | "tints" | "contrast" | "snippets";

// CopyBtn imported from shared/CopyButton.tsx

function FmtRow({ label, value }: { label: string; value: string }) {
 return (
 <div style={{ display: "flex", alignItems: "center", borderBottom: "1px solid var(--border-subtle)", padding: "5px 12px", gap: 10 }}>
 <span style={{ width: 100, flexShrink: 0, fontSize: 10, fontWeight: 700, color: "var(--text-secondary)" }}>{label}</span>
 <span style={{ flex: 1, fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--text-primary)", wordBreak: "break-all" }}>{value}</span>
 <CopyBtn text={value} />
 </div>
 );
}

export function ColorConverterPanel() {
 const [hex, setHex] = useState("#89B4FA");
 const [alpha, setAlpha] = useState(100);
 const [subTab, setSubTab] = useState<SubTab>("convert");
 const [bgHex, setBgHex] = useState("#1E1E2E"); // contrast custom bg

 const rgb = useMemo(() => hexToRgb(hex) ?? { r: 137, g: 180, b: 250 }, [hex]);
 const hsl = useMemo(() => rgbToHsl(rgb), [rgb]);
 const hsv = useMemo(() => rgbToHsv(rgb), [rgb]);
 const cmyk = useMemo(() => rgbToCmyk(rgb), [rgb]);
 const a = alpha / 100;

 const hexNorm = useMemo(() => rgbToHex(rgb), [rgb]);
 const nearestName = useMemo(() => closestNamedColor(rgb), [rgb]);

 const tints = useMemo(() =>Array.from({ length: 11 }, (_, i) => mixRgb(rgb, WHITE, i / 10)), [rgb]);
 const shades = useMemo(() =>Array.from({ length: 11 }, (_, i) => mixRgb(rgb, BLACK, i / 10)), [rgb]);

 const bgRgb = useMemo(() => hexToRgb(bgHex) ?? WHITE, [bgHex]);
 const contrastWhite = useMemo(() => contrastRatio(rgb, WHITE), [rgb]);
 const contrastBlack = useMemo(() => contrastRatio(rgb, BLACK), [rgb]);
 const contrastCustom = useMemo(() => contrastRatio(rgb, bgRgb), [rgb, bgRgb]);

 // ── Handle hex input ──────────────────────────────────────────────────────

 const handleHexInput = useCallback((s: string) => {
 setHex(s.startsWith("#") ? s : "#" + s);
 }, []);

 const handlePickerChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
 setHex(e.target.value.toUpperCase());
 }, []);

 // ── CSS snippets ──────────────────────────────────────────────────────────

 const cssSnippets = useMemo(() => [
 { label: "background-color", value: `background-color: ${hexNorm};` },
 { label: "color", value: `color: ${hexNorm};` },
 { label: "border", value: `border: 1px solid ${hexNorm};` },
 { label: "box-shadow", value: `box-shadow: 0 4px 16px ${hexNorm}40;` },
 { label: "RGBA fill", value: `background-color: rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${a.toFixed(2)});` },
 { label: "HSL", value: `color: hsl(${hsl.h}deg ${hsl.s}% ${hsl.l}%);` },
 { label: "CSS variable", value: `--color-accent: ${hexNorm};` },
 { label: "Tailwind (approx)", value: `/* Closest: text-[${hexNorm}] bg-[${hexNorm}] */` },
 { label: "Linear gradient", value: `background: linear-gradient(135deg, ${hexNorm}, ${rgbToHex(shades[5])});` },
 { label: "SVG fill", value: `fill="${hexNorm}"` },
 { label: "Android color", value: `android:color="${hexNorm}"` },
 { label: "Swift UIColor", value: `UIColor(red: ${(rgb.r/255).toFixed(3)}, green: ${(rgb.g/255).toFixed(3)}, blue: ${(rgb.b/255).toFixed(3)}, alpha: ${a.toFixed(2)})` },
 ], [hexNorm, rgb, hsl, a, shades]);

 // ── Contrast badge ────────────────────────────────────────────────────────

 const ContrastBadge = ({ ratio, bg }: { ratio: number; bg: RGB }) => {
 const { aa, aaa, aaLg, aaaLg } = wcagGrade(ratio);
 const isLightBg = luminance(bg) > 0.5;
 return (
 <div style={{ display: "flex", flexDirection: "column", gap: 6, padding: "10px 12px", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 6 }}>
 <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
 <div style={{ width: 48, height: 28, background: rgbToHex(bg), border: "1px solid var(--border-color)", borderRadius: 4, display: "flex", alignItems: "center", justifyContent: "center" }}>
 <span style={{ fontSize: 12, fontWeight: 700, color: hexNorm }}>Aa</span>
 </div>
 <div>
 <div style={{ fontSize: 15, fontWeight: 700, fontFamily: "var(--font-mono)", color: ratio >= 7 ? "var(--success-color, #a6e3a1)" : ratio >= 4.5 ? "var(--warning-color, #f9e2af)" : "var(--accent-rose)" }}>{ratio.toFixed(2)}:1</div>
 <div style={{ fontSize: 9, color: "var(--text-secondary)" }}>on {isLightBg ? "light" : "dark"} bg</div>
 </div>
 <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
 {([["AA", aa],["AAA", aaa],["AA-lg", aaLg],["AAA-lg", aaaLg]] as [string,boolean][]).map(([label, pass]) => (
 <span key={label} style={{ fontSize: 9, fontWeight: 700, padding: "1px 5px", borderRadius: 4, background: pass ? "color-mix(in srgb, var(--accent-green) 15%, transparent)" : "color-mix(in srgb, var(--accent-rose) 10%, transparent)", border: `1px solid ${pass ? "var(--success-color, #a6e3a1)" : "var(--accent-rose)"}`, color: pass ? "var(--success-color, #a6e3a1)" : "var(--accent-rose)" }}>{pass ? "✓" : "✕"} {label}</span>
 ))}
 </div>
 </div>
 </div>
 );
 };

 const TABS: { id: SubTab; label: string }[] = [
 { id: "convert", label: "Convert" },
 { id: "tints", label: "Tints & Shades" },
 { id: "contrast", label: "Contrast" },
 { id: "snippets", label: "CSS Snippets" },
 ];

 return (
 <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>

 {/* Header */}
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 6, alignItems: "center", flexWrap: "wrap" }}>
 <span style={{ fontSize: 13, fontWeight: 600 }}>Color Converter</span>
 {TABS.map(t => (
 <button key={t.id} onClick={() => setSubTab(t.id)} style={{ padding: "2px 10px", fontSize: 10, borderRadius: 10, background: subTab === t.id ? "color-mix(in srgb, var(--accent-blue) 20%, transparent)" : "var(--bg-primary)", border: `1px solid ${subTab === t.id ? "var(--accent-color, #6366f1)" : "var(--border-color)"}`, color: subTab === t.id ? "var(--info-color, #89b4fa)" : "var(--text-secondary)", cursor: "pointer", fontWeight: subTab === t.id ? 700 : 400 }}>{t.label}</button>
 ))}
 </div>

 {/* Picker row */}
 <div style={{ padding: "10px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
 {/* Color swatch + native picker */}
 <div style={{ position: "relative", width: 40, height: 40, flexShrink: 0 }}>
 <div style={{ width: 40, height: 40, background: hexNorm, borderRadius: 6, border: "2px solid var(--border-color)" }} />
 <input type="color" value={hexNorm} onChange={handlePickerChange}
 style={{ position: "absolute", inset: 0, opacity: 0, width: "100%", height: "100%", cursor: "pointer" }} />
 </div>
 {/* Hex input */}
 <input value={hex} onChange={e => handleHexInput(e.target.value)} maxLength={7} spellCheck={false}
 style={{ width: 100, padding: "5px 8px", fontSize: 13, fontFamily: "var(--font-mono)", fontWeight: 700, background: "var(--bg-primary)", border: `1px solid ${hexToRgb(hex) ? "var(--border-color)" : "var(--accent-rose)"}`, borderRadius: 4, color: hexNorm, outline: "none", letterSpacing: "0.05em" }} />
 {/* Alpha */}
 <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
 <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>α:</span>
 <input type="range" min={0} max={100} value={alpha} onChange={e => setAlpha(+e.target.value)} style={{ width: 80, accentColor: hexNorm }} />
 <span style={{ fontSize: 11, fontFamily: "var(--font-mono)", color: "var(--text-secondary)", width: 32 }}>{alpha}%</span>
 </div>
 {/* Nearest name */}
 <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>≈ <span style={{ color: hexNorm, fontWeight: 700 }}>{nearestName}</span></span>
 {/* Luminance */}
 <span style={{ fontSize: 10, color: "var(--text-secondary)", marginLeft: "auto" }}>
 L: <span style={{ fontFamily: "var(--font-mono)" }}>{luminance(rgb).toFixed(3)}</span>
 <span style={{ marginLeft: 8, fontSize: 9, padding: "1px 6px", borderRadius: 10, background: luminance(rgb) > 0.5 ? "rgba(249,226,175,0.15)" : "rgba(30,30,46,0.5)", border: "1px solid var(--border-color)", color: luminance(rgb) > 0.5 ? "var(--warning-color, #f9e2af)" : "var(--info-color, #89b4fa)" }}>
 {luminance(rgb) > 0.5 ? "light" : "dark"}
 </span>
 </span>
 </div>

 <div style={{ flex: 1, overflow: "auto" }}>

 {/* ── CONVERT ── */}
 {subTab === "convert" && (
 <div>
 <FmtRow label="HEX" value={hexNorm} />
 <FmtRow label="HEX (alpha)" value={`${hexNorm}${Math.round(alpha / 100 * 255).toString(16).padStart(2,"0").toUpperCase()}`} />
 <FmtRow label="RGB" value={`rgb(${rgb.r}, ${rgb.g}, ${rgb.b})`} />
 <FmtRow label="RGBA" value={`rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${a.toFixed(2)})`} />
 <FmtRow label="HSL" value={`hsl(${hsl.h}, ${hsl.s}%, ${hsl.l}%)`} />
 <FmtRow label="HSLA" value={`hsla(${hsl.h}, ${hsl.s}%, ${hsl.l}%, ${a.toFixed(2)})`} />
 <FmtRow label="HSV" value={`hsv(${hsv.h}, ${hsv.s}%, ${hsv.v}%)`} />
 <FmtRow label="CMYK" value={`cmyk(${cmyk.c}%, ${cmyk.m}%, ${cmyk.y}%, ${cmyk.k}%)`} />
 <FmtRow label="CSS var" value={`--color: ${hexNorm};`} />
 <FmtRow label="Float RGB" value={`vec3(${(rgb.r/255).toFixed(4)}, ${(rgb.g/255).toFixed(4)}, ${(rgb.b/255).toFixed(4)})`} />
 <FmtRow label="Decimal" value={`${parseInt(hexNorm.slice(1), 16)}`} />
 <FmtRow label="Android" value={`0xFF${hexNorm.slice(1)}`} />
 </div>
 )}

 {/* ── TINTS & SHADES ── */}
 {subTab === "tints" && (
 <div style={{ padding: "12px" }}>
 <div style={{ fontSize: 10, fontWeight: 700, color: "var(--text-info, #89b4fa)", marginBottom: 8, letterSpacing: "0.05em" }}>TINTS (mixed with white)</div>
 <div style={{ display: "flex", gap: 4, marginBottom: 16, flexWrap: "wrap" }}>
 {tints.map((c, i) => {
 const h = rgbToHex(c);
 return (
 <div key={i} role="button" tabIndex={0} onClick={() => setHex(h)} onKeyDown={e => e.key === "Enter" && setHex(h)} style={{ cursor: "pointer", display: "flex", flexDirection: "column", alignItems: "center", gap: 3 }}>
 <div style={{ width: 40, height: 40, background: h, borderRadius: 6, border: "1px solid var(--border-color)", transition: "transform 0.1s" }} title={h} />
 <span style={{ fontSize: 8, fontFamily: "var(--font-mono)", color: "var(--text-secondary)" }}>{i * 10}%</span>
 </div>
 );
 })}
 </div>
 <div style={{ fontSize: 10, fontWeight: 700, color: "var(--accent-rose)", marginBottom: 8, letterSpacing: "0.05em" }}>SHADES (mixed with black)</div>
 <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
 {shades.map((c, i) => {
 const h = rgbToHex(c);
 return (
 <div key={i} role="button" tabIndex={0} onClick={() => setHex(h)} onKeyDown={e => e.key === "Enter" && setHex(h)} style={{ cursor: "pointer", display: "flex", flexDirection: "column", alignItems: "center", gap: 3 }}>
 <div style={{ width: 40, height: 40, background: h, borderRadius: 6, border: "1px solid var(--border-color)" }} title={h} />
 <span style={{ fontSize: 8, fontFamily: "var(--font-mono)", color: "var(--text-secondary)" }}>{i * 10}%</span>
 </div>
 );
 })}
 </div>
 <div style={{ marginTop: 16, fontSize: 10, color: "var(--text-secondary)", fontStyle: "italic" }}>Click any swatch to set it as the active color.</div>
 </div>
 )}

 {/* ── CONTRAST ── */}
 {subTab === "contrast" && (
 <div style={{ padding: "12px", display: "flex", flexDirection: "column", gap: 12 }}>
 <ContrastBadge ratio={contrastWhite} bg={WHITE} />
 <ContrastBadge ratio={contrastBlack} bg={BLACK} />

 <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
 <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>Custom background:</span>
 <div style={{ position: "relative", width: 28, height: 28 }}>
 <div style={{ width: 28, height: 28, background: bgHex, border: "1px solid var(--border-color)", borderRadius: 4 }} />
 <input type="color" value={bgHex} onChange={e => setBgHex(e.target.value.toUpperCase())}
 style={{ position: "absolute", inset: 0, opacity: 0, cursor: "pointer", width: "100%", height: "100%" }} />
 </div>
 <input value={bgHex} onChange={e => setBgHex(e.target.value)} maxLength={7}
 style={{ width: 90, padding: "3px 6px", fontSize: 11, fontFamily: "var(--font-mono)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none" }} />
 </div>
 <ContrastBadge ratio={contrastCustom} bg={bgRgb} />

 <div style={{ padding: "10px 12px", background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)", fontSize: 11, lineHeight: 1.8, color: "var(--text-secondary)" }}>
 <strong style={{ color: "var(--text-info, #89b4fa)" }}>WCAG 2.1 thresholds:</strong><br/>
 AA normal text: ≥ 4.5:1 · AAA normal text: ≥ 7:1<br/>
 AA large text (18pt / 14pt bold): ≥ 3:1 · AAA large: ≥ 4.5:1
 </div>
 </div>
 )}

 {/* ── CSS SNIPPETS ── */}
 {subTab === "snippets" && (
 <div>
 {cssSnippets.map(({ label, value }) => (
 <FmtRow key={label} label={label} value={value} />
 ))}
 </div>
 )}

 </div>
 </div>
 );
}
