/**
 * CidrPanel — CIDR / Subnet Calculator.
 *
 * Tabs:
 * Calculator : IP + prefix → network, broadcast, mask, host count, binary view.
 * Split : divide a network into N equal sub-nets or /<newPrefix> blocks.
 * Reference : private ranges, AWS VPC defaults, common /prefix cheat-sheet.
 *
 * Supports IPv4 only. Pure TypeScript — no Tauri commands required.
 */
import { useState, useMemo } from "react";
import { CopyButton as CopyBtn } from "./shared/CopyButton";

// ── IPv4 math ──────────────────────────────────────────────────────────────────

function ipToInt(ip: string): number | null {
 const parts = ip.trim().split(".");
 if (parts.length !== 4) return null;
 let n = 0;
 for (const p of parts) {
 const v = parseInt(p, 10);
 if (isNaN(v) || v < 0 || v > 255) return null;
 n = (n << 8) | v;
 }
 return n >>> 0;
}

function intToIp(n: number): string {
 return [(n >>> 24) & 255, (n >>> 16) & 255, (n >>> 8) & 255, n & 255].join(".");
}

function intToBin(n: number): string {
 return n.toString(2).padStart(32, "0");
}

function subnetMask(prefix: number): number {
 return prefix === 0 ? 0 : (0xFFFFFFFF << (32 - prefix)) >>> 0;
}

function wildcardMask(prefix: number): number {
 return (~subnetMask(prefix)) >>> 0;
}

interface SubnetInfo {
 network: string;
 broadcast: string;
 firstHost: string;
 lastHost: string;
 mask: string;
 wildcard: string;
 prefix: number;
 hostCount: number;
 cidr: string;
 networkInt: number;
 maskInt: number;
}

function calcSubnet(ip: string, prefix: number): SubnetInfo | null {
 const ipInt = ipToInt(ip);
 if (ipInt === null || prefix < 0 || prefix > 32) return null;
 const maskInt = subnetMask(prefix);
 const networkInt = (ipInt & maskInt) >>> 0;
 const broadcastInt = (networkInt | wildcardMask(prefix)) >>> 0;
 const firstHost = prefix < 31 ? intToIp((networkInt + 1) >>> 0) : intToIp(networkInt);
 const lastHost = prefix < 31 ? intToIp((broadcastInt - 1) >>> 0) : intToIp(broadcastInt);
 const hostCount = prefix >= 31 ? Math.pow(2, 32 - prefix) : Math.pow(2, 32 - prefix) - 2;
 return {
 network: intToIp(networkInt),
 broadcast: intToIp(broadcastInt),
 firstHost, lastHost,
 mask: intToIp(maskInt),
 wildcard: intToIp(wildcardMask(prefix)),
 prefix, hostCount: Math.max(0, hostCount),
 cidr: `${intToIp(networkInt)}/${prefix}`,
 networkInt, maskInt,
 };
}

// ── Parse CIDR input ───────────────────────────────────────────────────────────

function parseCidr(s: string): { ip: string; prefix: number } | null {
 const parts = s.trim().split("/");
 if (parts.length !== 2) return null;
 const prefix = parseInt(parts[1], 10);
 if (isNaN(prefix) || prefix < 0 || prefix > 32) return null;
 if (ipToInt(parts[0]) === null) return null;
 return { ip: parts[0].trim(), prefix };
}

// ── Subnet splitting ───────────────────────────────────────────────────────────

function splitSubnet(base: SubnetInfo, newPrefix: number): SubnetInfo[] {
 if (newPrefix <= base.prefix || newPrefix > 32) return [];
 const count = Math.pow(2, newPrefix - base.prefix);
 const size = Math.pow(2, 32 - newPrefix);
 const result: SubnetInfo[] = [];
 for (let i = 0; i < Math.min(count, 256); i++) {
 const net = ((base.networkInt + i * size) >>> 0);
 const info = calcSubnet(intToIp(net), newPrefix);
 if (info) result.push(info);
 }
 return result;
}

// ── Reference data ────────────────────────────────────────────────────────────

