import { useState, useRef, useCallback, useMemo } from "react";
import { Table, X, ChevronDown, ChevronUp } from "lucide-react";

type Row = string[];
type SortDir = "asc" | "desc" | null;

function parseCsv(text: string, delimiter: string): Row[] {
 const lines = text.split(/\r?\n/);
 const rows: Row[] = [];
 for (const line of lines) {
 if (!line.trim()) continue;
 const cells: string[] = [];
 let cur = "";
 let inQuote = false;
 for (let i = 0; i < line.length; i++) {
 const ch = line[i];
 if (ch === '"') {
 if (inQuote && line[i + 1] === '"') { cur += '"'; i++; }
 else inQuote = !inQuote;
 } else if (ch === delimiter && !inQuote) {
 cells.push(cur); cur = "";
 } else {
 cur += ch;
 }
 }
 cells.push(cur);
 rows.push(cells);
 }
 return rows;
}

function serializeCsv(rows: Row[], delimiter: string): string {
 return rows.map(row =>
 row.map(cell => {
 if (cell.includes(delimiter) || cell.includes('"') || cell.includes("\n")) {
 return `"${cell.replace(/"/g, '""')}"`;
 }
 return cell;
 }).join(delimiter)
 ).join("\n");
}

function detectDelimiter(text: string): string {
 const sample = text.slice(0, 2000);
 const counts: Record<string, number> = { ",": 0, "\t": 0, ";": 0, "|": 0 };
 for (const ch of sample) if (ch in counts) counts[ch]++;
 return Object.entries(counts).sort((a, b) => b[1] - a[1])[0][0];
}

