/**
 * TimestampPanel — Timestamp & date utilities.
 *
 * Tabs:
 * Convert : Unix timestamp (sec/ms) ↔ formatted date across timezones.
 * Duration : compute the interval between two dates (days, hours, etc.).
 * Relative : "how long ago / until" + calendar offsets.
 * Formats : reference table of common date-format tokens.
 *
 * Pure TypeScript + Intl / Date — no Tauri commands required.
 */
import { useState, useMemo, useCallback, useEffect } from "react";
import { CopyButton as CopyBtn } from "./shared/CopyButton";

// ── Timezone list ──────────────────────────────────────────────────────────────

const TIMEZONES = [
 "UTC",
 "America/New_York",
 "America/Chicago",
 "America/Denver",
 "America/Los_Angeles",
 "America/Sao_Paulo",
 "Europe/London",
 "Europe/Paris",
 "Europe/Berlin",
 "Europe/Moscow",
 "Africa/Cairo",
 "Asia/Dubai",
 "Asia/Kolkata",
 "Asia/Bangkok",
 "Asia/Singapore",
 "Asia/Shanghai",
 "Asia/Tokyo",
 "Asia/Seoul",
 "Australia/Sydney",
 "Pacific/Auckland",
];

// ── Formatters ────────────────────────────────────────────────────────────────

function fmt(date: Date, tz: string, opts: Intl.DateTimeFormatOptions): string {
 try { return new Intl.DateTimeFormat("en-US", { ...opts, timeZone: tz }).format(date); }
 catch { return "—"; }
}

function iso8601(date: Date): string {
 return date.toISOString();
}

function rfc2822(date: Date): string {
 try { return new Intl.DateTimeFormat("en-US", { weekday: "short", day: "2-digit", month: "short", year: "numeric", hour: "2-digit", minute: "2-digit", second: "2-digit", timeZoneName: "short", hour12: false }).format(date).replace(",", ""); }
 catch { return date.toString(); }
}

function relativeTime(date: Date, now = new Date()): string {
 const diff = date.getTime() - now.getTime();
 const abs = Math.abs(diff);
 const secs = Math.floor(abs / 1000);
 const mins = Math.floor(secs / 60);
 const hrs = Math.floor(mins / 60);
 const days = Math.floor(hrs / 24);
 const wks = Math.floor(days / 7);
 const mos = Math.floor(days / 30.44);
 const yrs = Math.floor(days / 365.25);

 let label = "";
 if (secs < 5) label = "just now";
 else if (secs < 60) label = `${secs} second${secs !== 1 ? "s" : ""}`;
 else if (mins < 60) label = `${mins} minute${mins !== 1 ? "s" : ""}`;
 else if (hrs < 24) label = `${hrs} hour${hrs !== 1 ? "s" : ""}`;
 else if (days < 7) label = `${days} day${days !== 1 ? "s" : ""}`;
 else if (wks < 5) label = `${wks} week${wks !== 1 ? "s" : ""}`;
 else if (mos < 12) label = `${mos} month${mos !== 1 ? "s" : ""}`;
 else label = `${yrs} year${yrs !== 1 ? "s" : ""}`;

 if (label === "just now") return label;
 return diff < 0 ? `${label} ago` : `in ${label}`;
}

// ── Duration breakdown ────────────────────────────────────────────────────────

interface Duration {
 totalMs: number; totalSecs: number; totalMins: number; totalHours: number; totalDays: number;
 years: number; months: number; days: number; hours: number; minutes: number; seconds: number; ms: number;
}

