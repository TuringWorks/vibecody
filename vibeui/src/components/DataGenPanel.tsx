/**
 * DataGenPanel — Test data generator.
 *
 * Tabs:
 * Lorem : words / sentences / paragraphs with configurable count.
 * Fake Data : build a schema of fields (name, email, phone, etc.) and
 * generate N rows as JSON, CSV, or SQL INSERT.
 * UUID : bulk crypto.randomUUID() with copy-all and per-item copy.
 * Password : configurable password generator with strength meter.
 *
 * Schema save/load wired to Tauri backend. Generation logic stays client-side.
 */
import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CopyButton as CopyBtn } from "./shared/CopyButton";
import { X } from "lucide-react";

// ── Lorem Ipsum word bank ──────────────────────────────────────────────────────

const LOREM_WORDS = "lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua enim ad minim veniam quis nostrud exercitation ullamco laboris nisi aliquip ex ea commodo consequat duis aute irure dolor reprehenderit voluptate velit esse cillum fugiat nulla pariatur excepteur sint occaecat cupidatat non proident sunt culpa qui officia deserunt mollit anim est laborum".split(" ");

function loremWords(n: number): string {
 const out: string[] = [];
 for (let i = 0; i < n; i++) out.push(LOREM_WORDS[i % LOREM_WORDS.length]);
 return out.join(" ");
}

function loremSentence(): string {
 const len = 8 + Math.floor(rnd() * 10);
 const words = Array.from({ length: len }, () =>LOREM_WORDS[(Math.floor(rnd() * LOREM_WORDS.length))]);
 words[0] = words[0].charAt(0).toUpperCase() + words[0].slice(1);
 return words.join(" ") + ".";
}

function loremParagraph(): string {
 const count = 4 + Math.floor(rnd() * 4);
 return Array.from({ length: count }, loremSentence).join(" ");
}

// ── Deterministic-ish random (seeded per render but good enough) ───────────────

let _seed = 42;
function rnd(): number {
 _seed = (_seed * 1664525 + 1013904223) & 0xffffffff;
 return ((_seed >>> 0) / 0xffffffff);
}
function reseed() { _seed = Date.now() & 0xffffffff; }

// ── Fake data pools ───────────────────────────────────────────────────────────

const FIRST = ["Alice","Bob","Carol","Dave","Eve","Frank","Grace","Hank","Iris","Jack","Kara","Leo","Mia","Nate","Olivia","Pete","Quinn","Rose","Sam","Tara","Uma","Vic","Wendy","Xander","Yara","Zoe"];
const LAST = ["Smith","Johnson","Williams","Brown","Jones","Garcia","Miller","Davis","Wilson","Moore","Taylor","Anderson","Thomas","Jackson","White","Harris","Martin","Thompson","Young","Lee","Walker","Hall","Allen","King","Wright","Scott"];
const DOMAINS = ["example.com","test.org","demo.net","sample.io","fake.dev","mock.app"];
const STREETS = ["Main St","Oak Ave","Maple Dr","Cedar Ln","Pine Rd","Elm St","Park Blvd","Lake Dr","Hill Rd","River Ln"];
const CITIES = ["Springfield","Shelbyville","Ogdenville","Brockway","Waverly","Centerville","Greenfield","Fairview","Clinton","Franklin"];
const STATES = ["CA","TX","NY","FL","WA","OR","CO","AZ","IL","OH"];
const TLDS = ["com","org","net","io","dev","app","co"];
const COMPANIES = ["Acme Corp","Globex","Initech","Umbrella","Cyberdyne","Soylent","Stark Industries","Waystar","Pied Piper","Hooli"];
const JOBS = ["Engineer","Designer","Manager","Analyst","Director","Developer","Coordinator","Specialist","Consultant","Architect"];

function pick<T>(arr: T[]): T { return arr[Math.floor(rnd() * arr.length)]; }
function int(min: number, max: number): number { return min + Math.floor(rnd() * (max - min + 1)); }
function pad(n: number, len: number) { return String(n).padStart(len, "0"); }

const FAKE_TYPES = ["firstName","lastName","fullName","email","phone","company","jobTitle","street","city","state","zip","country","url","ipv4","date","boolean","number","uuid","color","username"] as const;
type FakeType = typeof FAKE_TYPES[number];

