import React, { useState } from "react";
import { Mail } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

type AuthProvider = "github" | "google" | "email" | "jwt";
type Framework = "nextjs" | "express" | "fastapi" | "axum" | "supabase";

interface AuthConfig {
 auth_provider: AuthProvider;
 framework: Framework;
 include_middleware: boolean;
 include_tests: boolean;
}

export function AuthPanel({ workspacePath, provider }: { workspacePath: string | null; provider: string }) {
 const [config, setConfig] = useState<AuthConfig>({
 auth_provider: "github",
 framework: "nextjs",
 include_middleware: true,
 include_tests: true,
 });
 const [generatedCode, setGeneratedCode] = useState<string>("");
 const [targetPath, setTargetPath] = useState("src/auth");
 const [loading, setLoading] = useState(false);
 const [error, setError] = useState<string | null>(null);
 const [saved, setSaved] = useState(false);

 if (!workspacePath) {
 return <div className="empty-state"><p>Open a workspace folder to generate auth scaffolding.</p></div>;
 }

 const generate = async () => {
 setLoading(true);
 setError(null);
 setSaved(false);
 try {
 const code = await invoke<string>("generate_auth_scaffold", {
 workspacePath,
 provider: provider,
 authProvider: config.auth_provider,
 framework: config.framework,
 includeMiddleware: config.include_middleware,
 includeTests: config.include_tests,
 });
 setGeneratedCode(code);
 } catch (e) {
 setError(String(e));
 } finally {
 setLoading(false);
 }
 };

 const saveToWorkspace = async () => {
 if (!generatedCode) return;
 setLoading(true);
 try {
 await invoke("write_auth_scaffold", {
 workspacePath,
 targetPath,
 code: generatedCode,
 framework: config.framework,
 });
 setSaved(true);
 } catch (e) {
 setError(String(e));
 } finally {
 setLoading(false);
 }
 };

 const s = {
 panel: { display: "flex", flexDirection: "column", height: "100%", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: "13px" } as React.CSSProperties,
 header: { padding: "10px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" } as React.CSSProperties,
 content: { flex: 1, overflow: "auto", padding: "12px", display: "flex", flexDirection: "column", gap: "12px" } as React.CSSProperties,
 label: { display: "block", marginBottom: "4px", fontSize: "11px", color: "var(--text-secondary)" } as React.CSSProperties,
 select: { width: "100%", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", color: "var(--text-primary)", padding: "6px 8px", borderRadius: "4px", fontSize: "12px" } as React.CSSProperties,
 input: { width: "100%", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", color: "var(--text-primary)", padding: "6px 8px", borderRadius: "4px", fontSize: "12px", boxSizing: "border-box" as const } as React.CSSProperties,
 btn: { padding: "8px 16px", background: "var(--accent-color)", color: "white", border: "none", borderRadius: "4px", cursor: "pointer", fontSize: "13px", fontWeight: 600 } as React.CSSProperties,
 code: { background: "var(--bg-secondary)", padding: "10px", borderRadius: "4px", fontSize: "11px", fontFamily: "monospace", whiteSpace: "pre" as const, overflow: "auto", maxHeight: "300px", border: "1px solid var(--border-color)" } as React.CSSProperties,
 row: { display: "flex", gap: "8px", alignItems: "center" } as React.CSSProperties,
 chip: (active: boolean): React.CSSProperties => ({ padding: "4px 12px", border: `1px solid ${active ? "var(--accent-color)" : "var(--border-color)"}`, borderRadius: "12px", cursor: "pointer", fontSize: "12px", background: active ? "rgba(0,122,204,0.15)" : "transparent", color: active ? "var(--accent-color)" : "var(--text-secondary)" }),
 };

 const AUTH_PROVIDERS: { value: AuthProvider; label: string; icon: React.ReactNode }[] = [
 { value: "github", label: "GitHub OAuth", icon: "" },
 { value: "google", label: "Google OAuth", icon: "" },
 { value: "email", label: "Email + Password", icon: <Mail size={14} strokeWidth={1.5} /> },
 { value: "jwt", label: "JWT / Bearer", icon: "" },
 ];

 const FRAMEWORKS: { value: Framework; label: string; lang: string }[] = [
 { value: "nextjs", label: "Next.js", lang: "TypeScript" },
 { value: "express", label: "Express.js", lang: "TypeScript" },
 { value: "fastapi", label: "FastAPI", lang: "Python" },
 { value: "axum", label: "Axum", lang: "Rust" },
 { value: "supabase", label: "Supabase Auth", lang: "TypeScript" },
 ];

 return (
 <div style={s.panel}>
 <div style={s.header}>
 <span style={{ fontSize: "14px", fontWeight: 600 }}>Auth Scaffolding</span>
 <p style={{ margin: "4px 0 0", color: "var(--text-secondary)", fontSize: "11px" }}>Generate authentication boilerplate for your stack</p>
 </div>

 <div style={s.content}>
 <div>
 <div style={s.label}>Auth Provider</div>
 <div style={{ display: "flex", gap: "6px", flexWrap: "wrap" as const }}>
 {AUTH_PROVIDERS.map(p => (
 <div key={p.value} style={s.chip(config.auth_provider === p.value)} onClick={() => setConfig(c => ({ ...c, auth_provider: p.value }))}>
 {p.icon} {p.label}
 </div>
 ))}
 </div>
 </div>

 <div>
 <div style={s.label}>Framework</div>
 <div style={{ display: "flex", gap: "6px", flexWrap: "wrap" as const }}>
 {FRAMEWORKS.map(f => (
 <div key={f.value} style={s.chip(config.framework === f.value)} onClick={() => setConfig(c => ({ ...c, framework: f.value }))}>
 {f.label} <span style={{ fontSize: "10px", opacity: 0.7 }}>({f.lang})</span>
 </div>
 ))}
 </div>
 </div>

 <div style={s.row}>
 <label style={{ display: "flex", alignItems: "center", gap: "8px", cursor: "pointer", fontSize: "12px" }}>
 <input type="checkbox" checked={config.include_middleware} onChange={e => setConfig(c => ({ ...c, include_middleware: e.target.checked }))} />
 Include auth middleware
 </label>
 <label style={{ display: "flex", alignItems: "center", gap: "8px", cursor: "pointer", fontSize: "12px" }}>
 <input type="checkbox" checked={config.include_tests} onChange={e => setConfig(c => ({ ...c, include_tests: e.target.checked }))} />
 Include tests
 </label>
 </div>

 <button style={s.btn} onClick={generate} disabled={loading}>
 {loading ? "Generating..." : "Generate Auth Code"}
 </button>

 {error && <div style={{ color: "var(--error-color)", fontSize: "12px", background: "rgba(244,67,54,0.1)", padding: "8px", borderRadius: "4px" }}>{error}</div>}

 {generatedCode && (
 <>
 <div>
 <div style={s.label}>Preview</div>
 <pre style={s.code}>{generatedCode.slice(0, 2000)}{generatedCode.length > 2000 ? "\n... (truncated)" : ""}</pre>
 </div>

 <div>
 <div style={s.label}>Save to workspace path</div>
 <div style={s.row}>
 <input style={s.input} value={targetPath} onChange={e => setTargetPath(e.target.value)} />
 <button style={{ ...s.btn, whiteSpace: "nowrap" as const }} onClick={saveToWorkspace} disabled={loading}>
 {saved ? "Saved" : "Save Files"}
 </button>
 </div>
 {saved && <div style={{ color: "var(--success-color)", fontSize: "12px", marginTop: "4px" }}>Files written to {workspacePath}/{targetPath}</div>}
 </div>
 </>
 )}
 </div>
 </div>
 );
}
