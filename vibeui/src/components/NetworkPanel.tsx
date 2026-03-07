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
 LISTEN: "#a6e3a1",
 ESTABLISHED: "#89b4fa",
 OPEN: "#89dceb",
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
 host: tlsHost.trim().replace(/^https?:\/\//, "").split("/")[0],
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
 padding: "6px 16px", fontSize: 11, fontWeight: tool === id ? 600 : 400,
 background: tool === id ? "rgba(99,102,241,0.15)" : "transparent",
 color: tool === id ? "var(--accent-color, #6366f1)" : "var(--text-muted)",
 border: "none", borderBottom: tool === id ? "2px solid var(--accent-color, #6366f1)" : "2px solid transparent",
 cursor: "pointer",
 }}
 >
 {label}
 </button>
 );

 const BTN = (label: string, onClick: () => void, loading: boolean, disabled?: boolean) => (
 <button
 onClick={onClick}
 disabled={loading || disabled}
 style={{
 padding: "6px 16px", fontSize: 11, fontWeight: 600,
 background: loading || disabled ? "var(--bg-secondary)" : "var(--accent-color, #6366f1)",
 color: loading || disabled ? "var(--text-muted)" : "var(--text-primary, #fff)",
 border: "none", borderRadius: 4, cursor: loading || disabled ? "not-allowed" : "pointer",
 }}
 >
 {loading ? "…" : label}
 </button>
 );

 const ERR = (msg: string | null) => msg && (
 <div style={{ padding: "6px 10px", background: "var(--error-bg,#2a1a1a)", color: "var(--text-danger, #f38ba8)", borderRadius: 4, fontSize: 11 }}>
 {msg}
 </div>
 );

 return (
 <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
 {/* Header */}
 <div style={{ padding: "10px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0, display: "flex", alignItems: "center", gap: 8 }}>
 <span style={{ fontSize: 16 }}></span>
 <div style={{ fontSize: 13, fontWeight: 600 }}>Network Tools</div>
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
 <div style={{ padding: "10px 12px", borderBottom: "1px solid var(--border-color)", flexShrink: 0, display: "flex", gap: 8, alignItems: "center" }}>
 <span style={{ fontSize: 11, color: "var(--text-muted)" }}>Localhost open ports via <code>lsof -i</code></span>
 {BTN(ports.length ? "↻ Refresh" : "Scan Ports", scanPorts, scanningPorts)}
 {ports.length > 0 && (
 <input
 value={portFilter}
 onChange={(e) => setPortFilter(e.target.value)}
 placeholder="Filter by port / process / state…"
 style={{ marginLeft: "auto", padding: "4px 8px", fontSize: 11, background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none", width: 220 }}
 />
 )}
 </div>
 <div style={{ flex: 1, overflow: "auto", padding: "8px 12px", display: "flex", flexDirection: "column", gap: 4 }}>
 {ERR(portError)}
 {ports.length === 0 && !scanningPorts && !portError && (
 <div style={{ textAlign: "center", padding: "40px 0", color: "var(--text-muted)", fontSize: 12 }}>
 Click "Scan Ports" to list open ports on localhost.
 </div>
 )}
 {filteredPorts.length > 0 && (
 <>
 <div style={{ display: "grid", gridTemplateColumns: "70px 60px 140px 1fr 90px", gap: 8, padding: "4px 8px", fontSize: 10, fontWeight: 600, color: "var(--text-muted)", borderBottom: "1px solid var(--border-color)" }}>
 <span>Port</span><span>Proto</span><span>Process (PID)</span><span>Address</span><span>State</span>
 </div>
 {filteredPorts.map((p, i) => (
 <div key={i} style={{ display: "grid", gridTemplateColumns: "70px 60px 140px 1fr 90px", gap: 8, padding: "5px 8px", fontSize: 11, borderBottom: "1px solid var(--border-color)", alignItems: "center" }}>
 <span style={{ fontFamily: "monospace", fontWeight: 700, color: "var(--text-info, #89b4fa)" }}>{p.port}</span>
 <span style={{ fontSize: 10, fontWeight: 600, color: p.protocol === "tcp" ? "#cba6f7" : "#f9e2af" }}>{p.protocol.toUpperCase()}</span>
 <span style={{ fontFamily: "monospace", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={`${p.process ?? ""} (${p.pid ?? "?"})`}>
 {p.process ?? "—"}{p.pid ? ` (${p.pid})` : ""}
 </span>
 <span style={{ fontFamily: "monospace", fontSize: 10, color: "var(--text-muted)" }}>{p.address}</span>
 <span style={{ fontSize: 10, fontWeight: 600, color: STATE_COLORS[p.state] ?? "var(--text-muted)", padding: "1px 6px", background: `${STATE_COLORS[p.state] ?? "#888"}22`, borderRadius: 10 }}>{p.state}</span>
 </div>
 ))}
 <div style={{ fontSize: 10, color: "var(--text-muted)", padding: "6px 8px" }}>
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
 style={{ flex: 1, padding: "6px 10px", fontSize: 12, fontFamily: "monospace", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none" }}
 />
 <select
 value={dnsType}
 onChange={(e) => setDnsType(e.target.value)}
 style={{ padding: "6px 8px", fontSize: 11, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none", fontWeight: 600 }}
 >
 {DNS_TYPES.map((t) => <option key={t}>{t}</option>)}
 </select>
 {BTN("Lookup", lookupDns, dnsLoading, !dnsDomain.trim())}
 </div>

 {/* Quick domains */}
 <div style={{ display: "flex", gap: 5, flexWrap: "wrap" }}>
 {["google.com", "cloudflare.com", "github.com", "localhost"].map((d) => (
 <button key={d} onClick={() => setDnsDomain(d)} style={{ padding: "2px 8px", fontSize: 10, borderRadius: 10, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", color: "var(--text-muted)", cursor: "pointer" }}>{d}</button>
 ))}
 </div>

 {ERR(dnsError)}

 {dnsRecords.length > 0 && (
 <div style={{ display: "flex", flexDirection: "column", gap: 3 }}>
 <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-muted)", marginBottom: 2 }}>
 {dnsRecords.length} {dnsType} record{dnsRecords.length !== 1 ? "s" : ""} for {dnsDomain}
 </div>
 {dnsRecords.map((r, i) => (
 <div key={i} style={{ display: "flex", alignItems: "center", gap: 10, padding: "6px 10px", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4 }}>
 <span style={{ fontSize: 10, fontWeight: 700, color: "var(--text-info, #89b4fa)", width: 40 }}>{r.record_type}</span>
 <span style={{ fontFamily: "monospace", fontSize: 12, flex: 1 }}>{r.value}</span>
 <button onClick={() => navigator.clipboard.writeText(r.value).catch(() => {})} style={{ padding: "2px 6px", fontSize: 10, background: "none", border: "1px solid var(--border-color)", borderRadius: 3, color: "var(--text-muted)", cursor: "pointer" }}>Copy</button>
 </div>
 ))}
 </div>
 )}

 {dnsRecords.length === 0 && !dnsLoading && !dnsError && dnsDomain && (
 <div style={{ textAlign: "center", padding: "20px 0", color: "var(--text-muted)", fontSize: 12 }}>
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
 style={{ flex: 1, padding: "6px 10px", fontSize: 12, fontFamily: "monospace", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none" }}
 />
 <input
 type="number" value={tlsPort} min={1} max={65535}
 onChange={(e) => setTlsPort(Number(e.target.value))}
 style={{ width: 72, padding: "6px 8px", fontSize: 12, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none", textAlign: "center" }}
 />
 {BTN("Check", checkTls, tlsLoading, !tlsHost.trim())}
 </div>

 {/* Quick hosts */}
 <div style={{ display: "flex", gap: 5, flexWrap: "wrap" }}>
 {["google.com", "github.com", "cloudflare.com", "example.com"].map((h) => (
 <button key={h} onClick={() => { setTlsHost(h); setTlsPort(443); }} style={{ padding: "2px 8px", fontSize: 10, borderRadius: 10, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", color: "var(--text-muted)", cursor: "pointer" }}>{h}</button>
 ))}
 </div>

 {ERR(tlsError)}

 {tlsCert && (
 <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
 {/* Status banner */}
 <div style={{
 padding: "10px 14px", borderRadius: 6, display: "flex", justifyContent: "space-between", alignItems: "center",
 background: tlsCert.valid ? "rgba(166,227,161,0.1)" : "rgba(243,139,168,0.1)",
 border: `1px solid ${tlsCert.valid ? "var(--success-color, #a6e3a1)" : "var(--error-color, #f38ba8)"}`,
 }}>
 <div>
 <span style={{ fontSize: 14, fontWeight: 700, color: tlsCert.valid ? "var(--success-color, #a6e3a1)" : "var(--error-color, #f38ba8)" }}>
 {tlsCert.valid ? "Valid" : "Invalid / Expired"}
 </span>
 <span style={{ marginLeft: 10, fontSize: 11, color: "var(--text-muted)" }}>{tlsHost}:{tlsPort}</span>
 </div>
 <div style={{ textAlign: "right" }}>
 <div style={{ fontSize: 20, fontWeight: 700, color: tlsCert.days_remaining > 30 ? "var(--success-color, #a6e3a1)" : tlsCert.days_remaining > 7 ? "var(--warning-color, #f9e2af)" : "var(--error-color, #f38ba8)" }}>
 {tlsCert.days_remaining}d
 </div>
 <div style={{ fontSize: 9, color: "var(--text-muted)" }}>remaining</div>
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
 <div key={label} style={{ display: "flex", gap: 10, padding: "6px 10px", background: "var(--bg-secondary)", borderRadius: 4, border: "1px solid var(--border-color)", alignItems: "flex-start" }}>
 <span style={{ fontSize: 10, fontWeight: 600, color: "var(--text-muted)", width: 90, flexShrink: 0 }}>{label}</span>
 <span style={{ fontFamily: "monospace", fontSize: 11, flex: 1, wordBreak: "break-all" }}>{value}</span>
 </div>
 ))}

 {/* SAN list */}
 {tlsCert.san.length > 0 && (
 <div>
 <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 5 }}>
 Subject Alternative Names ({tlsCert.san.length})
 </div>
 <div style={{ display: "flex", flexWrap: "wrap", gap: 5 }}>
 {tlsCert.san.map((s) => (
 <span key={s} style={{ padding: "2px 8px", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 10, fontSize: 11, fontFamily: "monospace" }}>{s}</span>
 ))}
 </div>
 </div>
 )}

 {/* Raw output toggle */}
 <div>
 <button onClick={() => setShowRawCert((p) => !p)} style={{ fontSize: 11, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer", padding: 0 }}>
 {showRawCert ? "Hide raw output" : "▼ Show raw openssl output"}
 </button>
 {showRawCert && (
 <pre style={{ margin: "6px 0 0", padding: 8, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, fontSize: 10, lineHeight: 1.4, overflow: "auto", maxHeight: 200, whiteSpace: "pre-wrap", color: "var(--text-muted)" }}>
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