function fakeValue(type: FakeType): string {
 switch (type) {
 case "firstName": return pick(FIRST);
 case "lastName": return pick(LAST);
 case "fullName": return `${pick(FIRST)} ${pick(LAST)}`;
 case "email": {
 const u = `${pick(FIRST).toLowerCase()}.${pick(LAST).toLowerCase()}${int(1,99)}`;
 return `${u}@${pick(DOMAINS)}`;
 }
 case "phone": return `+1 (${pad(int(200,999),3)}) ${pad(int(200,999),3)}-${pad(int(1000,9999),4)}`;
 case "company": return pick(COMPANIES);
 case "jobTitle": return `${pick(JOBS)}`;
 case "street": return `${int(1,9999)} ${pick(STREETS)}`;
 case "city": return pick(CITIES);
 case "state": return pick(STATES);
 case "zip": return pad(int(10000,99999),5);
 case "country": return "US";
 case "url": return `https://www.${pick(LAST).toLowerCase()}.${pick(TLDS)}`;
 case "ipv4": return `${int(1,255)}.${int(0,255)}.${int(0,255)}.${int(1,254)}`;
 case "date": {
 const y = int(2020, 2025), m = int(1,12), d = int(1,28);
 return `${y}-${pad(m,2)}-${pad(d,2)}`;
 }
 case "boolean": return rnd() > 0.5 ? "true" : "false";
 case "number": return String(int(1, 10000));
 case "uuid": return crypto.randomUUID();
 case "color": return `#${int(0,0xffffff).toString(16).padStart(6,"0").toUpperCase()}`;
 case "username": {
 const adj = ["happy","fast","cool","dark","wild","clever","brave","swift"];
 return `${pick(adj)}_${pick(LAST).toLowerCase()}${int(1,99)}`;
 }
 }
}

interface SchemaField { id: string; name: string; type: FakeType }

function generateRow(schema: SchemaField[]): Record<string, string> {
 const row: Record<string, string> = {};
 schema.forEach(f => { row[f.name || f.type] = fakeValue(f.type); });
 return row;
}

function toJson(rows: Record<string, string>[]): string {
 return JSON.stringify(rows, null, 2);
}
function toCsv(rows: Record<string, string>[]): string {
 if (!rows.length) return "";
 const keys = Object.keys(rows[0]);
 const escape = (v: string) => v.includes(",") || v.includes('"') ? `"${v.replace(/"/g,'""')}"` : v;
 return [keys.join(","), ...rows.map(r => keys.map(k => escape(r[k])).join(","))].join("\n");
}
function toSql(rows: Record<string, string>[], table = "users"): string {
 if (!rows.length) return "";
 const keys = Object.keys(rows[0]);
 const cols = keys.map(k => `\`${k}\``).join(", ");
 return rows.map(r => {
 const vals = keys.map(k => `'${r[k].replace(/'/g, "''")}'`).join(", ");
 return `INSERT INTO \`${table}\` (${cols}) VALUES (${vals});`;
 }).join("\n");
}

// ── Password generator ────────────────────────────────────────────────────────

const CHARSET = {
 upper: "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
 lower: "abcdefghijklmnopqrstuvwxyz",
 digits: "0123456789",
 symbols: "!@#$%^&*()_+-=[]{}|;:,.<>?",
};

function generatePassword(len: number, opts: { upper: boolean; lower: boolean; digits: boolean; symbols: boolean }): string {
 const pool = [opts.upper && CHARSET.upper, opts.lower && CHARSET.lower, opts.digits && CHARSET.digits, opts.symbols && CHARSET.symbols].filter(Boolean).join("");
 if (!pool) return "";
 const arr = new Uint32Array(len);
 crypto.getRandomValues(arr);
 return Array.from(arr).map(n => pool[n % pool.length]).join("");
}

function passwordStrength(pw: string): { score: number; label: string; colour: string } {
 let score = 0;
 if (pw.length >= 8) score++;
 if (pw.length >= 12) score++;
 if (pw.length >= 16) score++;
 if (/[A-Z]/.test(pw)) score++;
 if (/[a-z]/.test(pw)) score++;
 if (/\d/.test(pw)) score++;
 if (/[^A-Za-z0-9]/.test(pw)) score++;
 if (score <= 2) return { score, label: "Weak", colour: "var(--error-color)" };
 if (score <= 4) return { score, label: "Fair", colour: "var(--warning-color)" };
 if (score <= 5) return { score, label: "Good", colour: "var(--warning-color)" };
 return { score, label: "Strong", colour: "var(--success-color)" };
}

