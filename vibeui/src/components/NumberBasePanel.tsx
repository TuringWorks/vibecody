/**
 * NumberBasePanel — Number base converter + bitwise explorer.
 *
 * • Enter a number in any base (2/8/10/16) — all others update live.
 * • Supports 8/16/32/64-bit signed & unsigned integer interpretation.
 * • Bit-field visualizer: click individual bits to toggle.
 * • Bitwise operations: AND, OR, XOR, NOT, shifts on two operands.
 * • IEEE 754 float32 breakdown (sign, exponent, mantissa).
 *
 * Pure TypeScript — no Tauri commands required.
 */
import { useState, useMemo, useCallback } from "react";
// lucide-react icons not needed

// ── Constants ──────────────────────────────────────────────────────────────────

const BIT_WIDTHS = [8, 16, 32, 64] as const;
type BitWidth = typeof BIT_WIDTHS[number];

// ── Safe BigInt arithmetic ─────────────────────────────────────────────────────

function parseBigInt(s: string, base: number): bigint | null {
 const clean = s.trim().replace(/^0[xX]/, "").replace(/^0[oO]/, "").replace(/^0[bB]/, "").replace(/[\s_]/g, "");
 if (!clean) return null;
 try { return BigInt(base === 16 ? "0x" + clean : base === 8 ? "0o" + clean : base === 2 ? "0b" + clean : clean); }
 catch { return null; }
}

function toSigned(val: bigint, bits: BitWidth): bigint {
 const mask = (1n << BigInt(bits)) - 1n;
 const masked = val & mask;
 const sign = 1n << BigInt(bits - 1);
 return masked >= sign ? masked - (1n << BigInt(bits)) : masked;
}

function toUnsigned(val: bigint, bits: BitWidth): bigint {
 return val & ((1n << BigInt(bits)) - 1n);
}

// ── IEEE 754 float32 ───────────────────────────────────────────────────────────

function parseFloat32(val: bigint) {
 const bits32 = Number(val & 0xFFFFFFFFn);
 const buf = new ArrayBuffer(4);
 new DataView(buf).setUint32(0, bits32, false);
 const float = new DataView(buf).getFloat32(0, false);
 const sign = (bits32 >>> 31) & 1;
 const exp = (bits32 >>> 23) & 0xFF;
 const mant = bits32 & 0x7FFFFF;
 return { float, sign, exp: exp - 127, rawExp: exp, mant, isNaN: isNaN(float), isInf: !isFinite(float) && !isNaN(float) };
}

// ── Bit display ────────────────────────────────────────────────────────────────

function BitGrid({ val, bits, onToggle }: { val: bigint; bits: BitWidth; onToggle: (i: number) => void }) {
 const groups = bits === 8 ? 1 : bits === 16 ? 2 : bits === 32 ? 4 : 8;
 const groupSize = bits / groups;
 return (
 <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
 {Array.from({ length: groups }).map((_, gi) => (
 <div key={gi} style={{ display: "flex", gap: 1 }}>
 {Array.from({ length: groupSize }).map((_, bi) => {
 const bitIdx = bits - 1 - (gi * groupSize + bi);
 const isSet = ((val >>BigInt(bitIdx)) & 1n) === 1n;
 const isMsb = bitIdx === bits - 1;
 return (
 <button key={bi} onClick={() => onToggle(bitIdx)}
 title={`Bit ${bitIdx}`}
 style={{
 width: bits > 32 ? 12 : 16, height: bits > 32 ? 18 : 22,
 display: "flex", alignItems: "center", justifyContent: "center",
 fontSize: bits > 32 ? 8 : 10, fontWeight: 700, fontFamily: "var(--font-mono)",
 background: isSet ? (isMsb ? "rgba(243,139,168,0.3)" : "color-mix(in srgb, var(--accent-blue) 25%, transparent)") : "var(--bg-secondary)",
 border: `1px solid ${isSet ? (isMsb ? "var(--error-color)" : "var(--accent-color)") : "var(--border-color)"}`,
 borderRadius: 2, color: isSet ? (isMsb ? "var(--error-color)" : "var(--accent-color)") : "var(--text-secondary)",
 cursor: "pointer", padding: 0,
 }}>
 {isSet ? "1" : "0"}
 </button>
 );
 })}
 </div>
 ))}
 </div>
 );
}