function calcDuration(a: Date, b: Date): Duration {
 const totalMs = Math.abs(b.getTime() - a.getTime());
 const totalSecs = Math.floor(totalMs / 1000);
 const totalMins = Math.floor(totalSecs / 60);
 const totalHours = Math.floor(totalMins / 60);
 const totalDays = Math.floor(totalHours / 24);

 let years = 0, months = 0;
 const [start, end] = a <= b ? [a, b] : [b, a];
 years = end.getFullYear() - start.getFullYear();
 months = end.getMonth() - start.getMonth();
 if (months < 0) { years--; months += 12; }
 const dayDiff = end.getDate() - start.getDate();
 if (dayDiff < 0) { months--; if (months < 0) { years--; months += 12; } }
 const days = Math.floor((totalMs % (1000 * 60 * 60 * 24 * 30.44)) / (1000 * 60 * 60 * 24));
 const hours = Math.floor(totalMs / (1000 * 60 * 60)) % 24;
 const minutes = Math.floor(totalMs / (1000 * 60)) % 60;
 const seconds = Math.floor(totalMs / 1000) % 60;
 const ms = totalMs % 1000;
 return { totalMs, totalSecs, totalMins, totalHours, totalDays, years, months, days, hours, minutes, seconds, ms };
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function toLocalInputValue(date: Date): string {
 const pad = (n: number) =>String(n).padStart(2, "0");
 return `${date.getFullYear()}-${pad(date.getMonth()+1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function parseTimestampInput(s: string): Date | null {
 const trimmed = s.trim();
 if (!trimmed) return null;
 const n = Number(trimmed);
 if (!isNaN(n)) {
 // Auto-detect ms vs seconds: if > 1e12, treat as ms
 const ms = n > 1e12 ? n : n * 1000;
 const d = new Date(ms);
 return isNaN(d.getTime()) ? null : d;
 }
 const d = new Date(trimmed);
 return isNaN(d.getTime()) ? null : d;
}

type SubTab = "convert" | "duration" | "relative" | "formats";

// ── CopyButton ────────────────────────────────────────────────────────────────

// CopyBtn imported from shared/CopyButton.tsx

// ── Component ──────────────────────────────────────────────────────────────────

export function TimestampPanel() {
 const [subTab, setSubTab] = useState<SubTab>("convert");
 const now = new Date();

 // ── Convert tab ─────────────────────────────────────────────────────────────
 const [tsInput, setTsInput] = useState(String(Math.floor(now.getTime() / 1000)));
 const [tz, setTz] = useState("UTC");
 const [localTz] = useState(() =>Intl.DateTimeFormat().resolvedOptions().timeZone);
 const [tick, setTick] = useState(0);

 // Live clock update
 useEffect(() => {
 const id = setInterval(() => setTick(t => t + 1), 1000);
 return () => clearInterval(id);
 }, []);

 const parsedDate = useMemo(() => parseTimestampInput(tsInput), [tsInput]);

 const fmtRows = useMemo(() => {
 if (!parsedDate) return [];
 return [
 { label: "ISO 8601", value: iso8601(parsedDate) },
 { label: "RFC 2822", value: rfc2822(parsedDate) },
 { label: "Relative", value: relativeTime(parsedDate) },
 { label: "Unix (seconds)", value: String(Math.floor(parsedDate.getTime() / 1000)) },
 { label: "Unix (ms)", value: String(parsedDate.getTime()) },
 { label: "Locale (long)", value: fmt(parsedDate, tz, { dateStyle: "full", timeStyle: "long" }) },
 { label: "Locale (short)", value: fmt(parsedDate, tz, { dateStyle: "short", timeStyle: "short" }) },
 { label: "Date only", value: fmt(parsedDate, tz, { dateStyle: "long" }) },
 { label: "Time only", value: fmt(parsedDate, tz, { timeStyle: "medium" }) },
 { label: "Weekday", value: fmt(parsedDate, tz, { weekday: "long" }) },
 { label: "Week of year", value: (() => { const d = new Date(Date.UTC(parsedDate.getFullYear(),parsedDate.getMonth(),parsedDate.getDate())); d.setUTCDate(d.getUTCDate()+4-(d.getUTCDay()||7)); const y=new Date(Date.UTC(d.getUTCFullYear(),0,1)); return `W${String(Math.ceil((((d.getTime()-y.getTime())/86400000)+1)/7)).padStart(2,"0")}`; })() },
 ];
 }, [parsedDate, tz, tick]); // eslint-disable-line react-hooks/exhaustive-deps

 const tzRows = useMemo(() => {
 if (!parsedDate) return [];
 return [
 { tz: "UTC", label: "UTC" },
 { tz: localTz, label: `Local (${localTz})` },
 { tz: "America/New_York", label: "New York (ET)" },
 { tz: "America/Los_Angeles",label: "Los Angeles (PT)" },
 { tz: "Europe/London", label: "London (GMT/BST)" },
 { tz: "Europe/Paris", label: "Paris (CET)" },
 { tz: "Asia/Kolkata", label: "India (IST)" },
 { tz: "Asia/Tokyo", label: "Tokyo (JST)" },
 { tz: "Australia/Sydney", label: "Sydney (AEST)" },
 ].map(r => ({
 ...r,
 value: fmt(parsedDate, r.tz, { dateStyle: "medium", timeStyle: "short" }),
 }));
 }, [parsedDate, localTz, tick]); // eslint-disable-line react-hooks/exhaustive-deps

 const setNow = useCallback(() => setTsInput(String(Math.floor(Date.now() / 1000))), []);

 // ── Duration tab ────────────────────────────────────────────────────────────
 const [durA, setDurA] = useState(toLocalInputValue(new Date(Date.now() - 86400000 * 30)));
 const [durB, setDurB] = useState(toLocalInputValue(new Date()));

 const duration = useMemo(() => {
 const a = new Date(durA), b = new Date(durB);
 if (isNaN(a.getTime()) || isNaN(b.getTime())) return null;
 return calcDuration(a, b);
 }, [durA, durB]);

 // ── Relative tab ────────────────────────────────────────────────────────────
 const [relBase, setRelBase] = useState(toLocalInputValue(new Date()));
 const relDate = useMemo(() => { const d = new Date(relBase); return isNaN(d.getTime()) ? null : d; }, [relBase]);
 const offsets = [-365, -90, -30, -14, -7, -1, 0, 1, 7, 14, 30, 90, 180, 365];

 return (
 <div className="panel-container">

 {/* Header */}
 <div className="panel-tab-bar" style={{ padding: "8px 12px", alignItems: "center", flexWrap: "wrap" }}>
 <span style={{ fontSize: 13, fontWeight: 600, marginRight: 6 }}>Timestamp</span>
 {(["convert","duration","relative","formats"] as SubTab[]).map(t => (
 <button key={t} onClick={() => setSubTab(t)} className={`panel-tab${subTab === t ? " active" : ""}`}>
 {t.charAt(0).toUpperCase() + t.slice(1)}
 </button>
 ))}
 <div style={{ marginLeft: "auto", fontSize: 10, fontFamily: "var(--font-mono)", color: "var(--text-secondary)" }}>
 now: {Math.floor(Date.now() / 1000)}
 </div>
 </div>

 <div className="panel-body" style={{ overflow: "auto" }}>

 {/* ── CONVERT ── */}
 {subTab === "convert" && (
 <div style={{ display: "flex", flexDirection: "column" }}>
 {/* Input */}
 <div style={{ padding: "10px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
 <input value={tsInput} onChange={e => setTsInput(e.target.value)} placeholder="Unix timestamp or ISO date string…"
 style={{ flex: 1, minWidth: 200, padding: "5px 10px", fontSize: 12, fontFamily: "var(--font-mono)", background: !parsedDate && tsInput ? "color-mix(in srgb, var(--accent-rose) 8%, transparent)" : "var(--bg-primary)", border: `1px solid ${!parsedDate && tsInput ? "var(--text-danger)" : "var(--border-color)"}`, borderRadius: 4, color: "var(--text-primary)", outline: "none" }} />
 <button onClick={setNow} className="panel-btn panel-btn-secondary panel-btn-xs" style={{ color: "var(--text-info)" }}>Now</button>
 <select value={tz} onChange={e => setTz(e.target.value)} className="panel-select">
 {TIMEZONES.map(z => <option key={z} value={z}>{z}</option>)}
 </select>
 </div>

 {!parsedDate && tsInput.trim() && (
 <div style={{ padding: "8px 12px", fontSize: 11, color: "var(--text-danger)" }}>Could not parse input. Try a Unix timestamp (e.g. 1700000000) or ISO date (e.g. 2024-01-15T12:00:00Z).</div>
 )}

 {/* Format table */}
 {parsedDate && (
 <div>
 {fmtRows.map(({ label, value }) => (
 <div key={label} style={{ display: "flex", alignItems: "center", borderBottom: "1px solid var(--border-color)", padding: "5px 12px", gap: 10 }}>
 <span style={{ width: 140, flexShrink: 0, fontSize: 10, fontWeight: 700, color: "var(--text-secondary)" }}>{label}</span>
 <span style={{ flex: 1, fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--text-primary)", wordBreak: "break-all" }}>{value}</span>
 <CopyBtn text={value} />
 </div>
 ))}

 {/* Timezone grid */}
 <div style={{ padding: "6px 12px 0", fontSize: 10, fontWeight: 700, color: "var(--text-secondary)", background: "var(--bg-secondary)", borderTop: "1px solid var(--border-color)", marginTop: 8, letterSpacing: "0.05em" }}>WORLD CLOCKS</div>
 <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(260px, 1fr))", gap: 0 }}>
 {tzRows.map(({ tz: z, label, value }) => (
 <div key={z} style={{ padding: "5px 12px", borderBottom: "1px solid var(--border-color)", display: "flex", gap: 8, alignItems: "center" }}>
 <span style={{ flex: 1, fontSize: 10, color: "var(--text-secondary)" }}>{label}</span>
 <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--text-info)" }}>{value}</span>
 </div>
 ))}
 </div>
 </div>
 )}
 </div>
 )}

 {/* ── DURATION ── */}
 {subTab === "duration" && (
 <div style={{ padding: "12px", display: "flex", flexDirection: "column", gap: 12 }}>
 <div style={{ display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
 <div style={{ display: "flex", flexDirection: "column", gap: 4, flex: 1 }}>
 <label style={{ fontSize: 10, fontWeight: 700, color: "var(--text-secondary)" }}>START</label>
 <input type="datetime-local" value={durA} onChange={e => setDurA(e.target.value)}
 style={{ padding: "5px 8px", fontSize: 12, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none" }} />
 </div>
 <span style={{ fontSize: 18, color: "var(--text-secondary)", paddingTop: 16 }}>→</span>
 <div style={{ display: "flex", flexDirection: "column", gap: 4, flex: 1 }}>
 <label style={{ fontSize: 10, fontWeight: 700, color: "var(--text-secondary)" }}>END</label>
 <input type="datetime-local" value={durB} onChange={e => setDurB(e.target.value)}
 style={{ padding: "5px 8px", fontSize: 12, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none" }} />
 </div>
 <button onClick={() => { setDurA(toLocalInputValue(new Date())); setDurB(toLocalInputValue(new Date())); }} style={{ paddingTop: 16, fontSize: 10, background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer" }}>Reset</button>
 </div>

 {duration && (
 <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
 {/* Human breakdown */}
 <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(100px, 1fr))", gap: 8 }}>
 {[
 { label: "Years", value: duration.years, colour: "var(--text-info)" },
 { label: "Months", value: duration.months, colour: "var(--text-accent)" },
 { label: "Days", value: duration.days, colour: "var(--text-success)" },
 { label: "Hours", value: duration.hours, colour: "var(--text-warning-alt)" },
 { label: "Minutes", value: duration.minutes, colour: "var(--text-warning)" },
 { label: "Seconds", value: duration.seconds, colour: "var(--text-danger)" },
 ].map(({ label, value, colour }) => (
 <div key={label} style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 6, padding: "8px", textAlign: "center" }}>
 <div style={{ fontSize: 22, fontWeight: 700, fontFamily: "var(--font-mono)", color: colour }}>{value}</div>
 <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 2 }}>{label}</div>
 </div>
 ))}
 </div>
 {/* Totals */}
 <div style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 6, padding: "10px 12px" }}>
 {[
 ["Total days", duration.totalDays.toLocaleString()],
 ["Total hours", duration.totalHours.toLocaleString()],
 ["Total minutes", duration.totalMins.toLocaleString()],
 ["Total seconds", duration.totalSecs.toLocaleString()],
 ["Total ms", duration.totalMs.toLocaleString()],
 ].map(([label, value]) => (
 <div key={label} style={{ display: "flex", justifyContent: "space-between", padding: "3px 0", fontSize: 12, borderBottom: "1px solid var(--border-color)" }}>
 <span style={{ color: "var(--text-secondary)", fontSize: 11 }}>{label}</span>
 <span style={{ fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{value}</span>
 </div>
 ))}
 </div>
 </div>
 )}
 </div>
 )}

 {/* ── RELATIVE ── */}
 {subTab === "relative" && (
 <div style={{ padding: "12px", display: "flex", flexDirection: "column", gap: 12 }}>
 <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
 <label style={{ fontSize: 11, color: "var(--text-secondary)", flexShrink: 0 }}>Base date:</label>
 <input type="datetime-local" value={relBase} onChange={e => setRelBase(e.target.value)}
 style={{ flex: 1, padding: "4px 8px", fontSize: 12, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none" }} />
 <button onClick={() => setRelBase(toLocalInputValue(new Date()))} style={{ padding: "3px 10px", fontSize: 10, background: "color-mix(in srgb, var(--accent-blue) 10%, transparent)", border: "1px solid var(--text-info)", borderRadius: 4, color: "var(--text-info)", cursor: "pointer" }}>Now</button>
 </div>

 {relDate && (
 <>
 <div style={{ padding: "8px 12px", background: "var(--bg-secondary)", borderRadius: 6, border: "1px solid var(--border-color)", fontSize: 13, textAlign: "center" }}>
 <span style={{ color: "var(--text-secondary)" }}>That date is </span>
 <span style={{ color: "var(--text-info)", fontWeight: 700 }}>{relativeTime(relDate)}</span>
 </div>

 <div>
 <div style={{ fontSize: 10, fontWeight: 700, color: "var(--text-secondary)", marginBottom: 8, letterSpacing: "0.05em" }}>OFFSETS FROM BASE DATE</div>
 <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))", gap: 4 }}>
 {offsets.map(days => {
 const d = new Date(relDate.getTime() + days * 86400000);
 const label = days === 0 ? "Base date" : days > 0 ? `+${days} day${Math.abs(days) !== 1 ? "s" : ""}` : `${days} day${Math.abs(days) !== 1 ? "s" : ""}`;
 return (
 <div key={days} style={{ display: "flex", justifyContent: "space-between", padding: "5px 10px", background: days === 0 ? "rgba(137,180,250,0.08)" : "var(--bg-secondary)", border: `1px solid ${days === 0 ? "var(--text-info)" : "var(--border-color)"}`, borderRadius: 4, fontSize: 11 }}>
 <span style={{ color: days < 0 ? "var(--text-danger)" : days > 0 ? "var(--text-success)" : "var(--text-info)", fontWeight: 600 }}>{label}</span>
 <span style={{ fontFamily: "var(--font-mono)", color: "var(--text-primary)", fontSize: 10 }}>{d.toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" })}</span>
 </div>
 );
 })}
 </div>
 </div>
 </>
 )}
 </div>
 )}

 {/* ── FORMATS REFERENCE ── */}
 {subTab === "formats" && (
 <div style={{ padding: "12px" }}>
 <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 12 }}>Common date format tokens used across languages and libraries.</div>
 {[
 { title: "ISO 8601 / ECMAScript",
 rows: [["YYYY-MM-DD","2025-01-15"],["YYYY-MM-DDTHH:mm:ssZ","2025-01-15T14:30:00Z"],["YYYY-MM-DDTHH:mm:ss.sssZ","2025-01-15T14:30:00.000Z"]] },
 { title: "Unix Epoch",
 rows: [["Seconds","1705326600"],["Milliseconds","1705326600000"]] },
 { title: "strftime (C/Python/Ruby/Go)",
 rows: [["%Y-%m-%d","2025-01-15"],["%Y-%m-%dT%H:%M:%S","2025-01-15T14:30:00"],["%d %b %Y","15 Jan 2025"],["%A, %B %d, %Y","Wednesday, January 15, 2025"],["%I:%M %p","02:30 PM"],["%s","Unix seconds"]] },
 { title: "moment.js / day.js / date-fns",
 rows: [["YYYY-MM-DD","2025-01-15"],["MMM D, YYYY","Jan 15, 2025"],["ddd, DD MMM YYYY HH:mm:ss","Wed, 15 Jan 2025 14:30:00"],["x","Unix ms"],["X","Unix seconds"]] },
 { title: "Java / Kotlin (DateTimeFormatter)",
 rows: [["yyyy-MM-dd","2025-01-15"],["yyyy-MM-dd'T'HH:mm:ss","2025-01-15T14:30:00"],["EEE, d MMM yyyy HH:mm:ss z","Wed, 15 Jan 2025 14:30:00 UTC"]] },
 { title: ".NET (C#)",
 rows: [["yyyy-MM-dd","2025-01-15"],["yyyy-MM-ddTHH:mm:ss","2025-01-15T14:30:00"],["ddd, dd MMM yyyy HH':'mm':'ss 'GMT'","Wed, 15 Jan 2025 14:30:00 GMT"]] },
 ].map(({ title, rows }) => (
 <div key={title} style={{ marginBottom: 16 }}>
 <div style={{ fontSize: 11, fontWeight: 700, color: "var(--text-info)", marginBottom: 6 }}>{title}</div>
 <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11 }}>
 <tbody>
 {rows.map(([fmt, ex]) => (
 <tr key={fmt} style={{ borderBottom: "1px solid var(--border-color)" }}>
 <td style={{ padding: "4px 8px", fontFamily: "var(--font-mono)", color: "var(--text-warning-alt)", width: "45%" }}>{fmt}</td>
 <td style={{ padding: "4px 8px", fontFamily: "var(--font-mono)", color: "var(--text-secondary)" }}>{ex}</td>
 </tr>
 ))}
 </tbody>
 </table>
 </div>
 ))}
 </div>
 )}

 </div>
 </div>
 );
}