// ── Component ──────────────────────────────────────────────────────────────────

type SubTab = "lorem" | "fakedata" | "uuid" | "password";

// CopyBtn imported from shared/CopyButton.tsx

export function DataGenPanel() {
 const [subTab, setSubTab] = useState<SubTab>("fakedata");

 // ── Lorem state ─────────────────────────────────────────────────────────────
 const [loremMode, setLoremMode] = useState<"words" | "sentences" | "paragraphs">("paragraphs");
 const [loremCount, setLoremCount] = useState(3);
 const [loremOutput, setLoremOutput] = useState("");

 const genLorem = useCallback(() => {
 reseed();
 let out = "";
 if (loremMode === "words") out = loremWords(loremCount);
 else if (loremMode === "sentences") out = Array.from({length: loremCount}, loremSentence).join(" ");
 else out = Array.from({length: loremCount}, loremParagraph).join("\n\n");
 setLoremOutput(out);
 }, [loremMode, loremCount]);

 // ── Fake data state ──────────────────────────────────────────────────────────
 const [schema, setSchema] = useState<SchemaField[]>([
 { id: "1", name: "name", type: "fullName" },
 { id: "2", name: "email", type: "email" },
 { id: "3", name: "company", type: "company" },
 { id: "4", name: "city", type: "city" },
 ]);
 const [rowCount, setRowCount] = useState(10);
 const [outFormat, setOutFormat] = useState<"json" | "csv" | "sql">("json");
 const [sqlTable, setSqlTable] = useState("users");
 const [fakeOutput, setFakeOutput] = useState("");

 const addField = () => setSchema(s => [...s, { id: Date.now().toString(), name: FAKE_TYPES[s.length % FAKE_TYPES.length], type: FAKE_TYPES[s.length % FAKE_TYPES.length] }]);
 const removeField = (id: string) => setSchema(s => s.filter(f => f.id !== id));
 const updateField = (id: string, patch: Partial<SchemaField>) => setSchema(s => s.map(f => f.id === id ? { ...f, ...patch } : f));

 const genFakeData = useCallback(() => {
 reseed();
 const rows = Array.from({ length: rowCount }, () => generateRow(schema));
 if (outFormat === "json") setFakeOutput(toJson(rows));
 else if (outFormat === "csv") setFakeOutput(toCsv(rows));
 else setFakeOutput(toSql(rows, sqlTable));
 }, [schema, rowCount, outFormat, sqlTable]);

 // ── Schema save/load via Tauri backend ─────────────────────────────────────
 interface SavedSchema { id: string; name: string; fields: SchemaField[]; created: string }
 const [savedSchemas, setSavedSchemas] = useState<SavedSchema[]>([]);
 const [schemaName, setSchemaName] = useState("");

 const loadSavedSchemas = useCallback(async () => {
 try {
 const list = await invoke<SavedSchema[]>("datagen_list_schemas");
 setSavedSchemas(Array.isArray(list) ? list : []);
 } catch { /* ignore */ }
 }, []);

 useEffect(() => { loadSavedSchemas(); }, [loadSavedSchemas]);

 const saveCurrentSchema = useCallback(async () => {
 const name = schemaName.trim() || `Schema ${new Date().toLocaleTimeString()}`;
 try {
 await invoke("datagen_save_schema", { name, fields: schema });
 setSchemaName("");
 loadSavedSchemas();
 } catch { /* ignore */ }
 }, [schema, schemaName, loadSavedSchemas]);

 const loadSchema = useCallback((saved: SavedSchema) => {
 if (Array.isArray(saved.fields)) {
 setSchema(saved.fields as SchemaField[]);
 }
 }, []);

 // ── UUID state ───────────────────────────────────────────────────────────────
 const [uuidCount, setUuidCount] = useState(10);
 const [uuidVersion, setUuidVersion] = useState<"v4" | "v7">("v4");
 const [uuids, setUuids] = useState<string[]>([]);

 const genUuids = useCallback(() => {
 const list: string[] = [];
 for (let i = 0; i < uuidCount; i++) {
 if (uuidVersion === "v4") {
 list.push(crypto.randomUUID());
 } else {
 // v7: timestamp-based (simulated)
 const ts = Date.now();
 const tsHex = ts.toString(16).padStart(12, "0");
 const rand = crypto.randomUUID().replace(/-/g, "").slice(12);
 list.push(`${tsHex.slice(0,8)}-${tsHex.slice(8,12)}-7${rand.slice(0,3)}-${(parseInt(rand[3],16) & 0x3 | 0x8).toString(16)}${rand.slice(4,7)}-${rand.slice(7,19)}`);
 }
 }
 setUuids(list);
 }, [uuidCount, uuidVersion]);

 // ── Password state ───────────────────────────────────────────────────────────
 const [pwLen, setPwLen] = useState(16);
 const [pwOpts, setPwOpts] = useState({ upper: true, lower: true, digits: true, symbols: true });
 const [pwCount, setPwCount] = useState(5);
 const [passwords, setPasswords] = useState<string[]>([]);

 const genPasswords = useCallback(() => {
 setPasswords(Array.from({ length: pwCount }, () => generatePassword(pwLen, pwOpts)));
 }, [pwLen, pwOpts, pwCount]);

 const TABS: { id: SubTab; label: string }[] = [
 { id: "fakedata", label: "Fake Data" },
 { id: "lorem", label: "Lorem Ipsum" },
 { id: "uuid", label: "UUID" },
 { id: "password", label: "Password" },
 ];

 return (
 <div className="panel-container">

 {/* Header */}
 <div className="panel-header" style={{ display: "flex", gap: 6, alignItems: "center", flexWrap: "wrap" }}>
 <span style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>Data Generator</span>
 <div className="panel-tab-bar" style={{ border: "none", padding: 0, margin: 0 }}>
 {TABS.map(t => (
 <button key={t.id} onClick={() => setSubTab(t.id)} className={`panel-tab ${subTab === t.id ? "active" : ""}`}>{t.label}</button>
 ))}
 </div>
 </div>

 <div className="panel-body" style={{ padding: 0 }}>

 {/* ── FAKE DATA ── */}
 {subTab === "fakedata" && (
 <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
 {/* Schema builder */}
 <div style={{ width: 260, flexShrink: 0, borderRight: "1px solid var(--border-color)", display: "flex", flexDirection: "column", overflow: "hidden" }}>
 <div style={{ padding: "8px 12px", fontSize: "var(--font-size-xs)", fontWeight: 700, color: "var(--text-secondary)", background: "var(--bg-secondary)", borderBottom: "1px solid var(--border-color)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
 <span>SCHEMA FIELDS</span>
 <div style={{ display: "flex", gap: 3 }}>
 <button className="panel-btn" onClick={addField} style={{ fontSize: 9, padding: "1px 8px", background: "color-mix(in srgb, var(--accent-blue) 15%, transparent)", border: "1px solid var(--accent-primary)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-info)", cursor: "pointer" }}>+ Add</button>
 <button className="panel-btn" onClick={saveCurrentSchema} style={{ fontSize: 9, padding: "1px 8px", background: "color-mix(in srgb, var(--accent-green) 15%, transparent)", border: "1px solid var(--success-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-success)", cursor: "pointer" }}>Save</button>
 </div>
 </div>
 <div style={{ flex: 1, overflow: "auto" }}>
 {schema.map((f, i) => (
 <div key={f.id} style={{ padding: "4px 8px", borderBottom: "1px solid var(--border-subtle)", display: "flex", gap: 5, alignItems: "center" }}>
 <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", width: 14, flexShrink: 0 }}>{i+1}.</span>
 <input value={f.name} onChange={e => updateField(f.id, { name: e.target.value })} placeholder="field name"
 style={{ width: 72, padding: "2px 4px", fontSize: "var(--font-size-xs)", fontFamily: "var(--font-mono)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 3, color: "var(--text-primary)", outline: "none" }} />
 <select value={f.type} onChange={e => updateField(f.id, { type: e.target.value as FakeType })}
 style={{ flex: 1, padding: "2px 4px", fontSize: "var(--font-size-xs)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 3, color: "var(--text-primary)", outline: "none" }}>
 {FAKE_TYPES.map(t => <option key={t} value={t}>{t}</option>)}
 </select>
 <button onClick={() => removeField(f.id)} style={{ background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", padding: "0 2px", display: "flex", alignItems: "center" }}><X size={10} /></button>
 </div>
 ))}
 </div>
 {/* Options */}
 <div style={{ padding: "8px", borderTop: "1px solid var(--border-color)", display: "flex", flexDirection: "column", gap: 6 }}>
 <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
 <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", flex: 1 }}>Rows:</span>
 <input type="number" value={rowCount} min={1} max={500} onChange={e => setRowCount(Math.min(500, Math.max(1, +e.target.value)))}
 style={{ width: 60, padding: "2px 8px", fontSize: "var(--font-size-sm)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none" }} />
 </div>
 <div style={{ display: "flex", gap: 4 }}>
 {(["json","csv","sql"] as const).map(f => <button key={f} onClick={() => setOutFormat(f)} className={`panel-tab ${outFormat === f ? "active" : ""}`}>{f.toUpperCase()}</button>)}
 </div>
 {outFormat === "sql" && (
 <input value={sqlTable} onChange={e => setSqlTable(e.target.value)} placeholder="table name"
 style={{ padding: "3px 8px", fontSize: "var(--font-size-sm)", fontFamily: "var(--font-mono)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none" }} />
 )}
 <button className="panel-btn" onClick={genFakeData} style={{ padding: "4px", fontSize: "var(--font-size-sm)", fontWeight: 700, background: "color-mix(in srgb, var(--accent-green) 15%, transparent)", border: "1px solid var(--success-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-success)", cursor: "pointer" }}>Generate</button>
 {/* Schema name input for saving */}
 <input value={schemaName} onChange={e => setSchemaName(e.target.value)} placeholder="Schema name..." style={{ padding: "3px 8px", fontSize: "var(--font-size-xs)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none" }} />
 {/* Saved schemas list */}
 {savedSchemas.length > 0 && (
 <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", fontWeight: 700, marginTop: 2 }}>SAVED SCHEMAS</div>
 )}
 {savedSchemas.map(s => (
 <button key={s.id} onClick={() => loadSchema(s)} style={{ padding: "3px 8px", fontSize: "var(--font-size-xs)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", cursor: "pointer", textAlign: "left" as const }}>{s.name}</button>
 ))}
 </div>
 </div>
 {/* Output */}
 <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
 <div style={{ padding: "4px 12px", fontSize: "var(--font-size-xs)", fontWeight: 700, color: "var(--text-secondary)", background: "var(--bg-secondary)", borderBottom: "1px solid var(--border-color)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
 <span>OUTPUT</span>
 {fakeOutput && <CopyBtn text={fakeOutput} label="Copy all" />}
 </div>
 <pre style={{ flex: 1, margin: 0, padding: "12px 12px", fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)", lineHeight: 1.6, overflow: "auto", whiteSpace: "pre-wrap", wordBreak: "break-all", color: "var(--text-primary)", background: "var(--bg-primary)" }}>
 {fakeOutput || <span style={{ color: "var(--text-secondary)", fontStyle: "italic" }}>Configure schema and click Generate </span>}
 </pre>
 </div>
 </div>
 )}

 {/* ── LOREM IPSUM ── */}
 {subTab === "lorem" && (
 <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
 {(["words","sentences","paragraphs"] as const).map(m => (
 <button key={m} onClick={() => setLoremMode(m)} className={`panel-tab ${loremMode === m ? "active" : ""}`}>{m.charAt(0).toUpperCase() + m.slice(1)}</button>
 ))}
 <input type="number" value={loremCount} min={1} max={loremMode === "words" ? 500 : loremMode === "sentences" ? 100 : 20}
 onChange={e => setLoremCount(Math.max(1, +e.target.value))}
 style={{ width: 60, padding: "3px 8px", fontSize: "var(--font-size-base)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none" }} />
 <button className="panel-btn" onClick={genLorem} style={{ padding: "3px 16px", fontSize: "var(--font-size-sm)", fontWeight: 700, background: "color-mix(in srgb, var(--accent-green) 15%, transparent)", border: "1px solid var(--success-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-success)", cursor: "pointer" }}>Generate</button>
 {loremOutput && <CopyBtn text={loremOutput} label="Copy" />}
 </div>
 <div style={{ padding: "12px", fontSize: "var(--font-size-md)", lineHeight: 1.8, color: "var(--text-primary)", whiteSpace: "pre-wrap" }}>
 {loremOutput || <span style={{ color: "var(--text-secondary)", fontStyle: "italic" }}>Click Generate to produce lorem ipsum text.</span>}
 </div>
 </div>
 )}

 {/* ── UUID ── */}
 {subTab === "uuid" && (
 <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
 {(["v4","v7"] as const).map(v => <button key={v} onClick={() => setUuidVersion(v)} className={`panel-tab ${uuidVersion === v ? "active" : ""}`}>UUID {v}</button>)}
 <input type="number" value={uuidCount} min={1} max={100} onChange={e => setUuidCount(Math.min(100, Math.max(1, +e.target.value)))}
 style={{ width: 60, padding: "3px 8px", fontSize: "var(--font-size-base)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none" }} />
 <button className="panel-btn" onClick={genUuids} style={{ padding: "3px 16px", fontSize: "var(--font-size-sm)", fontWeight: 700, background: "color-mix(in srgb, var(--accent-green) 15%, transparent)", border: "1px solid var(--success-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-success)", cursor: "pointer" }}>Generate</button>
 {uuids.length > 0 && <CopyBtn text={uuids.join("\n")} label="Copy all" />}
 </div>
 <div style={{ padding: "8px 12px" }}>
 {uuids.length === 0
 ? <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-base)", fontStyle: "italic" }}>Click Generate to produce UUIDs.</span>
 : uuids.map((u, i) => (
 <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, padding: "4px 0", borderBottom: "1px solid var(--border-subtle)" }}>
 <span style={{ fontSize: 9, color: "var(--text-secondary)", width: 24, textAlign: "right", flexShrink: 0, fontFamily: "var(--font-mono)" }}>{i+1}</span>
 <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-base)", color: "var(--text-info)", flex: 1, letterSpacing: "0.03em" }}>{u}</span>
 <CopyBtn text={u} />
 </div>
 ))
 }
 </div>
 </div>
 )}

 {/* ── PASSWORD ── */}
 {subTab === "password" && (
 <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
 {/* Options */}
 <div style={{ padding: "12px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", flexDirection: "column", gap: 8 }}>
 <div style={{ display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
 <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Length:</span>
 <input type="range" min={6} max={128} value={pwLen} onChange={e => setPwLen(+e.target.value)} style={{ flex: 1, minWidth: 80, accentColor: "var(--accent-color)" }} />
 <span style={{ fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono)", color: "var(--text-primary)", width: 30 }}>{pwLen}</span>
 </div>
 <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
 {([["upper","A–Z"],["lower","a–z"],["digits","0–9"],["symbols","!@#"]] as [keyof typeof pwOpts, string][]).map(([k, label]) => (
 <label key={k} style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", display: "flex", gap: 4, alignItems: "center", cursor: "pointer" }}>
 <input type="checkbox" checked={pwOpts[k]} onChange={e => setPwOpts(o => ({...o, [k]: e.target.checked}))} style={{ accentColor: "var(--accent-color)" }} />
 {label}
 </label>
 ))}
 </div>
 <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
 <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Count:</span>
 <input type="number" value={pwCount} min={1} max={50} onChange={e => setPwCount(Math.min(50, Math.max(1, +e.target.value)))}
 style={{ width: 55, padding: "3px 8px", fontSize: "var(--font-size-sm)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none" }} />
 <button className="panel-btn" onClick={genPasswords} style={{ padding: "3px 16px", fontSize: "var(--font-size-sm)", fontWeight: 700, background: "color-mix(in srgb, var(--accent-green) 15%, transparent)", border: "1px solid var(--success-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-success)", cursor: "pointer" }}>Generate</button>
 {passwords.length > 0 && <CopyBtn text={passwords.join("\n")} label="Copy all" />}
 </div>
 </div>
 {/* Results */}
 <div style={{ padding: "8px 12px" }}>
 {passwords.length === 0
 ? <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-base)", fontStyle: "italic" }}>Configure options and click Generate.</span>
 : passwords.map((pw, i) => {
 const str = passwordStrength(pw);
 return (
 <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, padding: "4px 0", borderBottom: "1px solid var(--border-subtle)" }}>
 <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-base)", color: "var(--text-primary)", flex: 1, letterSpacing: "0.05em", wordBreak: "break-all" }}>{pw}</span>
 <span style={{ fontSize: 9, fontWeight: 700, color: str.colour, padding: "1px 8px", background: `${str.colour}22`, border: `1px solid ${str.colour}`, borderRadius: "var(--radius-md)", flexShrink: 0 }}>{str.label}</span>
 <CopyBtn text={pw} />
 </div>
 );
 })
 }
 </div>
 </div>
 )}

 </div>
 </div>
 );
}