// ── Input field ────────────────────────────────────────────────────────────────

function BaseInput({ label, prefix, value, onChange, valid, mono = true }: {
 label: string; prefix: string; value: string; onChange: (s: string) => void; valid: boolean; mono?: boolean;
}) {
 return (
 <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
 <span style={{ width: 60, fontSize: 10, fontWeight: 700, color: "var(--text-secondary)", flexShrink: 0 }}>{label}</span>
 <span style={{ fontSize: 11, color: "var(--text-secondary)", fontFamily: "var(--font-mono)", flexShrink: 0 }}>{prefix}</span>
 <input value={value} onChange={e => onChange(e.target.value)} spellCheck={false}
 style={{ flex: 1, padding: "4px 8px", fontSize: 12, fontFamily: mono ? "var(--font-mono)" : "inherit", background: !valid && value ? "color-mix(in srgb, var(--accent-rose) 8%, transparent)" : "var(--bg-primary)", border: `1px solid ${!valid && value ? "var(--error-color)" : "var(--border-color)"}`, borderRadius: 4, color: "var(--text-primary)", outline: "none" }} />
 </div>
 );
}

// ── Component ──────────────────────────────────────────────────────────────────

type SubTab = "convert" | "bitwise" | "float32";

export function NumberBasePanel() {
 const [subTab, setSubTab] = useState<SubTab>("convert");
 const [bits, setBits] = useState<BitWidth>(32);
 const [signed, setSigned] = useState(true);

 // Inputs per base
 const [decInput, setDecInput] = useState("255");
 const [hexInput, setHexInput] = useState("");
 const [octInput, setOctInput] = useState("");
 const [binInput, setBinInput] = useState("");

 // Bitwise operands
 const [opA, setOpA] = useState("0b10110011");
 const [opB, setOpB] = useState("0b01101101");

 // ── Canonical value from whichever field changed last ──────────────────────

 const [source, setSource] = useState<"dec" | "hex" | "oct" | "bin">("dec");

 const canonical: bigint | null = useMemo(() => {
 const raw = source === "dec" ? decInput : source === "hex" ? hexInput : source === "oct" ? octInput : binInput;
 const base = source === "dec" ? 10 : source === "hex" ? 16 : source === "oct" ? 8 : 2;
 const v = parseBigInt(raw, base);
 if (v === null) return null;
 return signed ? toSigned(v, bits) : toUnsigned(v, bits);
 }, [source, decInput, hexInput, octInput, binInput, bits, signed]);

 const mask = (1n << BigInt(bits)) - 1n;
 const uVal = canonical !== null ? canonical & mask : 0n;

 // Sync all fields from canonical
 const syncFrom = useCallback((val: bigint | null, src: "dec" | "hex" | "oct" | "bin") => {
 if (val === null) return;
 const u = val & mask;
 if (src !== "dec") setDecInput(signed ? toSigned(val, bits).toString(10) : u.toString(10));
 if (src !== "hex") setHexInput(u.toString(16).toUpperCase());
 if (src !== "oct") setOctInput(u.toString(8));
 if (src !== "bin") setBinInput(u.toString(2).padStart(bits, "0").replace(/(.{4})/g, "$1 ").trim());
 // eslint-disable-next-line react-hooks/exhaustive-deps
 }, [bits, signed, mask]);

 const handleDec = (s: string) => { setDecInput(s); setSource("dec"); const v = parseBigInt(s, 10); syncFrom(v, "dec"); };
 const handleHex = (s: string) => { setHexInput(s); setSource("hex"); const v = parseBigInt(s, 16); syncFrom(v, "hex"); };
 const handleOct = (s: string) => { setOctInput(s); setSource("oct"); const v = parseBigInt(s, 8); syncFrom(v, "oct"); };
 const handleBin = (s: string) => { setBinInput(s); setSource("bin"); const v = parseBigInt(s.replace(/\s/g,""), 2); syncFrom(v, "bin"); };

 // Bit toggle
 const toggleBit = (i: number) => {
 const toggled = uVal ^ (1n << BigInt(i));
 const val = signed ? toSigned(toggled, bits) : toUnsigned(toggled, bits);
 setDecInput(val.toString(10));
 setHexInput((toggled & mask).toString(16).toUpperCase());
 setOctInput((toggled & mask).toString(8));
 setBinInput((toggled & mask).toString(2).padStart(bits, "0").replace(/(.{4})/g, "$1 ").trim());
 setSource("dec");
 };

 // Bitwise ops
 const aVal = useMemo(() => {
 const s = opA.replace(/\s/g, "");
 if (s.startsWith("0b") || s.startsWith("0B")) return parseBigInt(s.slice(2), 2);
 if (s.startsWith("0x") || s.startsWith("0X")) return parseBigInt(s.slice(2), 16);
 return parseBigInt(s, 10);
 }, [opA]);

 const bVal = useMemo(() => {
 const s = opB.replace(/\s/g, "");
 if (s.startsWith("0b") || s.startsWith("0B")) return parseBigInt(s.slice(2), 2);
 if (s.startsWith("0x") || s.startsWith("0X")) return parseBigInt(s.slice(2), 16);
 return parseBigInt(s, 10);
 }, [opB]);

 const bitwiseOps = useMemo(() => {
 if (aVal === null || bVal === null) return null;
 const a = aVal & mask, b = bVal & mask;
 return {
 AND: a & b,
 OR: a | b,
 XOR: a ^ b,
 NOT_A: (~a) & mask,
 SHL1: (a << 1n) & mask,
 SHR1: a >> 1n,
 SHL4: (a << 4n) & mask,
 SHR4: a >> 4n,
 };
 }, [aVal, bVal, mask]);

 // Float32
 const f32 = useMemo(() => canonical !== null ? parseFloat32(canonical) : null, [canonical]);

 // ── Render ────────────────────────────────────────────────────────────────

 const validDec = decInput === "" || parseBigInt(decInput, 10) !== null;
 const validHex = hexInput === "" || parseBigInt(hexInput, 16) !== null;
 const validOct = octInput === "" || parseBigInt(octInput, 8) !== null;
 const validBin = binInput === "" || parseBigInt(binInput.replace(/\s/g,""), 2) !== null;

 return (
 <div style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, overflow: "hidden" }}>

 {/* Header */}
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
 <span style={{ fontSize: 13, fontWeight: 600 }}>Number Bases</span>
 {(["convert","bitwise","float32"] as SubTab[]).map(t => (
 <button key={t} onClick={() => setSubTab(t)} style={{ padding: "2px 10px", fontSize: 10, borderRadius: 10, background: subTab === t ? "color-mix(in srgb, var(--accent-blue) 20%, transparent)" : "var(--bg-primary)", border: `1px solid ${subTab === t ? "var(--accent-color)" : "var(--border-color)"}`, color: subTab === t ? "var(--info-color)" : "var(--text-secondary)", cursor: "pointer", fontWeight: subTab === t ? 700 : 400 }}>
 {t === "convert" ? "Convert" : t === "bitwise" ? "Bitwise" : "Float32"}
 </button>
 ))}
 <div style={{ marginLeft: "auto", display: "flex", gap: 6, alignItems: "center" }}>
 {/* Bit width */}
 {BIT_WIDTHS.map(w => (
 <button key={w} onClick={() => setBits(w)} style={{ padding: "2px 8px", fontSize: 10, borderRadius: 4, background: bits === w ? "color-mix(in srgb, var(--accent-blue) 20%, transparent)" : "var(--bg-primary)", border: `1px solid ${bits === w ? "var(--accent-color)" : "var(--border-color)"}`, color: bits === w ? "var(--info-color)" : "var(--text-secondary)", cursor: "pointer" }}>{w}-bit</button>
 ))}
 <label style={{ fontSize: 10, color: "var(--text-secondary)", display: "flex", gap: 4, alignItems: "center", cursor: "pointer" }}>
 <input type="checkbox" checked={signed} onChange={e => setSigned(e.target.checked)} style={{ accentColor: "var(--accent-color)" }} />
 signed
 </label>
 </div>
 </div>

 <div style={{ flex: 1, overflow: "auto", padding: "12px" }}>

 {/* ── CONVERT ── */}
 {subTab === "convert" && (
 <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
 <BaseInput label="Decimal" prefix="d" value={decInput} onChange={handleDec} valid={validDec} />
 <BaseInput label="Hexadecimal" prefix="0x" value={hexInput} onChange={handleHex} valid={validHex} />
 <BaseInput label="Octal" prefix="0o" value={octInput} onChange={handleOct} valid={validOct} />
 <BaseInput label="Binary" prefix="0b" value={binInput} onChange={handleBin} valid={validBin} />

 {canonical !== null && (
 <div style={{ marginTop: 12 }}>
 <div style={{ fontSize: 10, fontWeight: 700, color: "var(--text-secondary)", marginBottom: 8, letterSpacing: "0.05em" }}>BIT FIELD ({bits}-bit, {signed ? "signed" : "unsigned"})</div>
 <BitGrid val={uVal} bits={bits} onToggle={toggleBit} />
 {bits <= 32 && (
 <div style={{ display: "flex", gap: 4, marginTop: 4, flexWrap: "wrap" }}>
 {Array.from({ length: bits }).map((_, i) => {
 const idx = bits - 1 - i;
 return <span key={i} style={{ fontSize: 7, color: "var(--text-secondary)", width: bits > 16 ? 16 : 22, textAlign: "center", fontFamily: "var(--font-mono)" }}>{idx}</span>;
 })}
 </div>
 )}
 </div>
 )}

 {/* Integer range info */}
 <div style={{ marginTop: 8, padding: "8px 12px", background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)", fontSize: 11, lineHeight: 1.8 }}>
 <div style={{ display: "flex", gap: 16, flexWrap: "wrap" }}>
 <span style={{ color: "var(--text-secondary)" }}>Min: <span style={{ fontFamily: "var(--font-mono)", color: "var(--text-danger)" }}>{signed ? (-(1n << BigInt(bits - 1))).toString() : "0"}</span></span>
 <span style={{ color: "var(--text-secondary)" }}>Max: <span style={{ fontFamily: "var(--font-mono)", color: "var(--text-success)" }}>{signed ? ((1n << BigInt(bits - 1)) - 1n).toString() : ((1n << BigInt(bits)) - 1n).toString()}</span></span>
 {canonical !== null && <span style={{ color: "var(--text-secondary)" }}>Value: <span style={{ fontFamily: "var(--font-mono)", color: "var(--text-info)" }}>{canonical.toString()}</span></span>}
 </div>
 </div>
 </div>
 )}

 {/* ── BITWISE ── */}
 {subTab === "bitwise" && (
 <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
 <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
 <BaseInput label="Operand A" prefix="" value={opA} onChange={setOpA} valid={aVal !== null} />
 <BaseInput label="Operand B" prefix="" value={opB} onChange={setOpB} valid={bVal !== null} />
 <div style={{ fontSize: 10, color: "var(--text-secondary)", marginLeft: 68 }}>Supports 0b…, 0x…, or decimal. Operations use {bits}-bit unsigned.</div>
 </div>
 {bitwiseOps !== null ? (
 <div style={{ marginTop: 4 }}>
 {([
 ["A AND B", bitwiseOps.AND, "var(--success-color)"],
 ["A OR B", bitwiseOps.OR, "var(--accent-color)"],
 ["A XOR B", bitwiseOps.XOR, "var(--text-accent)"],
 ["NOT A", bitwiseOps.NOT_A,"var(--error-color)"],
 ["A << 1", bitwiseOps.SHL1, "var(--warning-color)"],
 ["A >> 1", bitwiseOps.SHR1, "var(--warning-color)"],
 ["A << 4", bitwiseOps.SHL4, "var(--warning-color)"],
 ["A >> 4", bitwiseOps.SHR4, "var(--warning-color)"],
 ] as [string, bigint, string][]).map(([label, result, colour]) => (
 <div key={label} style={{ borderBottom: "1px solid var(--border-color)", padding: "5px 0", display: "flex", gap: 10, alignItems: "center" }}>
 <span style={{ width: 80, fontSize: 11, fontWeight: 700, color: "var(--text-secondary)", flexShrink: 0, fontFamily: "var(--font-mono)" }}>{label}</span>
 <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: colour, flex: 1 }}>0x{result.toString(16).toUpperCase().padStart(bits / 4, "0")}</span>
 <span style={{ fontFamily: "var(--font-mono)", fontSize: 10, color: "var(--text-secondary)" }}>{result.toString(10)}</span>
 <button onClick={() => { setDecInput(result.toString(10)); setSource("dec"); syncFrom(result, "dec"); setSubTab("convert"); }} style={{ fontSize: 9, padding: "1px 6px", background: "none", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-secondary)", cursor: "pointer" }}>→</button>
 </div>
 ))}
 </div>
 ) : (
 <div style={{ color: "var(--text-secondary)", fontSize: 12 }}>Enter valid operands above.</div>
 )}
 </div>
 )}

 {/* ── FLOAT32 ── */}
 {subTab === "float32" && (
 <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
 <div style={{ fontSize: 11, color: "var(--text-secondary)", lineHeight: 1.6 }}>
 Interprets the {bits}-bit value as an IEEE 754 single-precision float (uses lower 32 bits).
 </div>
 <BaseInput label="Decimal" prefix="d" value={decInput} onChange={handleDec} valid={validDec} />
 <BaseInput label="Hex" prefix="0x" value={hexInput} onChange={handleHex} valid={validHex} />

 {f32 !== null && (
 <div style={{ display: "flex", flexDirection: "column", gap: 8, marginTop: 4 }}>
 {/* Float value */}
 <div style={{ padding: "10px 12px", background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)", textAlign: "center" }}>
 <div style={{ fontSize: 24, fontWeight: 700, fontFamily: "var(--font-mono)", color: f32.isNaN ? "var(--error-color)" : f32.isInf ? "var(--warning-color)" : "var(--accent-color)" }}>
 {f32.isNaN ? "NaN" : f32.isInf ? (f32.sign ? "-∞" : "+∞") : f32.float.toPrecision(8)}
 </div>
 <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 4 }}>float32 value</div>
 </div>
 {/* Breakdown */}
 <div style={{ display: "flex", gap: 8 }}>
 {[
 { label: "Sign", bits2: 1, value: f32.sign, colour: "var(--error-color)", note: f32.sign ? "negative" : "positive" },
 { label: "Exponent", bits2: 8, value: f32.rawExp, colour: "var(--warning-color)", note: `2^${f32.exp} (biased ${f32.rawExp})` },
 { label: "Mantissa", bits2: 23, value: f32.mant, colour: "var(--success-color)", note: `0x${f32.mant.toString(16).toUpperCase().padStart(6, "0")}` },
 ].map(({ label, bits2, value, colour, note }) => (
 <div key={label} style={{ flex: bits2, padding: "8px", background: "var(--bg-secondary)", border: `1px solid ${colour}`, borderRadius: 6, minWidth: 0 }}>
 <div style={{ fontSize: 9, fontWeight: 700, color: colour, marginBottom: 2 }}>{label} ({bits2}b)</div>
 <div style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--text-primary)", wordBreak: "break-all" }}>
 {value.toString(2).padStart(bits2, "0")}
 </div>
 <div style={{ fontSize: 9, color: "var(--text-secondary)", marginTop: 2 }}>{note}</div>
 </div>
 ))}
 </div>
 {/* 32-bit view */}
 <div>
 <div style={{ fontSize: 10, fontWeight: 700, color: "var(--text-secondary)", marginBottom: 6 }}>32-BIT LAYOUT</div>
 <BitGrid val={BigInt(f32.sign) << 31n | BigInt(f32.rawExp) << 23n | BigInt(f32.mant)} bits={32} onToggle={i => { const toggled = (BigInt(f32.sign) << 31n | BigInt(f32.rawExp) << 23n | BigInt(f32.mant)) ^ (1n << BigInt(i)); setHexInput(toggled.toString(16).toUpperCase()); setSource("hex"); syncFrom(toggled, "hex"); }} />
 </div>
 </div>
 )}
 </div>
 )}

 </div>
 </div>
 );
}
