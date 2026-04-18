/**
 * NetworkPanel — Network Tools.
 *
 * Three sub-tools:
 * 1. Port Scanner – list open ports on localhost via lsof/netstat
 * 2. DNS Lookup – resolve records via dig/host
 * 3. TLS Inspector – SSL cert details, expiry, SAN via openssl s_client
 */
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ChevronDown } from "lucide-react";

interface OpenPort {
 port: number;
 protocol: string;
 pid: number | null;
 process: string | null;
 state: string;
 address: string;
}

interface DnsRecord {
 record_type: string;
 value: string;
 ttl: number | null;
}

interface TlsCertInfo {
 subject: string;
 issuer: string;
 not_before: string;
 not_after: string;
 san: string[];
 serial: string;
 valid: boolean;
 days_remaining: number;
 raw: string;
}

type Tool = "ports" | "dns" | "tls";

const DNS_TYPES = ["A", "AAAA", "CNAME", "MX", "TXT", "NS", "SOA", "PTR", "SRV", "ANY"];

const STATE_COLORS: Record<string, string> = {
 LISTEN: "var(--success-color)",
 ESTABLISHED: "var(--accent-color)",
 OPEN: "var(--accent-color)",
};

export function NetworkPanel() {
 const [tool, setTool] = useState<Tool>("ports");

 // Port scanner state
 const [ports, setPorts] = useState<OpenPort[]>([]);
 const [portFilter, setPortFilter] = useState("");
 const [scanningPorts, setScanningPorts] = useState(false);
 const [portError, setPortError] = useState<string | null>(null);

 // DNS state
 const [dnsDomain, setDnsDomain] = useState("");
 const [dnsType, setDnsType] = useState("A");
 const [dnsRecords, setDnsRecords] = useState<DnsRecord[]>([]);
 const [dnsLoading, setDnsLoading] = useState(false);
 const [dnsError, setDnsError] = useState<string | null>(null);

 // TLS state
 const [tlsHost, setTlsHost] = useState("");
 const [tlsPort, setTlsPort] = useState(443);
 const [tlsCert, setTlsCert] = useState<TlsCertInfo | null>(null);
 const [tlsLoading, setTlsLoading] = useState(false);
 const [tlsError, setTlsError] = useState<string | null>(null);
 const [showRawCert, setShowRawCert] = useState(false);

 // ── Port Scanner ──────────────────────────────────────────────────────────
 const scanPorts = async () => {
 setScanningPorts(true);
 setPortError(null);
 try {
 const result = await invoke<OpenPort[]>("scan_open_ports", { host: null });
 setPorts(result);
 } catch (e) {
 setPortError(String(e));
 } finally {
 setScanningPorts(false);
 }
 };

 const filteredPorts = ports.filter((p) => {
 if (!portFilter) return true;
 const f = portFilter.toLowerCase();
 return String(p.port).includes(f)
 || (p.process ?? "").toLowerCase().includes(f)
 || p.state.toLowerCase().includes(f)
 || p.protocol.includes(f);
 });

 // ── DNS Lookup ────────────────────────────────────────────────────────────
 const lookupDns = async () => {
 if (!dnsDomain.trim()) return;
 setDnsLoading(true);
 setDnsError(null);
 setDnsRecords([]);
 try {
 const result = await invoke<DnsRecord[]>("dns_lookup", {
 domain: dnsDomain.trim(),
 recordType: dnsType,
 });
 setDnsRecords(result);
 } catch (e) {
 setDnsError(String(e));
 } finally {
 setDnsLoading(false);
 }
 };

 // ── TLS Inspector ─────────────────────────────────────────────────────────
 const checkTls = async () => {
 if (!tlsHost.trim()) return;
 setTlsLoading(true);
 setTlsError(null);
 setTlsCert(null);
 try {
 const result = await invoke<TlsCertInfo>("check_tls_cert", {
 host: tlsHost.trim().replace(/^https?:\/\//i, "").split("/")[0],
 port: tlsPort,
 });
 setTlsCert(result);
 } catch (e) {
 setTlsError(String(e));
 } finally {
 setTlsLoading(false);
 }
 };

 // ── Styles ────────────────────────────────────────────────────────────────
 const TAB_BTN = (id: Tool, label: string) => (
 <button
 key={id}
 onClick={() => setTool(id)}
 style={{
 padding: "8px 16px", fontSize: "var(--font-size-sm)", fontWeight: tool === id ? 600 : 400,
 background: tool === id ? "color-mix(in srgb, var(--accent-blue) 15%, transparent)" : "transparent",
 color: tool === id ? "var(--text-primary)" : "var(--text-secondary)",
 border: "none", borderBottom: tool === id ? "2px solid var(--accent-blue)" : "2px solid transparent",
 cursor: "pointer",
 }}
 >
 {label}
 </button>
 );

 const BTN = (label: string, onClick: () => void, loading: boolean, disabled?: boolean) => (
 <button className="panel-btn"
 onClick={onClick}
 disabled={loading || disabled}
 style={{
 padding: "8px 16px", fontSize: "var(--font-size-sm)", fontWeight: 600,
 background: loading || disabled ? "var(--bg-secondary)" : "var(--accent-color)",
 color: loading || disabled ? "var(--text-secondary)" : "var(--text-primary)",
 border: "none", borderRadius: "var(--radius-xs-plus)", cursor: loading || disabled ? "not-allowed" : "pointer",
 }}
 >
 {loading ? "…" : label}
 </button>
 );

 const ERR = (msg: string | null) => msg && (
 <div style={{ padding: "8px 12px", background: "var(--error-bg)", color: "var(--text-danger)", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-sm)" }}>
 {msg}
 </div>
 );

 return (
 <div className="panel-container">
 {/* Header */}
 <div className="panel-header">
 <span style={{ fontSize: 16 }}></span>
 <h3>Network Tools</h3>
 </div>

 {/* Sub-tabs */}
 <div style={{ display: "flex", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0 }}>
 {TAB_BTN("ports", "Port Scanner")}
 {TAB_BTN("dns", "DNS Lookup")}
 {TAB_BTN("tls", "TLS Inspector")}
 </div>

 {/* ── Port Scanner ──────────────────────────────────────────────── */}
 {tool === "ports" && (
 <div style={{ flex: 1, overflow: "hidden", display: "flex", flexDirection: "column" }}>
 <div style={{ padding: "12px 12px", borderBottom: "1px solid var(--border-color)", flexShrink: 0, display: "flex", gap: 8, alignItems: "center" }}>
 <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Localhost open ports via <code>lsof -i</code></span>
 {BTN(ports.length ? "↻ Refresh" : "Scan Ports", scanPorts, scanningPorts)}
 {ports.length > 0 && (
 <input
 value={portFilter}
 onChange={(e) => setPortFilter(e.target.value)}
 placeholder="Filter by port / process / state…"
 style={{ marginLeft: "auto", padding: "4px 8px", fontSize: "var(--font-size-sm)", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none", width: 220 }}
 />
 )}
 </div>
 <div style={{ flex: 1, overflow: "auto", padding: "8px 12px", display: "flex", flexDirection: "column", gap: 4 }}>
 {ERR(portError)}
 {ports.length === 0 && !scanningPorts && !portError && (
 <div style={{ textAlign: "center", padding: "40px 0", color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>
 Click "Scan Ports" to list open ports on localhost.
 </div>
 )}
 {filteredPorts.length > 0 && (
 <>
 <div style={{ display: "grid", gridTemplateColumns: "70px 60px 140px 1fr 90px", gap: 8, padding: "4px 8px", fontSize: "var(--font-size-xs)", fontWeight: 600, color: "var(--text-secondary)", borderBottom: "1px solid var(--border-color)" }}>
 <span>Port</span><span>Proto</span><span>Process (PID)</span><span>Address</span><span>State</span>
 </div>
 {filteredPorts.map((p, i) => (
 <div key={i} style={{ display: "grid", gridTemplateColumns: "70px 60px 140px 1fr 90px", gap: 8, padding: "4px 8px", fontSize: "var(--font-size-sm)", borderBottom: "1px solid var(--border-color)", alignItems: "center" }}>
 <span style={{ fontFamily: "var(--font-mono)", fontWeight: 700, color: "var(--text-info)" }}>{p.port}</span>
 <span style={{ fontSize: "var(--font-size-xs)", fontWeight: 600, color: p.protocol === "tcp" ? "var(--accent-color)" : "var(--warning-color)" }}>{p.protocol.toUpperCase()}</span>
 <span style={{ fontFamily: "var(--font-mono)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={`${p.process ?? ""} (${p.pid ?? "?"})`}>
 {p.process ?? "—"}{p.pid ? ` (${p.pid})` : ""}
 </span>
 <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-xs)", color: "var(--text-secondary)" }}>{p.address}</span>
 <span style={{ fontSize: "var(--font-size-xs)", fontWeight: 600, color: STATE_COLORS[p.state] ?? "var(--text-secondary)", padding: "1px 8px", background: STATE_COLORS[p.state] ? `${STATE_COLORS[p.state]}22` : "var(--bg-secondary)", borderRadius: "var(--radius-md)" }}>{p.state}</span>
 </div>
 ))}
 <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", padding: "8px 8px" }}>
 {filteredPorts.length} of {ports.length} ports
 </div>
 </>
 )}
 </div>
 </div>
 )}

 {/* ── DNS Lookup ────────────────────────────────────────────────── */}
 {tool === "dns" && (
 <div style={{ flex: 1, overflow: "auto", padding: 12, display: "flex", flexDirection: "column", gap: 10 }}>
 <div style={{ display: "flex", gap: 6 }}>
 <input
 value={dnsDomain}
 onChange={(e) => setDnsDomain(e.target.value)}
 onKeyDown={(e) => e.key === "Enter" && lookupDns()}
 placeholder="example.com"
 style={{ flex: 1, padding: "8px 12px", fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono)", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none" }}
 />
 <select
 value={dnsType}
 onChange={(e) => setDnsType(e.target.value)}
 style={{ padding: "8px 8px", fontSize: "var(--font-size-sm)", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none", fontWeight: 600 }}
 >
 {DNS_TYPES.map((t) => <option key={t}>{t}</option>)}
 </select>
 {BTN("Lookup", lookupDns, dnsLoading, !dnsDomain.trim())}
 </div>

 {/* Quick domains */}
 <div style={{ display: "flex", gap: 5, flexWrap: "wrap" }}>
 {["google.com", "cloudflare.com", "github.com", "localhost"].map((d) => (
 <button key={d} onClick={() => setDnsDomain(d)} style={{ padding: "2px 8px", fontSize: "var(--font-size-xs)", borderRadius: "var(--radius-md)", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", color: "var(--text-secondary)", cursor: "pointer" }}>{d}</button>
 ))}
 </div>

 {ERR(dnsError)}

 {dnsRecords.length > 0 && (
 <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
 <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, color: "var(--text-secondary)", marginBottom: 2 }}>
 {dnsRecords.length} {dnsType} record{dnsRecords.length !== 1 ? "s" : ""} for {dnsDomain}
 </div>
 {dnsRecords.map((r, i) => (
 <div key={i} style={{ display: "flex", alignItems: "center", gap: 10, padding: "8px 12px", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)" }}>
 <span style={{ fontSize: "var(--font-size-xs)", fontWeight: 700, color: "var(--text-info)", width: 40 }}>{r.record_type}</span>
 <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-base)", flex: 1 }}>{r.value}</span>
 <button onClick={() => navigator.clipboard.writeText(r.value).catch(() => {})} style={{ padding: "2px 8px", fontSize: "var(--font-size-xs)", background: "none", border: "1px solid var(--border-color)", borderRadius: 3, color: "var(--text-secondary)", cursor: "pointer" }}>Copy</button>
 </div>
 ))}
 </div>
 )}

 {dnsRecords.length === 0 && !dnsLoading && !dnsError && dnsDomain && (
 <div style={{ textAlign: "center", padding: "20px 0", color: "var(--text-secondary)", fontSize: "var(--font-size-base)" }}>
 No records found for {dnsType} {dnsDomain}
 </div>
 )}
 </div>
 )}

 {/* ── TLS Inspector ─────────────────────────────────────────────── */}
 {tool === "tls" && (
 <div style={{ flex: 1, overflow: "auto", padding: 12, display: "flex", flexDirection: "column", gap: 10 }}>
 <div style={{ display: "flex", gap: 6 }}>
 <input
 value={tlsHost}
 onChange={(e) => setTlsHost(e.target.value)}
 onKeyDown={(e) => e.key === "Enter" && checkTls()}
 placeholder="example.com or https://example.com"
 style={{ flex: 1, padding: "8px 12px", fontSize: "var(--font-size-base)", fontFamily: "var(--font-mono)", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none" }}
 />
 <input
 type="number" value={tlsPort} min={1} max={65535}
 onChange={(e) => setTlsPort(Number(e.target.value))}
 style={{ width: 72, padding: "8px 8px", fontSize: "var(--font-size-base)", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", color: "var(--text-primary)", outline: "none", textAlign: "center" }}
 />
 {BTN("Check", checkTls, tlsLoading, !tlsHost.trim())}
 </div>

 {/* Quick hosts */}
 <div style={{ display: "flex", gap: 5, flexWrap: "wrap" }}>
 {["google.com", "github.com", "cloudflare.com", "example.com"].map((h) => (
 <button key={h} onClick={() => { setTlsHost(h); setTlsPort(443); }} style={{ padding: "2px 8px", fontSize: "var(--font-size-xs)", borderRadius: "var(--radius-md)", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", color: "var(--text-secondary)", cursor: "pointer" }}>{h}</button>
 ))}
 </div>

 {ERR(tlsError)}

 {tlsCert && (
 <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
 {/* Status banner */}
 <div style={{
 padding: "12px 16px", borderRadius: "var(--radius-sm)", display: "flex", justifyContent: "space-between", alignItems: "center",
 background: tlsCert.valid ? "color-mix(in srgb, var(--accent-green) 10%, transparent)" : "color-mix(in srgb, var(--accent-rose) 10%, transparent)",
 border: `1px solid ${tlsCert.valid ? "var(--success-color)" : "var(--error-color)"}`,
 }}>
 <div>
 <span style={{ fontSize: "var(--font-size-lg)", fontWeight: 700, color: tlsCert.valid ? "var(--success-color)" : "var(--error-color)" }}>
 {tlsCert.valid ? "Valid" : "Invalid / Expired"}
 </span>
 <span style={{ marginLeft: 10, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{tlsHost}:{tlsPort}</span>
 </div>
 <div style={{ textAlign: "right" }}>
 <div style={{ fontSize: 20, fontWeight: 700, fontFamily: "var(--font-mono)", color: tlsCert.days_remaining > 30 ? "var(--success-color)" : tlsCert.days_remaining > 7 ? "var(--warning-color)" : "var(--error-color)" }}>
 {tlsCert.days_remaining}d
 </div>
 <div style={{ fontSize: 9, color: "var(--text-secondary)" }}>remaining</div>
 </div>
 </div>

 {/* Cert details */}
 {[
 ["Subject", tlsCert.subject],
 ["Issuer", tlsCert.issuer],
 ["Valid From", tlsCert.not_before],
 ["Valid Until", tlsCert.not_after],
 ["Serial", tlsCert.serial],
 ].filter(([, v]) => v).map(([label, value]) => (
 <div key={label} style={{ display: "flex", gap: 10, padding: "8px 12px", background: "var(--bg-secondary)", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", alignItems: "flex-start" }}>
 <span style={{ fontSize: "var(--font-size-xs)", fontWeight: 600, color: "var(--text-secondary)", width: 90, flexShrink: 0 }}>{label}</span>
 <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)", flex: 1, wordBreak: "break-all" }}>{value}</span>
 </div>
 ))}

 {/* SAN list */}
 {tlsCert.san.length > 0 && (
 <div>
 <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, marginBottom: 5 }}>
 Subject Alternative Names ({tlsCert.san.length})
 </div>
 <div style={{ display: "flex", flexWrap: "wrap", gap: 5 }}>
 {tlsCert.san.map((s) => (
 <span key={s} style={{ padding: "2px 8px", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-md)", fontSize: "var(--font-size-sm)", fontFamily: "var(--font-mono)" }}>{s}</span>
 ))}
 </div>
 </div>
 )}

 {/* Raw output toggle */}
 <div>
 <button onClick={() => setShowRawCert((p) => !p)} style={{ fontSize: "var(--font-size-sm)", background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", padding: 0 }}>
 {showRawCert ? "Hide raw output" : <><ChevronDown size={10} /> Show raw openssl output</>}
 </button>
 {showRawCert && (
 <pre style={{ margin: "8px 0 0", padding: 8, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-xs)", lineHeight: 1.4, overflow: "auto", maxHeight: 200, whiteSpace: "pre-wrap", wordBreak: "break-word", color: "var(--text-secondary)" }}>
 {tlsCert.raw}
 </pre>
 )}
 </div>
 </div>
 )}
 </div>
 )}
 </div>
 );
}