const PRIVATE_RANGES = [
 { range: "10.0.0.0/8", name: "Class A Private (RFC 1918)", hosts: "16,777,214", use: "Large enterprise LANs" },
 { range: "172.16.0.0/12", name: "Class B Private (RFC 1918)", hosts: "1,048,574", use: "Medium enterprise networks" },
 { range: "192.168.0.0/16", name: "Class C Private (RFC 1918)", hosts: "65,534", use: "Home/small office networks" },
 { range: "127.0.0.0/8", name: "Loopback", hosts: "16,777,214", use: "localhost (127.0.0.1)" },
 { range: "169.254.0.0/16", name: "APIPA / Link-local", hosts: "65,534", use: "Auto-configured when DHCP fails" },
 { range: "100.64.0.0/10", name: "Shared Address (RFC 6598)", hosts: "4,194,302", use: "Carrier-grade NAT" },
 { range: "0.0.0.0/8", name: "This network", hosts: "—", use: "Unroutable / unspecified source" },
 { range: "240.0.0.0/4", name: "Reserved (Class E)", hosts: "268,435,454",use: "Future / experimental use" },
];

const CLOUD_DEFAULTS = [
 { provider: "AWS", cidr: "172.31.0.0/16", note: "Default VPC" },
 { provider: "AWS", cidr: "10.0.0.0/16", note: "Recommended custom VPC" },
 { provider: "GCP", cidr: "10.128.0.0/9", note: "Auto-mode VPC (all regions)" },
 { provider: "Azure", cidr: "10.0.0.0/16", note: "Default VNet" },
 { provider: "k8s", cidr: "10.244.0.0/16", note: "Flannel pod network (default)" },
 { provider: "k8s", cidr: "10.96.0.0/12", note: "Service cluster IP range (default)" },
 { provider: "Docker",cidr: "172.17.0.0/16", note: "Default bridge network" },
];

const PREFIX_TABLE = [
 ["/8", "16,777,214", "255.0.0.0", "Class A"],
 ["/16", "65,534", "255.255.0.0", "Class B"],
 ["/24", "254", "255.255.255.0", "Common LAN"],
 ["/25", "126", "255.255.255.128", "Half a /24"],
 ["/26", "62", "255.255.255.192", "Quarter /24"],
 ["/27", "30", "255.255.255.224", "Small segment"],
 ["/28", "14", "255.255.255.240", "Very small"],
 ["/29", "6", "255.255.255.248", "Point-to-point LAN"],
 ["/30", "2", "255.255.255.252", "P2P link"],
 ["/31", "2*", "255.255.255.254", "P2P (RFC 3021, no bcast)"],
 ["/32", "1", "255.255.255.255", "Host route / single IP"],
];

// ── Component ──────────────────────────────────────────────────────────────────

type SubTab = "calc" | "split" | "reference";

// CopyBtn imported from shared/CopyButton.tsx

function InfoRow({ label, value, mono = true, colour }: { label: string; value: string; mono?: boolean; colour?: string }) {
 return (
 <div style={{ display: "flex", alignItems: "center", borderBottom: "1px solid var(--border-subtle)", padding: "5px 12px", gap: 10 }}>
 <span style={{ width: 140, flexShrink: 0, fontSize: 10, fontWeight: 700, color: "var(--text-muted)" }}>{label}</span>
 <span style={{ flex: 1, fontFamily: mono ? "var(--font-mono)" : "inherit", fontSize: 12, color: colour ?? "var(--text-primary)", wordBreak: "break-all" }}>{value}</span>
 <CopyBtn text={value} />
 </div>
 );
}

