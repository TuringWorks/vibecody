/**
 * JwtPanel — JWT Debugger & Signer.
 *
 * Decode tab : paste a JWT → see formatted header + payload + expiry status.
 * Sign tab : build header/payload JSON + secret → generate HS256 JWT.
 * Pure TypeScript / Web Crypto — no Tauri commands required.
 */
import { useState, useMemo, useCallback } from "react";

// ── Base64url helpers ──────────────────────────────────────────────────────────

function b64urlDecode(s: string): string {
 const pad = s.replace(/-/g, "+").replace(/_/g, "/");
 const padded = pad + "=".repeat((4 - (pad.length % 4)) % 4);
 try { return atob(padded); } catch { return ""; }
}

function b64urlEncode(buf: ArrayBuffer): string {
 const bytes = new Uint8Array(buf);
 let bin = "";
 bytes.forEach(b => (bin += String.fromCharCode(b)));
 return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=/g, "");
}

// ── Pretty-print JSON with minimal syntax colouring ───────────────────────────

function prettyJson(obj: unknown): string {
 try { return JSON.stringify(obj, null, 2); } catch { return String(obj); }
}

interface JsonViewProps { json: unknown }
function JsonView({ json }: JsonViewProps) {
 const text = prettyJson(json);
 // Tokenise for basic highlighting
 const parts = text.split(/("(?:[^"\\]|\\.)*"\s*:)|("(?:[^"\\]|\\.)*")|(\b(?:true|false|null)\b)|(\b-?\d+(?:\.\d+)?\b)/g)
 .filter(Boolean);
 return (
 <pre style={{ margin: 0, fontFamily: "monospace", fontSize: 12, lineHeight: 1.7, whiteSpace: "pre-wrap", wordBreak: "break-all" }}>
 {parts.map((p, i) => {
 if (/^"[^"]*"\s*:$/.test(p)) return <span key={i} style={{ color: "var(--text-info)" }}>{p}</span>;
 if (/^"/.test(p)) return <span key={i} style={{ color: "var(--text-success)" }}>{p}</span>;
 if (p === "true" || p === "false") return <span key={i} style={{ color: "var(--text-warning-alt)" }}>{p}</span>;
 if (p === "null") return <span key={i} style={{ color: "var(--text-danger)" }}>{p}</span>;
 if (/^-?\d/.test(p)) return <span key={i} style={{ color: "var(--text-warning)" }}>{p}</span>;
 return <span key={i} style={{ color: "var(--text-primary)" }}>{p}</span>;
 })}
 </pre>
 );
}

// ── Expiry helpers ─────────────────────────────────────────────────────────────

interface ExpiryInfo { status: "valid" | "expired" | "none"; remaining: string; exp: number | null }

function getExpiryInfo(payload: Record<string, unknown>): ExpiryInfo {
 const exp = typeof payload.exp === "number" ? payload.exp : null;
 if (exp === null) return { status: "none", remaining: "", exp: null };
 const now = Math.floor(Date.now() / 1000);
 if (exp < now) {
 const ago = now - exp;
 return { status: "expired", remaining: `expired ${fmtDuration(ago)} ago`, exp };
 }
 return { status: "valid", remaining: `expires in ${fmtDuration(exp - now)}`, exp };
}

function fmtDuration(secs: number): string {
 if (secs < 60) return `${secs}s`;
 if (secs < 3600) return `${Math.floor(secs / 60)}m ${secs % 60}s`;
 if (secs < 86400) return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
 return `${Math.floor(secs / 86400)}d ${Math.floor((secs % 86400) / 3600)}h`;
}

function fmtTs(ts: number): string {
 try { return new Date(ts * 1000).toLocaleString(); } catch { return String(ts); }
}

// ── Known claims reference ────────────────────────────────────────────────────

const KNOWN_CLAIMS: Record<string, string> = {
 iss: "Issuer — who issued the token",
 sub: "Subject — whom the token refers to",
 aud: "Audience — intended recipient(s)",
 exp: "Expiration — Unix timestamp after which token is invalid",
 nbf: "Not Before — Unix timestamp before which token is invalid",
 iat: "Issued At — Unix timestamp when token was issued",
 jti: "JWT ID — unique identifier for this token",
 name: "Full name (OIDC)",
 email: "Email address (OIDC)",
 roles: "User roles / permissions",
 scope: "OAuth 2.0 scopes granted",
};

// ── Sample JWT (HS256, secret = "secret") ─────────────────────────────────────

const SAMPLE_JWT =
 "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9." +
 "eyJzdWIiOiJ1c2VyXzEyMyIsIm5hbWUiOiJBbGljZSBTbWl0aCIsImVtYWlsIjoiYWxpY2VAZXhhbXBsZS5jb20iLCJyb2xlcyI6WyJhZG1pbiIsImVkaXRvciJdLCJpYXQiOjE3MDAwMDAwMDAsImV4cCI6OTk5OTk5OTk5OX0." +
 "SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

// ── Sign (HS256 via Web Crypto) ────────────────────────────────────────────────

async function signHS256(headerJson: string, payloadJson: string, secret: string): Promise<string> {
 const enc = new TextEncoder();
 const key = await crypto.subtle.importKey(
 "raw", enc.encode(secret),
 { name: "HMAC", hash: "SHA-256" },
 false, ["sign"]
 );
 const headerB64 = b64urlEncode(enc.encode(headerJson).buffer as ArrayBuffer);
 const payloadB64 = b64urlEncode(enc.encode(payloadJson).buffer as ArrayBuffer);
 const sigBuf = await crypto.subtle.sign("HMAC", key, enc.encode(`${headerB64}.${payloadB64}`));
 return `${headerB64}.${payloadB64}.${b64urlEncode(sigBuf)}`;
}

// ── Component ──────────────────────────────────────────────────────────────────

type Tab = "decode" | "sign" | "claims";

const DEFAULT_HEADER = `{\n "alg": "HS256",\n "typ": "JWT"\n}`;
const DEFAULT_PAYLOAD = `{\n "sub": "user_123",\n "name": "Alice Smith",\n "iat": ${Math.floor(Date.now()/1000)},\n "exp": ${Math.floor(Date.now()/1000) + 3600}\n}`;

export function JwtPanel() {
 const [tab, setTab] = useState<Tab>("decode");
 const [rawJwt, setRawJwt] = useState(SAMPLE_JWT);
 const [signHeader, setSignHeader] = useState(DEFAULT_HEADER);
 const [signPayload, setSignPayload] = useState(DEFAULT_PAYLOAD);
 const [secret, setSecret] = useState("my-secret-key");
 const [showSecret, setShowSecret] = useState(false);
 const [generated, setGenerated] = useState("");
 const [signing, setSigning] = useState(false);
 const [signError, setSignError] = useState("");
 const [copied, setCopied] = useState<string | null>(null);

 // ── Decode ─────────────────────────────────────────────────────────────────

 const { header, payload, signature, decodeError } = useMemo(() => {
 const trimmed = rawJwt.trim();
 if (!trimmed) return { header: null, payload: null, signature: "", decodeError: "" };
 const parts = trimmed.split(".");
 if (parts.length !== 3) return { header: null, payload: null, signature: "", decodeError: "Expected 3 dot-separated parts" };
 try {
 const h = JSON.parse(b64urlDecode(parts[0]));
 const p = JSON.parse(b64urlDecode(parts[1]));
 return { header: h, payload: p, signature: parts[2], decodeError: "" };
 } catch (e) {
 return { header: null, payload: null, signature: "", decodeError: (e as Error).message };
 }
 }, [rawJwt]);

 const expiry = useMemo(() => payload ? getExpiryInfo(payload as Record<string, unknown>) : null, [payload]);

 // ── Copy helper ────────────────────────────────────────────────────────────

 const copy = useCallback((text: string, key: string) => {
 navigator.clipboard.writeText(text);
 setCopied(key);
 setTimeout(() => setCopied(null), 1500);
 }, []);

 // ── Sign ───────────────────────────────────────────────────────────────────

 const handleSign = async () => {
 setSignError("");
 setSigning(true);
 try {
 JSON.parse(signHeader);
 JSON.parse(signPayload);
 const jwt = await signHS256(signHeader, signPayload, secret);
 setGenerated(jwt);
 } catch (e) {
 setSignError((e as Error).message);
 } finally {
 setSigning(false);
 }
 };

 // ── Render ─────────────────────────────────────────────────────────────────

 return (
 <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>

 {/* Header */}
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 10, alignItems: "center" }}>
 <span style={{ fontSize: 13, fontWeight: 600 }}>JWT Debugger</span>
 <div style={{ display: "flex", gap: 4, marginLeft: "auto" }}>
 {(["decode", "sign", "claims"] as Tab[]).map(t => (
 <button key={t} onClick={() => setTab(t)} style={{ padding: "2px 10px", fontSize: 10, borderRadius: 10, background: tab === t ? "rgba(99,102,241,0.2)" : "var(--bg-primary)", border: `1px solid ${tab === t ? "var(--accent-primary)" : "var(--border-color)"}`, color: tab === t ? "var(--text-info)" : "var(--text-muted)", cursor: "pointer", fontWeight: tab === t ? 700 : 400 }}>
 {t === "decode" ? "Decode" : t === "sign" ? "Sign" : "Claims Ref"}
 </button>
 ))}
 </div>
 </div>

 <div style={{ flex: 1, overflow: "auto" }}>

 {/* ── DECODE TAB ── */}
 {tab === "decode" && (
 <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
 {/* Input */}
 <div style={{ borderBottom: "1px solid var(--border-color)" }}>
 <div style={{ padding: "4px 12px", fontSize: 10, fontWeight: 700, color: "var(--text-muted)", background: "var(--bg-secondary)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
 <span>JWT INPUT</span>
 <div style={{ display: "flex", gap: 6 }}>
 <button onClick={() => setRawJwt(SAMPLE_JWT)} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}>Load sample</button>
 <button onClick={() => setRawJwt("")} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}>✕ Clear</button>
 </div>
 </div>
 <textarea value={rawJwt} onChange={e => setRawJwt(e.target.value)} rows={4} spellCheck={false} placeholder="Paste JWT here…"
 style={{ width: "100%", resize: "vertical", padding: "8px 12px", fontSize: 11, fontFamily: "monospace", lineHeight: 1.6, background: "var(--bg-primary)", color: decodeError ? "var(--text-danger)" : "var(--text-accent)", border: "none", outline: "none", boxSizing: "border-box" }} />
 {decodeError && <div style={{ padding: "4px 12px", fontSize: 11, color: "var(--text-danger)", background: "rgba(243,139,168,0.06)" }}>{decodeError}</div>}
 </div>

 {/* Header */}
 {header && (
 <div style={{ borderBottom: "1px solid var(--border-color)" }}>
 <div style={{ padding: "4px 12px", fontSize: 10, fontWeight: 700, color: "var(--text-info)", background: "var(--bg-secondary)", display: "flex", justifyContent: "space-between" }}>
 <span>HEADER · <span style={{ fontFamily: "monospace", color: "var(--text-muted)" }}>{(header as Record<string, unknown>).alg as string} · {(header as Record<string, unknown>).typ as string}</span></span>
 <button onClick={() => copy(prettyJson(header), "header")} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}>{copied === "header" ? "✓" : ""}</button>
 </div>
 <div style={{ padding: "8px 12px", background: "var(--bg-primary)" }}><JsonView json={header} /></div>
 </div>
 )}

 {/* Payload */}
 {payload && (
 <div style={{ borderBottom: "1px solid var(--border-color)" }}>
 <div style={{ padding: "4px 12px", fontSize: 10, fontWeight: 700, color: "var(--text-success)", background: "var(--bg-secondary)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
 <span>PAYLOAD</span>
 <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
 {expiry && expiry.status !== "none" && (
 <span style={{ fontSize: 9, padding: "1px 6px", borderRadius: 10, fontWeight: 700, background: expiry.status === "expired" ? "rgba(243,139,168,0.15)" : "rgba(166,227,161,0.15)", border: `1px solid ${expiry.status === "expired" ? "var(--text-danger)" : "var(--text-success)"}`, color: expiry.status === "expired" ? "var(--text-danger)" : "var(--text-success)" }}>
 {expiry.status === "expired" ? "✕ EXPIRED" : "✓ VALID"} · {expiry.remaining}
 </span>
 )}
 {expiry?.status === "none" && <span style={{ fontSize: 9, color: "var(--text-muted)" }}>no expiry</span>}
 <button onClick={() => copy(prettyJson(payload), "payload")} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}>{copied === "payload" ? "✓" : ""}</button>
 </div>
 </div>
 <div style={{ padding: "8px 12px", background: "var(--bg-primary)" }}><JsonView json={payload} /></div>
 {/* Timestamp fields */}
 {["iat","exp","nbf"].filter(k => typeof (payload as Record<string, unknown>)[k] === "number").map(k => (
 <div key={k} style={{ padding: "2px 12px 4px", fontSize: 10, color: "var(--text-muted)", background: "var(--bg-primary)" }}>
 <span style={{ color: "var(--text-info)" }}>{k}</span> = {fmtTs((payload as Record<string, unknown>)[k] as number)}
 </div>
 ))}
 </div>
 )}

 {/* Signature */}
 {signature && (
 <div>
 <div style={{ padding: "4px 12px", fontSize: 10, fontWeight: 700, color: "var(--text-danger)", background: "var(--bg-secondary)", display: "flex", justifyContent: "space-between" }}>
 <span>SIGNATURE</span>
 <button onClick={() => copy(signature, "sig")} style={{ fontSize: 9, background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}>{copied === "sig" ? "✓" : ""}</button>
 </div>
 <div style={{ padding: "8px 12px", fontFamily: "monospace", fontSize: 11, color: "var(--text-danger)", wordBreak: "break-all", background: "var(--bg-primary)" }}>{signature}</div>
 <div style={{ padding: "2px 12px 8px", fontSize: 10, color: "var(--text-muted)", background: "var(--bg-primary)", fontStyle: "italic" }}>
 Signature verification requires the secret/public key. Use the Sign tab to generate a matching token.
 </div>
 </div>
 )}
 </div>
 )}

 {/* ── SIGN TAB ── */}
 {tab === "sign" && (
 <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
 {/* Header editor */}
 <div style={{ borderBottom: "1px solid var(--border-color)" }}>
 <div style={{ padding: "4px 12px", fontSize: 10, fontWeight: 700, color: "var(--text-info)", background: "var(--bg-secondary)" }}>HEADER (JSON)</div>
 <textarea value={signHeader} onChange={e => setSignHeader(e.target.value)} rows={4} spellCheck={false}
 style={{ width: "100%", resize: "vertical", padding: "8px 12px", fontSize: 12, fontFamily: "monospace", lineHeight: 1.6, background: "var(--bg-primary)", color: "var(--text-primary)", border: "none", outline: "none", boxSizing: "border-box" }} />
 </div>
 {/* Payload editor */}
 <div style={{ borderBottom: "1px solid var(--border-color)" }}>
 <div style={{ padding: "4px 12px", fontSize: 10, fontWeight: 700, color: "var(--text-success)", background: "var(--bg-secondary)" }}>PAYLOAD (JSON)</div>
 <textarea value={signPayload} onChange={e => setSignPayload(e.target.value)} rows={7} spellCheck={false}
 style={{ width: "100%", resize: "vertical", padding: "8px 12px", fontSize: 12, fontFamily: "monospace", lineHeight: 1.6, background: "var(--bg-primary)", color: "var(--text-primary)", border: "none", outline: "none", boxSizing: "border-box" }} />
 </div>
 {/* Secret */}
 <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", display: "flex", gap: 8, alignItems: "center" }}>
 <span style={{ fontSize: 11, color: "var(--text-muted)", flexShrink: 0 }}>Secret (HS256):</span>
 <input type={showSecret ? "text" : "password"} value={secret} onChange={e => setSecret(e.target.value)}
 style={{ flex: 1, padding: "4px 8px", fontSize: 12, fontFamily: "monospace", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-primary)", outline: "none" }} />
 <button onClick={() => setShowSecret(v => !v)} style={{ fontSize: 10, padding: "3px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer" }}>{showSecret ? "Hide" : "Show"}</button>
 <button onClick={handleSign} disabled={signing} style={{ padding: "4px 14px", fontSize: 11, fontWeight: 700, background: "rgba(99,102,241,0.2)", border: "1px solid var(--accent-primary)", borderRadius: 4, color: "var(--text-info)", cursor: "pointer" }}>
 {signing ? "Signing…" : "Sign"}
 </button>
 </div>
 {signError && <div style={{ padding: "6px 12px", fontSize: 11, color: "var(--text-danger)", background: "rgba(243,139,168,0.06)", borderBottom: "1px solid var(--border-color)" }}>{signError}</div>}
 {/* Generated JWT */}
 {generated && (
 <div>
 <div style={{ padding: "4px 12px", fontSize: 10, fontWeight: 700, color: "var(--text-accent)", background: "var(--bg-secondary)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
 <span>GENERATED JWT</span>
 <div style={{ display: "flex", gap: 6 }}>
 <button onClick={() => copy(generated, "gen")} style={{ fontSize: 9, padding: "2px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer" }}>{copied === "gen" ? "✓ Copied" : "Copy"}</button>
 <button onClick={() => { setRawJwt(generated); setTab("decode"); }} style={{ fontSize: 9, padding: "2px 8px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-muted)", cursor: "pointer" }}>→ Decode</button>
 </div>
 </div>
 <div style={{ padding: "10px 12px", fontFamily: "monospace", fontSize: 11, color: "var(--text-accent)", wordBreak: "break-all", lineHeight: 1.8, background: "var(--bg-primary)" }}>
 {generated.split(".").map((part, i) => (
 <span key={i}>
 <span style={{ color: ["var(--accent-color)","var(--success-color)","var(--error-color)"][i] }}>{part}</span>
 {i < 2 && <span style={{ color: "var(--text-muted)" }}>.</span>}
 </span>
 ))}
 </div>
 </div>
 )}
 </div>
 )}

 {/* ── CLAIMS REFERENCE TAB ── */}
 {tab === "claims" && (
 <div style={{ padding: "12px" }}>
 <div style={{ fontSize: 12, color: "var(--text-muted)", marginBottom: 12 }}>Standard JWT claims (RFC 7519) and common OIDC/OAuth 2.0 extensions.</div>
 <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
 <thead>
 <tr style={{ background: "var(--bg-secondary)" }}>
 <th style={{ padding: "6px 10px", textAlign: "left", fontSize: 10, fontWeight: 700, color: "var(--text-muted)", borderBottom: "1px solid var(--border-color)" }}>CLAIM</th>
 <th style={{ padding: "6px 10px", textAlign: "left", fontSize: 10, fontWeight: 700, color: "var(--text-muted)", borderBottom: "1px solid var(--border-color)" }}>DESCRIPTION</th>
 </tr>
 </thead>
 <tbody>
 {Object.entries(KNOWN_CLAIMS).map(([k, v], i) => (
 <tr key={k} style={{ background: i % 2 === 0 ? "transparent" : "rgba(255,255,255,0.02)", borderBottom: "1px solid rgba(255,255,255,0.04)" }}>
 <td style={{ padding: "6px 10px", fontFamily: "monospace", color: "var(--text-info)", fontWeight: 700, whiteSpace: "nowrap" }}>{k}</td>
 <td style={{ padding: "6px 10px", color: "var(--text-primary)" }}>{v}</td>
 </tr>
 ))}
 </tbody>
 </table>
 <div style={{ marginTop: 16, padding: "10px 12px", background: "rgba(250,179,135,0.06)", border: "1px solid rgba(250,179,135,0.3)", borderRadius: 6, fontSize: 11, color: "var(--text-muted)", lineHeight: 1.7 }}>
 <strong style={{ color: "var(--text-warning-alt)" }}>Security note:</strong>JWT payloads are Base64-encoded, <em>not encrypted</em>. Anyone who holds the token can read its claims. Only put non-sensitive data in JWT payloads, and always validate the signature server-side before trusting any claim.
 </div>
 </div>
 )}

 </div>
 </div>
 );
}