const CARD: React.CSSProperties = { background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", padding: "8px 14px", display: "flex", flexDirection: "column", alignItems: "center" };

export function CsvPanel() {
 const [tab, setTab] = useState<"table" | "stats" | "filter" | "convert">("table");
 const [rawText, setRawText] = useState("");
 const [delimiter, setDelimiter] = useState(",");
 const [hasHeader, setHasHeader] = useState(true);
 const [sortCol, setSortCol] = useState<number | null>(null);
 const [sortDir, setSortDir] = useState<SortDir>(null);
 const [filterText, setFilterText] = useState("");
 const [filterCol, setFilterCol] = useState<number | "all">("all");
 const [editCell, setEditCell] = useState<{ r: number; c: number } | null>(null);
 const [editVal, setEditVal] = useState("");
 const [rows, setRows] = useState<Row[]>([]);
 const fileInputRef = useRef<HTMLInputElement>(null);

 const loadText = useCallback((text: string) => {
 const det = detectDelimiter(text);
 setDelimiter(det);
 const parsed = parseCsv(text, det);
 setRows(parsed);
 setRawText(text);
 setSortCol(null); setSortDir(null); setFilterText("");
 }, []);

 const handleFile = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
 const file = e.target.files?.[0];
 if (!file) return;
 const reader = new FileReader();
 reader.onload = ev => loadText((ev.target?.result as string) ?? "");
 reader.readAsText(file);
 }, [loadText]);

 const headers = useMemo(() => hasHeader && rows.length > 0 ? rows[0] : [], [hasHeader, rows]);
 const dataRows = useMemo(() => hasHeader ? rows.slice(1) : rows, [hasHeader, rows]);

 const filteredRows = useMemo(() => {
 if (!filterText) return dataRows;
 const q = filterText.toLowerCase();
 return dataRows.filter(row => {
 if (filterCol === "all") return row.some(c => c.toLowerCase().includes(q));
 return (row[filterCol as number] ?? "").toLowerCase().includes(q);
 });
 }, [dataRows, filterText, filterCol]);

 const sortedRows = useMemo(() => {
 if (sortCol === null || !sortDir) return filteredRows;
 return [...filteredRows].sort((a, b) => {
 const av = a[sortCol] ?? "", bv = b[sortCol] ?? "";
 const an = Number(av), bn = Number(bv);
 const cmp = (!isNaN(an) && !isNaN(bn)) ? an - bn : av.localeCompare(bv);
 return sortDir === "asc" ? cmp : -cmp;
 });
 }, [filteredRows, sortCol, sortDir]);

 const handleSort = (col: number) => {
 if (sortCol === col) {
 setSortDir(d => d === "asc" ? "desc" : d === "desc" ? null : "asc");
 if (sortDir === "desc") setSortCol(null);
 } else {
 setSortCol(col); setSortDir("asc");
 }
 };

 const commitEdit = () => {
 if (!editCell) return;
 const { r, c } = editCell;
 const offset = hasHeader ? 1 : 0;
 const newRows = rows.map((row, ri) => ri === r + offset ? row.map((cell, ci) => ci === c ? editVal : cell) : row);
 setRows(newRows);
 setRawText(serializeCsv(newRows, delimiter));
 setEditCell(null);
 };

 const addRow = () => {
 const width = headers.length || (rows[0]?.length ?? 1);
 const newRow = Array(width).fill("");
 const newRows = [...rows, newRow];
 setRows(newRows);
 setRawText(serializeCsv(newRows, delimiter));
 };

 const deleteRow = (ri: number) => {
 const offset = hasHeader ? 1 : 0;
 const newRows = rows.filter((_, i) => i !== ri + offset);
 setRows(newRows);
 setRawText(serializeCsv(newRows, delimiter));
 };

 const exportCsv = () => {
 const allRows = hasHeader ? [headers, ...sortedRows] : sortedRows;
 const blob = new Blob([serializeCsv(allRows, delimiter)], { type: "text/csv" });
 const a = document.createElement("a"); a.href = URL.createObjectURL(blob); a.download = "export.csv"; a.click();
 };

 const convertToJson = () => {
 if (!headers.length) return JSON.stringify(sortedRows, null, 2);
 return JSON.stringify(sortedRows.map(row =>Object.fromEntries(headers.map((h, i) => [h, row[i] ?? ""]))), null, 2);
 };

 const convertToSql = (table = "my_table") => {
 if (!headers.length) return "";
 const cols = headers.map(h => `\`${h}\``).join(", ");
 return sortedRows.map(row => {
 const vals = row.map(v => `'${v.replace(/'/g, "''")}'`).join(", ");
 return `INSERT INTO \`${table}\` (${cols}) VALUES (${vals});`;
 }).join("\n");
 };

 // Stats
 const stats = useMemo(() => {
 if (!headers.length) return null;
 return headers.map((h, ci) => {
 const vals = dataRows.map(r => r[ci] ?? "");
 const nums = vals.map(Number).filter(n => !isNaN(n) && n.toString() !== "");
 const nonEmpty = vals.filter(v => v !== "").length;
 const unique = new Set(vals).size;
 const avg = nums.length ? nums.reduce((a, b) => a + b, 0) / nums.length : null;
 const mn = nums.length ? Math.min(...nums) : null;
 const mx = nums.length ? Math.max(...nums) : null;
 return { header: h, nonEmpty, unique, avg, min: mn, max: mx, isNumeric: nums.length > dataRows.length * 0.6 };
 });
 }, [headers, dataRows]);

 const TABS: { id: typeof tab; label: string }[] = [
 { id: "table", label: "Table" },
 { id: "filter", label: "Filter" },
 { id: "stats", label: "Stats" },
 { id: "convert", label: "Convert" },
 ];

 return (
 <div className="panel-container">
 {/* Toolbar */}
 <div className="panel-header" style={{ flexWrap: "wrap" }}>
 <input ref={fileInputRef} type="file" accept=".csv,.tsv,.txt" style={{ display: "none" }} onChange={handleFile} />
 <button onClick={() => fileInputRef.current?.click()}
 style={{ padding: "4px 12px", background: "var(--accent-color)", color: "var(--text-on-accent)", border: "none", borderRadius: "var(--radius-xs-plus)", cursor: "pointer", fontSize: "var(--font-size-base)" }}>
 Open File
 </button>
 <label style={{ fontSize: "var(--font-size-base)", display: "flex", alignItems: "center", gap: 4 }}>
 Delimiter:
 <select value={delimiter} onChange={e => { setDelimiter(e.target.value); setRows(parseCsv(rawText, e.target.value)); }}
 style={{ background: "var(--bg-tertiary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: "2px 4px" }}>
 <option value=",">, (CSV)</option>
 <option value={"\t"}>⇥ (TSV)</option>
 <option value=";">; (semicolon)</option>
 <option value="|">| (pipe)</option>
 </select>
 </label>
 <label style={{ fontSize: "var(--font-size-base)", display: "flex", alignItems: "center", gap: 4 }}>
 <input type="checkbox" checked={hasHeader} onChange={e => setHasHeader(e.target.checked)} />
 Header row
 </label>
 {rows.length > 0 && (
 <>
 <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginLeft: "auto" }}>
 {dataRows.length} rows × {headers.length || rows[0]?.length || 0} cols
 </span>
 <button className="panel-btn" onClick={exportCsv}
 style={{ padding: "4px 12px", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", cursor: "pointer", fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>
 Export
 </button>
 </>
 )}
 </div>

 {/* Paste area when empty */}
 {rows.length === 0 && (
 <div className="panel-body" style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 16, padding: 24 }}>
 <Table size={32} strokeWidth={1.5} style={{ color: "var(--accent, #4a9eff)" }} />
 <div style={{ color: "var(--text-secondary)", textAlign: "center" }}>Open a CSV / TSV file or paste data below</div>
 <textarea
 placeholder="Paste CSV data here..."
 style={{ width: "100%", height: 180, background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-sm)", padding: 8, fontSize: "var(--font-size-base)", resize: "vertical", boxSizing: "border-box" }}
 onChange={e => { if (e.target.value) loadText(e.target.value); }}
 />
 <div style={{ display: "flex", gap: 8 }}>
 {[
 "Name,Age,City\nAlice,30,NYC\nBob,25,LA\nCarla,35,Chicago",
 "product\tprice\tstock\nApple\t1.20\t500\nBanana\t0.50\t1200\nCherry\t3.00\t80",
 ].map((sample, i) => (
 <button key={i} onClick={() => loadText(sample)}
 style={{ padding: "4px 12px", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", cursor: "pointer", fontSize: "var(--font-size-sm)", color: "var(--text-primary)" }}>
 {i === 0 ? "CSV Sample" : "TSV Sample"}
 </button>
 ))}
 </div>
 </div>
 )}

 {rows.length > 0 && (
 <>
 {/* Sub-tabs */}
 <div className="panel-tab-bar">
 {TABS.map(t => (
 <button key={t.id} onClick={() => setTab(t.id)}
 className={`panel-tab ${tab === t.id ? "active" : ""}`}>
 {t.label}
 </button>
 ))}
 </div>

 {/* Table */}
 {tab === "table" && (
 <div className="panel-body" style={{ padding: 8 }}>
 <div style={{ overflowX: "auto" }}>
 <table style={{ borderCollapse: "collapse", width: "100%", fontSize: "var(--font-size-base)" }}>
 {hasHeader && headers.length > 0 && (
 <thead>
 <tr>
 <th style={{ padding: "4px 8px", borderBottom: "2px solid var(--accent-blue)", color: "var(--text-secondary)", fontSize: "var(--font-size-sm)", textAlign: "left", width: 32 }}>#</th>
 {headers.map((h, ci) => (
 <th key={ci} onClick={() => handleSort(ci)}
 style={{ padding: "4px 8px", borderBottom: "2px solid var(--accent-blue)", color: "var(--text-primary)", textAlign: "left", cursor: "pointer", userSelect: "none", whiteSpace: "nowrap" }}>
 {h} {sortCol === ci ? (sortDir === "asc" ? <ChevronUp size={10} /> : <ChevronDown size={10} />) : ""}
 </th>
 ))}
 <th style={{ width: 32 }} />
 </tr>
 </thead>
 )}
 <tbody>
 {sortedRows.map((row, ri) => (
 <tr key={ri} style={{ background: ri % 2 === 0 ? "transparent" : "var(--border-subtle)" }}>
 <td style={{ padding: "3px 8px", color: "var(--text-secondary)", fontSize: "var(--font-size-xs)" }}>{ri + 1}</td>
 {row.map((cell, ci) => (
 <td key={ci} onDoubleClick={() => { setEditCell({ r: ri, c: ci }); setEditVal(cell); }}
 style={{ padding: "3px 8px", borderBottom: "1px solid var(--border-color)", maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", wordBreak: "break-word", cursor: "text" }}>
 {editCell?.r === ri && editCell?.c === ci ? (
 <input autoFocus value={editVal} onChange={e => setEditVal(e.target.value)}
 onBlur={commitEdit} onKeyDown={e => { if (e.key === "Enter") commitEdit(); if (e.key === "Escape") setEditCell(null); }}
 style={{ width: "100%", background: "var(--bg-tertiary)", color: "var(--text-primary)", border: "1px solid var(--accent-color)", borderRadius: 2, padding: "1px 4px", fontSize: "var(--font-size-base)", boxSizing: "border-box" }} />
 ) : cell}
 </td>
 ))}
 <td style={{ padding: "2px 4px" }}>
 <button onClick={() => deleteRow(ri)} title="Delete row"
 style={{ background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", padding: "0 2px", display: "flex", alignItems: "center" }}><X size={11} /></button>
 </td>
 </tr>
 ))}
 </tbody>
 </table>
 </div>
 <button className="panel-btn" onClick={addRow}
 style={{ marginTop: 8, padding: "4px 12px", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", cursor: "pointer", fontSize: "var(--font-size-sm)", color: "var(--text-primary)" }}>
 + Add Row
 </button>
 </div>
 )}

 {/* Filter */}
 {tab === "filter" && (
 <div className="panel-body" style={{ padding: 12 }}>
 <div style={{ display: "flex", gap: 8, marginBottom: 12, alignItems: "center" }}>
 <input value={filterText} onChange={e => setFilterText(e.target.value)} placeholder="Filter value..."
 style={{ flex: 1, background: "var(--bg-secondary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: "4px 8px", fontSize: "var(--font-size-base)" }} />
 <select value={filterCol} onChange={e => setFilterCol(e.target.value === "all" ? "all" : Number(e.target.value))}
 style={{ background: "var(--bg-tertiary)", color: "var(--text-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: "4px 8px", fontSize: "var(--font-size-base)" }}>
 <option value="all">All columns</option>
 {headers.map((h, i) => <option key={i} value={i}>{h}</option>)}
 </select>
 <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{filteredRows.length} / {dataRows.length} rows</span>
 </div>
 <div style={{ overflowX: "auto" }}>
 <table style={{ borderCollapse: "collapse", width: "100%", fontSize: "var(--font-size-base)" }}>
 {hasHeader && headers.length > 0 && (
 <thead>
 <tr>
 {headers.map((h, ci) => (
 <th key={ci} style={{ padding: "4px 8px", borderBottom: "2px solid var(--accent-blue)", color: "var(--text-primary)", textAlign: "left" }}>{h}</th>
 ))}
 </tr>
 </thead>
 )}
 <tbody>
 {filteredRows.slice(0, 200).map((row, ri) => (
 <tr key={ri} style={{ background: ri % 2 === 0 ? "transparent" : "var(--border-subtle)" }}>
 {row.map((cell, ci) => {
 const highlighted = filterText && (filterCol === "all" || filterCol === ci) && cell.toLowerCase().includes(filterText.toLowerCase());
 return (
 <td key={ci} style={{ padding: "3px 8px", borderBottom: "1px solid var(--border-color)", background: highlighted ? "rgba(255,200,0,0.15)" : undefined }}>
 {cell}
 </td>
 );
 })}
 </tr>
 ))}
 </tbody>
 </table>
 </div>
 {filteredRows.length > 200 && <div style={{ padding: 8, color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>Showing first 200 of {filteredRows.length} matches</div>}
 </div>
 )}

 {/* Stats */}
 {tab === "stats" && stats && (
 <div className="panel-body" style={{ padding: 12 }}>
 <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))", gap: 10, marginBottom: 16 }}>
 {[
 { label: "Total Rows", value: dataRows.length },
 { label: "Columns", value: headers.length },
 { label: "Filtered Rows", value: filteredRows.length },
 { label: "Total Cells", value: dataRows.length * (headers.length || 1) },
 ].map(({ label, value }) => (
 <div key={label} style={CARD}>
 <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--text-primary)" }}>{value}</div>
 <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{label}</div>
 </div>
 ))}
 </div>
 <div style={{ overflowX: "auto" }}>
 <table style={{ borderCollapse: "collapse", width: "100%", fontSize: "var(--font-size-base)" }}>
 <thead>
 <tr>
 {["Column", "Non-empty", "Unique", "Type", "Min", "Max", "Avg"].map(h => (
 <th key={h} style={{ padding: "4px 8px", borderBottom: "2px solid var(--accent-blue)", color: "var(--text-secondary)", textAlign: "left", fontSize: "var(--font-size-sm)" }}>{h}</th>
 ))}
 </tr>
 </thead>
 <tbody>
 {stats.map((s, i) => (
 <tr key={i} style={{ background: i % 2 === 0 ? "transparent" : "var(--border-subtle)" }}>
 <td style={{ padding: "4px 8px", fontWeight: 600 }}>{s.header}</td>
 <td style={{ padding: "4px 8px" }}>{s.nonEmpty}</td>
 <td style={{ padding: "4px 8px" }}>{s.unique}</td>
 <td style={{ padding: "4px 8px" }}><span style={{ padding: "1px 4px", borderRadius: 3, fontSize: "var(--font-size-xs)", background: s.isNumeric ? "rgba(100,200,100,0.2)" : "rgba(100,150,255,0.2)", color: s.isNumeric ? "var(--text-success)" : "var(--text-info)" }}>{s.isNumeric ? "numeric" : "text"}</span></td>
 <td style={{ padding: "4px 8px", color: "var(--text-secondary)" }}>{s.min ?? "—"}</td>
 <td style={{ padding: "4px 8px", color: "var(--text-secondary)" }}>{s.max ?? "—"}</td>
 <td style={{ padding: "4px 8px", color: "var(--text-secondary)" }}>{s.avg != null ? s.avg.toFixed(2) : "—"}</td>
 </tr>
 ))}
 </tbody>
 </table>
 </div>
 </div>
 )}

 {/* Convert */}
 {tab === "convert" && (
 <div className="panel-body" style={{ padding: 12, display: "flex", flexDirection: "column", gap: 12 }}>
 <div style={{ display: "flex", gap: 8 }}>
 {(["JSON", "SQL", "Markdown"] as const).map(fmt => (
 <button key={fmt}
 onClick={() => {
 let out = "";
 if (fmt === "JSON") out = convertToJson();
 else if (fmt === "SQL") out = convertToSql();
 else {
 const headerRow = hasHeader ? `| ${headers.join(" | ")} |\n| ${headers.map(() => "---").join(" | ")} |\n` : "";
 out = headerRow + sortedRows.map(r => `| ${r.join(" | ")} |`).join("\n");
 }
 navigator.clipboard.writeText(out);
 }}
 style={{ padding: "4px 12px", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", cursor: "pointer", fontSize: "var(--font-size-base)", color: "var(--text-primary)" }}>
 Copy as {fmt}
 </button>
 ))}
 </div>
 <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
 <div>
 <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 4 }}>JSON</div>
 <pre style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: 8, fontSize: "var(--font-size-sm)", overflow: "auto", maxHeight: "calc(60vh - 200px)", margin: 0 }}>
 {convertToJson().slice(0, 2000)}{convertToJson().length > 2000 ? "\n…" : ""}
 </pre>
 </div>
 <div>
 <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 4 }}>SQL INSERT</div>
 <pre style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: 8, fontSize: "var(--font-size-sm)", overflow: "auto", maxHeight: "calc(60vh - 200px)", margin: 0 }}>
 {convertToSql().slice(0, 2000)}{convertToSql().length > 2000 ? "\n…" : ""}
 </pre>
 </div>
 <div>
 <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 4 }}>Markdown Table</div>
 <pre style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: 8, fontSize: "var(--font-size-sm)", overflow: "auto", maxHeight: "calc(60vh - 200px)", margin: 0 }}>
 {(() => {
 const headerRow = hasHeader ? `| ${headers.join(" | ")} |\n| ${headers.map(() => "---").join(" | ")} |\n` : "";
 const body = sortedRows.slice(0, 20).map(r => `| ${r.join(" | ")} |`).join("\n");
 return (headerRow + body).slice(0, 2000);
 })()}
 </pre>
 </div>
 </div>
 </div>
 )}
 </>
 )}
 </div>
 );
}