export function CidrPanel() {
 const [subTab, setSubTab] = useState<SubTab>("calc");
 const [cidrInput, setCidrInput] = useState("192.168.1.100/24");
 const [splitPrefix, setSplitPrefix] = useState(26);

 const parsed = useMemo(() => parseCidr(cidrInput), [cidrInput]);
 const info = useMemo(() => parsed ? calcSubnet(parsed.ip, parsed.prefix) : null, [parsed]);

 const splitResults = useMemo(() => {
 if (!info || splitPrefix <= info.prefix) return [];
 return splitSubnet(info, splitPrefix);
 }, [info, splitPrefix]);

 // Binary view with network bits highlighted
 const BinaryView = ({ value, prefix }: { value: number; prefix: number }) => {
 const bits = intToBin(value);
 return (
 <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, letterSpacing: "0.05em" }}>
 {bits.split("").map((bit, i) => (
 <span key={i}>
 {i > 0 && i % 8 === 0 && <span style={{ color: "var(--border-color)", margin: "0 1px" }}>.</span>}
 <span style={{ color: i < prefix ? "var(--accent-color)" : "var(--success-color)" }}>{bit}</span>
 </span>
 ))}
 </span>
 );
 };

 const TABS: { id: SubTab; label: string }[] = [
 { id: "calc", label: "Calculator" },
 { id: "split", label: "Split" },
 { id: "reference", label: "Reference" },
 ];

 return (
 <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>

 {/* Header */}
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 6, alignItems: "center", flexWrap: "wrap" }}>
 <span style={{ fontSize: 13, fontWeight: 600 }}>CIDR Calculator</span>
 {TABS.map(t => (
 <button key={t.id} onClick={() => setSubTab(t.id)} style={{ padding: "2px 10px", fontSize: 10, borderRadius: 10, background: subTab === t.id ? "color-mix(in srgb, var(--accent-blue) 20%, transparent)" : "var(--bg-primary)", border: `1px solid ${subTab === t.id ? "var(--accent-color)" : "var(--border-color)"}`, color: subTab === t.id ? "var(--accent-color)" : "var(--text-muted)", cursor: "pointer", fontWeight: subTab === t.id ? 700 : 400 }}>{t.label}</button>
 ))}
 </div>

 {/* CIDR input (shared across calc + split tabs) */}
 {subTab !== "reference" && (
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
 <span style={{ fontSize: 11, color: "var(--text-muted)", flexShrink: 0 }}>CIDR:</span>
 <input value={cidrInput} onChange={e => setCidrInput(e.target.value)} spellCheck={false} placeholder="192.168.1.0/24"
 style={{ flex: 1, minWidth: 160, padding: "5px 10px", fontSize: 13, fontFamily: "var(--font-mono)", background: (!info && cidrInput.trim()) ? "color-mix(in srgb, var(--accent-rose) 8%, transparent)" : "var(--bg-primary)", border: `1px solid ${(!info && cidrInput.trim()) ? "var(--error-color)" : "var(--border-color)"}`, borderRadius: 4, color: "var(--text-primary)", outline: "none" }} />
 {/* Quick presets */}
 {["10.0.0.0/8","172.16.0.0/12","192.168.0.0/16","10.0.1.0/24"].map(p => (
 <button key={p} onClick={() => setCidrInput(p)} style={{ fontSize: 9, fontFamily: "var(--font-mono)", padding: "2px 6px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer" }}>{p}</button>
 ))}
 </div>
 )}

 <div style={{ flex: 1, overflow: "auto" }}>

 {/* ── CALCULATOR ── */}
 {subTab === "calc" && (
 <div>
 {!info && cidrInput.trim() && (
 <div style={{ padding: "8px 12px", fontSize: 11, color: "var(--text-danger)" }}>Invalid CIDR. Format: x.x.x.x/prefix (e.g. 192.168.1.0/24)</div>
 )}
 {info && (
 <div>
 <InfoRow label="CIDR Notation" value={info.cidr} colour="var(--accent-color)" />
 <InfoRow label="Network Address" value={info.network} colour="var(--success-color)" />
 <InfoRow label="Broadcast" value={info.broadcast} colour="var(--error-color)" />
 <InfoRow label="First Host" value={info.firstHost} />
 <InfoRow label="Last Host" value={info.lastHost} />
 <InfoRow label="Usable Hosts" value={info.hostCount.toLocaleString()} />
 <InfoRow label="Subnet Mask" value={info.mask} />
 <InfoRow label="Wildcard Mask" value={info.wildcard} />
 <InfoRow label="Prefix Length" value={`/${info.prefix}`} />
 <InfoRow label="Total Addresses" value={Math.pow(2, 32 - info.prefix).toLocaleString()} />

 {/* Binary view */}
 <div style={{ padding: "10px 12px", borderTop: "1px solid var(--border-color)" }}>
 <div style={{ fontSize: 10, fontWeight: 700, color: "var(--text-muted)", marginBottom: 8, letterSpacing: "0.05em" }}>
 BINARY REPRESENTATION &nbsp;
 <span style={{ color: "var(--text-info)", fontWeight: 400 }}>■ network bits</span>
 <span style={{ color: "var(--text-success)", fontWeight: 400, marginLeft: 8 }}>■ host bits</span>
 </div>
 {[
 { label: "IP Address", val: ipToInt(parsed!.ip)! },
 { label: "Network", val: info.networkInt },
 { label: "Subnet Mask", val: info.maskInt },
 { label: "Broadcast", val: ipToInt(info.broadcast)! },
 ].map(({ label, val }) => (
 <div key={label} style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 4 }}>
 <span style={{ width: 110, fontSize: 10, color: "var(--text-muted)", flexShrink: 0 }}>{label}</span>
 <BinaryView value={val} prefix={info.prefix} />
 </div>
 ))}
 </div>

 {/* Is this a private range? */}
 <div style={{ padding: "6px 12px", borderTop: "1px solid var(--border-color)", fontSize: 11, color: "var(--text-muted)" }}>
 {PRIVATE_RANGES.some(r => {
 const p = parseCidr(r.range);
 if (!p) return false;
 const base = calcSubnet(p.ip, p.prefix);
 if (!base) return false;
 return (info.networkInt & base.maskInt) === base.networkInt && info.prefix >= p.prefix;
 })
 ? <span style={{ color: "var(--text-warning-alt)" }}>This range falls within a private (RFC 1918 / reserved) address space.</span>
 : <span style={{ color: "var(--text-success)" }}>✓ Public address space.</span>
 }
 </div>
 </div>
 )}
 </div>
 )}

 {/* ── SPLIT ── */}
 {subTab === "split" && (
 <div style={{ display: "flex", flexDirection: "column" }}>
 {info && (
 <>
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
 <span style={{ fontSize: 11, color: "var(--text-muted)" }}>Split into /<span style={{ color: "var(--text-info)", fontFamily: "var(--font-mono)" }}>{splitPrefix}</span> blocks</span>
 <input type="range" min={info.prefix + 1} max={32} value={splitPrefix} onChange={e => setSplitPrefix(+e.target.value)}
 style={{ flex: 1, minWidth: 80, accentColor: "var(--accent-color)" }} />
 <span style={{ fontSize: 11, fontFamily: "var(--font-mono)", color: "var(--text-muted)" }}>
 {Math.pow(2, splitPrefix - info.prefix).toLocaleString()} subnets × {Math.max(0, Math.pow(2, 32 - splitPrefix) - 2).toLocaleString()} hosts
 </span>
 </div>
 <div style={{ overflow: "auto" }}>
 {splitResults.length === 0
 ? <div style={{ padding: 12, color: "var(--text-muted)", fontSize: 12 }}>Select a prefix larger than /{info.prefix}.</div>
 : splitResults.map((s, i) => (
 <div key={i} style={{ display: "flex", gap: 8, padding: "5px 12px", borderBottom: "1px solid var(--border-subtle)", alignItems: "center", fontSize: 11 }}>
 <span style={{ width: 24, color: "var(--text-muted)", fontFamily: "var(--font-mono)", flexShrink: 0 }}>{i + 1}</span>
 <span style={{ fontFamily: "var(--font-mono)", color: "var(--text-info)", width: 160, flexShrink: 0 }}>{s.cidr}</span>
 <span style={{ color: "var(--text-success)", width: 110, flexShrink: 0 }}>first: {s.firstHost}</span>
 <span style={{ color: "var(--text-danger)", flex: 1 }}>last: {s.lastHost}</span>
 <CopyBtn text={s.cidr} />
 </div>
 ))
 }
 {splitResults.length === 256 && (
 <div style={{ padding: "6px 12px", fontSize: 10, color: "var(--text-muted)", fontStyle: "italic" }}>Showing first 256 subnets.</div>
 )}
 </div>
 </>
 )}
 {!info && cidrInput.trim() && <div style={{ padding: 12, color: "var(--text-danger)", fontSize: 12 }}>Fix the CIDR input above first.</div>}
 </div>
 )}

 {/* ── REFERENCE ── */}
 {subTab === "reference" && (
 <div style={{ padding: "12px", display: "flex", flexDirection: "column", gap: 16 }}>

 {/* Private ranges */}
 <div>
 <div style={{ fontSize: 11, fontWeight: 700, color: "var(--text-info)", marginBottom: 8 }}>PRIVATE & RESERVED RANGES (IPv4)</div>
 <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11 }}>
 <thead>
 <tr style={{ background: "var(--bg-secondary)" }}>
 {["Range","Name","Usable Hosts","Use Case"].map(h => (
 <th key={h} style={{ padding: "5px 8px", textAlign: "left", fontSize: 10, fontWeight: 700, color: "var(--text-muted)", borderBottom: "1px solid var(--border-color)" }}>{h}</th>
 ))}
 </tr>
 </thead>
 <tbody>
 {PRIVATE_RANGES.map(r => (
 <tr key={r.range} style={{ borderBottom: "1px solid var(--border-subtle)", cursor: "pointer" }} onClick={() => { setCidrInput(r.range); setSubTab("calc"); }}>
 <td style={{ padding: "5px 8px", fontFamily: "var(--font-mono)", color: "var(--text-warning-alt)" }}>{r.range}</td>
 <td style={{ padding: "5px 8px", color: "var(--text-primary)" }}>{r.name}</td>
 <td style={{ padding: "5px 8px", fontFamily: "var(--font-mono)", color: "var(--text-muted)" }}>{r.hosts}</td>
 <td style={{ padding: "5px 8px", color: "var(--text-muted)", fontSize: 10 }}>{r.use}</td>
 </tr>
 ))}
 </tbody>
 </table>
 <div style={{ fontSize: 10, color: "var(--text-muted)", marginTop: 4, fontStyle: "italic" }}>Click any row to load it into the Calculator.</div>
 </div>

 {/* Cloud defaults */}
 <div>
 <div style={{ fontSize: 11, fontWeight: 700, color: "var(--text-success)", marginBottom: 8 }}>CLOUD & CONTAINER DEFAULTS</div>
 <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11 }}>
 <thead>
 <tr style={{ background: "var(--bg-secondary)" }}>
 {["Provider","CIDR","Note"].map(h => (
 <th key={h} style={{ padding: "5px 8px", textAlign: "left", fontSize: 10, fontWeight: 700, color: "var(--text-muted)", borderBottom: "1px solid var(--border-color)" }}>{h}</th>
 ))}
 </tr>
 </thead>
 <tbody>
 {CLOUD_DEFAULTS.map((r, i) => (
 <tr key={i} style={{ borderBottom: "1px solid var(--border-subtle)", cursor: "pointer" }} onClick={() => { setCidrInput(r.cidr); setSubTab("calc"); }}>
 <td style={{ padding: "5px 8px", color: "var(--text-accent)", fontWeight: 700 }}>{r.provider}</td>
 <td style={{ padding: "5px 8px", fontFamily: "var(--font-mono)", color: "var(--text-info)" }}>{r.cidr}</td>
 <td style={{ padding: "5px 8px", color: "var(--text-muted)" }}>{r.note}</td>
 </tr>
 ))}
 </tbody>
 </table>
 </div>

 {/* Prefix cheat-sheet */}
 <div>
 <div style={{ fontSize: 11, fontWeight: 700, color: "var(--text-warning-alt)", marginBottom: 8 }}>PREFIX LENGTH CHEAT SHEET</div>
 <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11 }}>
 <thead>
 <tr style={{ background: "var(--bg-secondary)" }}>
 {["Prefix","Usable Hosts","Subnet Mask","Notes"].map(h => (
 <th key={h} style={{ padding: "5px 8px", textAlign: "left", fontSize: 10, fontWeight: 700, color: "var(--text-muted)", borderBottom: "1px solid var(--border-color)" }}>{h}</th>
 ))}
 </tr>
 </thead>
 <tbody>
 {PREFIX_TABLE.map(([prefix, hosts, mask, notes]) => (
 <tr key={prefix} style={{ borderBottom: "1px solid var(--border-subtle)" }}>
 <td style={{ padding: "5px 8px", fontFamily: "var(--font-mono)", fontWeight: 700, color: "var(--text-warning-alt)" }}>{prefix}</td>
 <td style={{ padding: "5px 8px", fontFamily: "var(--font-mono)", color: "var(--text-success)" }}>{hosts}</td>
 <td style={{ padding: "5px 8px", fontFamily: "var(--font-mono)", color: "var(--text-muted)" }}>{mask}</td>
 <td style={{ padding: "5px 8px", color: "var(--text-muted)", fontSize: 10 }}>{notes}</td>
 </tr>
 ))}
 </tbody>
 </table>
 </div>

 </div>
 )}

 </div>
 </div>
 );
}
