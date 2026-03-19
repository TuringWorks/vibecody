/**
 * CronPanel — Cron Expression Builder & Scheduler.
 *
 * Visual field-by-field cron builder (second optional, minute, hour,
 * day-of-month, month, day-of-week), human-readable description,
 * next-N-runs calculator, and a preset library.
 * Pure TypeScript — no Tauri commands needed.
 */
import { useState, useMemo } from "react";
// lucide-react icons removed — using emoji labels

// ── Types ─────────────────────────────────────────────────────────────────────

interface CronField {
 label: string;
 min: number;
 max: number;
 names?: string[];
}

const FIELDS: CronField[] = [
 { label: "Minute", min: 0, max: 59 },
 { label: "Hour", min: 0, max: 23 },
 { label: "Day (month)", min: 1, max: 31 },
 { label: "Month", min: 1, max: 12, names: ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"] },
 { label: "Day (week)", min: 0, max: 6, names: ["Sun","Mon","Tue","Wed","Thu","Fri","Sat"] },
];

const PRESETS = [
 { label: "Every minute", expr: "* * * * *" },
 { label: "Every 5 minutes", expr: "*/5 * * * *" },
 { label: "Every 15 minutes", expr: "*/15 * * * *" },
 { label: "Every 30 minutes", expr: "*/30 * * * *" },
 { label: "Every hour", expr: "0 * * * *" },
 { label: "Every 6 hours", expr: "0 */6 * * *" },
 { label: "Daily at midnight", expr: "0 0 * * *" },
 { label: "Daily at noon", expr: "0 12 * * *" },
 { label: "Weekdays at 9am", expr: "0 9 * * 1-5" },
 { label: "Weekly (Mon midnight)", expr: "0 0 * * 1" },
 { label: "Monthly (1st midnight)", expr: "0 0 1 * *" },
 { label: "Quarterly", expr: "0 0 1 */3 *" },
 { label: "Yearly (Jan 1 midnight)", expr: "0 0 1 1 *" },
 { label: "Every 2 hours (work hrs)", expr: "0 9-17/2 * * 1-5" },
];

// ── Parser / description ──────────────────────────────────────────────────────

function describeField(val: string, field: CronField): string {
 if (val === "*") return `every ${field.label.toLowerCase()}`;
 if (val.startsWith("*/")) {
 const step = val.slice(2);
 return `every ${step} ${field.label.toLowerCase()}s`;
 }
 if (val.includes("-") && val.includes("/")) {
 const [range, step] = val.split("/");
 return `every ${step} ${field.label.toLowerCase()}s from ${range}`;
 }
 if (val.includes("-")) {
 const [a, b] = val.split("-");
 const na = field.names ? field.names[Number(a)] : a;
 const nb = field.names ? field.names[Number(b)] : b;
 return `from ${na} to ${nb}`;
 }
 if (val.includes(",")) {
 const parts = val.split(",").map(v => field.names ? field.names[Number(v)] ?? v : v);
 return parts.join(", ");
 }
 return field.names ? (field.names[Number(val)] ?? val) : val;
}

function describe(parts: string[]): string {
 if (parts.length !== 5) return "Invalid expression";
 const [min, hr, dom, mon, dow] = parts;
 const bits: string[] = [];
 if (min !== "*" || hr !== "*") {
 bits.push(`at ${describeField(hr, FIELDS[1])} hour, ${describeField(min, FIELDS[0])} minute`);
 } else {
 bits.push(describeField(min, FIELDS[0]));
 }
 if (dom !== "*") bits.push(`on day ${describeField(dom, FIELDS[2])} of the month`);
 if (mon !== "*") bits.push(`in ${describeField(mon, FIELDS[3])}`);
 if (dow !== "*") bits.push(`on ${describeField(dow, FIELDS[4])}`);
 return bits.join(", ").replace(/^./, c => c.toUpperCase());
}

// ── Next-run calculator ───────────────────────────────────────────────────────

function matchesField(val: string, n: number, field: CronField): boolean {
 if (val === "*") return true;
 if (val.startsWith("*/")) {
 const step = Number(val.slice(2));
 return (n - field.min) % step === 0;
 }
 if (val.includes("/")) {
 const [range, stepStr] = val.split("/");
 const step = Number(stepStr);
 const [start, end] = range.includes("-")
 ? range.split("-").map(Number)
 : [Number(range), field.max];
 if (n < start || n > end) return false;
 return (n - start) % step === 0;
 }
 if (val.includes("-")) {
 const [a, b] = val.split("-").map(Number);
 return n >= a && n <= b;
 }
 if (val.includes(",")) {
 return val.split(",").map(Number).includes(n);
 }
 return Number(val) === n;
}

function nextRuns(expr: string, count = 8): Date[] {
 const parts = expr.trim().split(/\s+/);
 if (parts.length !== 5) return [];
 const [minP, hrP, domP, monP, dowP] = parts;
 const results: Date[] = [];
 const now = new Date();
 now.setSeconds(0, 0);
 now.setMinutes(now.getMinutes() + 1); // start from next minute

 const limit = new Date(now.getTime() + 366 * 24 * 60 * 60 * 1000); // search 1 year ahead
 const cur = new Date(now);

 while (results.length < count && cur < limit) {
 const mon = cur.getMonth() + 1; // 1-12
 const dom = cur.getDate(); // 1-31
 const dow = cur.getDay(); // 0-6
 const hr = cur.getHours();
 const min = cur.getMinutes();

 if (
 matchesField(monP, mon, FIELDS[3]) &&
 matchesField(domP, dom, FIELDS[2]) &&
 matchesField(dowP, dow, FIELDS[4]) &&
 matchesField(hrP, hr, FIELDS[1]) &&
 matchesField(minP, min, FIELDS[0])
 ) {
 results.push(new Date(cur));
 }
 cur.setMinutes(cur.getMinutes() + 1);
 }
 return results;
}

// ── Validation ────────────────────────────────────────────────────────────────

function validatePart(val: string, field: CronField): boolean {
 if (val === "*") return true;
 const single = /^\d+$/.test(val) && Number(val) >= field.min && Number(val) <= field.max;
 if (single) return true;
 if (/^\*\/\d+$/.test(val)) return Number(val.split("/")[1]) > 0;
 if (/^\d+-\d+(\/\d+)?$/.test(val)) {
 const [range] = val.split("/");
 const [a, b] = range.split("-").map(Number);
 return a >= field.min && b <= field.max && a <= b;
 }
 if (/^\d+\/\d+$/.test(val)) return true;
 if (/^[\d,]+$/.test(val)) return val.split(",").every(n =>Number(n) >= field.min && Number(n) <= field.max);
 return false;
}

function validate(parts: string[]): string | null {
 if (parts.length !== 5) return "Must have exactly 5 fields";
 for (let i = 0; i < 5; i++) {
 if (!validatePart(parts[i], FIELDS[i])) return `Invalid ${FIELDS[i].label.toLowerCase()} field: "${parts[i]}"`;
 }
 return null;
}

// ── Component ─────────────────────────────────────────────────────────────────

export function CronPanel() {
 const [expr, setExpr] = useState("*/5 * * * *");
 const [copied, setCopied] = useState(false);

 const parts = useMemo(() => expr.trim().split(/\s+/), [expr]);
 const error = useMemo(() => validate(parts), [parts]);
 const desc = useMemo(() => error ? "—" : describe(parts), [parts, error]);
 const runs = useMemo(() => error ? [] : nextRuns(expr), [expr, error]);

 const setPart = (idx: number, val: string) => {
 const next = [...parts];
 next[idx] = val || "*";
 setExpr(next.join(" "));
 };

 const copy = () => {
 navigator.clipboard.writeText(expr);
 setCopied(true);
 setTimeout(() => setCopied(false), 1500);
 };

 return (
 <div style={{ display: "flex", height: "100%", overflow: "hidden" }}>
 {/* Preset sidebar */}
 <div style={{ width: 200, borderRight: "1px solid var(--border-color)", display: "flex", flexDirection: "column", flexShrink: 0 }}>
 <div style={{ padding: "8px 10px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", fontSize: 11, fontWeight: 600 }}>
 Presets
 </div>
 <div style={{ flex: 1, overflowY: "auto" }}>
 {PRESETS.map(p => (
 <button
 key={p.expr}
 onClick={() => setExpr(p.expr)}
 style={{
 display: "block", width: "100%", textAlign: "left",
 padding: "7px 10px", cursor: "pointer",
 background: expr === p.expr ? "var(--accent-bg, rgba(99,102,241,0.15))" : "transparent",
 border: "none", borderBottom: "1px solid var(--border-color)",
 color: "var(--text-primary)",
 }}
 >
 <div style={{ fontSize: 11, fontWeight: expr === p.expr ? 600 : 400 }}>{p.label}</div>
 <div style={{ fontSize: 10, fontFamily: "var(--font-mono)", color: "var(--accent-primary)" }}>{p.expr}</div>
 </button>
 ))}
 </div>
 </div>

 {/* Main */}
 <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden", padding: 16, gap: 16, overflowY: "auto" }}>

 {/* Expression input */}
 <div>
 <label style={{ fontSize: 10, fontWeight: 600, color: "var(--text-muted)", display: "block", marginBottom: 6 }}>CRON EXPRESSION</label>
 <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
 <input
 value={expr}
 onChange={e => setExpr(e.target.value)}
 spellCheck={false}
 style={{
 flex: 1, padding: "8px 12px", fontSize: 16, fontFamily: "var(--font-mono)", fontWeight: 700,
 background: "var(--bg-secondary)", border: `1px solid ${error ? "var(--text-danger)" : "var(--border-color)"}`,
 borderRadius: 6, color: error ? "var(--text-danger)" : "var(--text-info)", outline: "none",
 }}
 />
 <button onClick={copy} style={{ padding: "8px 14px", fontSize: 11, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 6, color: "var(--text-muted)", cursor: "pointer" }}>
 {copied ? "✓" : ""}
 </button>
 </div>
 {error
 ? <div style={{ fontSize: 11, color: "var(--text-danger)", marginTop: 5 }}> {error}</div>
 : <div style={{ fontSize: 11, color: "var(--text-success)", marginTop: 5 }}>✓ {desc}</div>
 }
 </div>

 {/* Field editors */}
 <div style={{ display: "grid", gridTemplateColumns: "repeat(5, 1fr)", gap: 10 }}>
 {FIELDS.map((f, i) => (
 <div key={f.label} style={{ display: "flex", flexDirection: "column", gap: 5 }}>
 <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
 <label style={{ fontSize: 10, fontWeight: 600, color: "var(--text-muted)" }}>{f.label}</label>
 <span style={{ fontSize: 9, color: "var(--text-muted)" }}>{f.min}–{f.max}</span>
 </div>
 <input
 value={parts[i] ?? "*"}
 onChange={e => setPart(i, e.target.value)}
 spellCheck={false}
 style={{
 padding: "5px 8px", fontSize: 13, fontFamily: "var(--font-mono)", fontWeight: 600, textAlign: "center",
 background: "var(--bg-secondary)", border: `1px solid ${parts[i] && !validatePart(parts[i], f) ? "var(--text-danger)" : "var(--border-color)"}`,
 borderRadius: 4, color: "var(--text-primary)", outline: "none",
 }}
 />
 {/* Quick-set chips */}
 <div style={{ display: "flex", gap: 3, flexWrap: "wrap" }}>
 {["*", "*/2", "*/5", "0"].filter(chip => {
 if (chip === "*/5" && f.max < 10) return false;
 return true;
 }).map(chip => (
 <button
 key={chip}
 onClick={() => setPart(i, chip)}
 style={{ padding: "1px 5px", fontSize: 9, borderRadius: 4, background: parts[i] === chip ? "rgba(99,102,241,0.25)" : "var(--bg-primary)", border: `1px solid ${parts[i] === chip ? "var(--accent-primary)" : "var(--border-color)"}`, color: parts[i] === chip ? "var(--text-info)" : "var(--text-muted)", cursor: "pointer" }}
 >
 {chip}
 </button>
 ))}
 {f.names && f.names.slice(0, 3).map((name, ni) => (
 <button
 key={name}
 onClick={() => setPart(i, String(f.min + ni))}
 style={{ padding: "1px 5px", fontSize: 9, borderRadius: 4, background: parts[i] === String(f.min + ni) ? "rgba(99,102,241,0.25)" : "var(--bg-primary)", border: `1px solid ${parts[i] === String(f.min + ni) ? "var(--accent-primary)" : "var(--border-color)"}`, color: parts[i] === String(f.min + ni) ? "var(--text-info)" : "var(--text-muted)", cursor: "pointer" }}
 >
 {name}
 </button>
 ))}
 </div>
 </div>
 ))}
 </div>

 {/* Field reference */}
 <div style={{ padding: "10px 14px", background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)" }}>
 <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 8 }}>Syntax Reference</div>
 <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "4px 20px", fontSize: 11, fontFamily: "var(--font-mono)" }}>
 {[
 ["*", "any value"],
 ["*/n", "every n steps"],
 ["a-b", "range from a to b"],
 ["a,b,c", "list of values"],
 ["a-b/n", "range with step"],
 ["n", "exact value n"],
 ].map(([syn, def]) => (
 <div key={syn} style={{ display: "flex", gap: 8 }}>
 <span style={{ color: "var(--accent-primary)", minWidth: 60 }}>{syn}</span>
 <span style={{ color: "var(--text-muted)" }}>{def}</span>
 </div>
 ))}
 </div>
 </div>

 {/* Next runs */}
 {runs.length > 0 && (
 <div>
 <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 8 }}>Next {runs.length} Scheduled Runs</div>
 <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
 {runs.map((d, i) => (
 <div key={i} style={{ display: "flex", gap: 12, padding: "5px 10px", background: i === 0 ? "rgba(99,102,241,0.1)" : "var(--bg-secondary)", borderRadius: 4, border: `1px solid ${i === 0 ? "var(--accent-primary)" : "var(--border-color)"}`, fontSize: 12, fontFamily: "var(--font-mono)" }}>
 <span style={{ color: "var(--text-muted)", minWidth: 20 }}>#{i + 1}</span>
 <span style={{ color: i === 0 ? "var(--text-info)" : "var(--text-primary)", fontWeight: i === 0 ? 600 : 400 }}>
 {d.toLocaleString([], { weekday: "short", year: "numeric", month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" })}
 </span>
 {i === 0 && (
 <span style={{ color: "var(--text-success)", fontSize: 10, marginLeft: "auto" }}>
 in {Math.round((d.getTime() - Date.now()) / 60000)} min
 </span>
 )}
 </div>
 ))}
 </div>
 </div>
 )}
 </div>
 </div>
 );
}
