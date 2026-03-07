/**
 * SshPanel — SSH Remote Manager.
 *
 * Save, edit, and connect to remote SSH servers. Run one-off commands with
 * live output streaming. Uses the system `ssh` binary — no native deps.
 */
import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { EmptyState } from "./EmptyState";
import { StatusMessage } from "./StatusMessage";

interface SshProfile {
 id: string;
 name: string;
 host: string;
 port: number;
 user: string;
 key_path: string | null;
 notes: string | null;
}

interface SshCommandResult {
 stdout: string;
 stderr: string;
 exit_code: number;
 duration_ms: number;
 success: boolean;
}

interface SshPanelProps {
 workspacePath: string | null;
}

const BLANK_PROFILE: SshProfile = {
 id: "", name: "", host: "", port: 22, user: "", key_path: null, notes: null,
};

function newId() {
 return Math.random().toString(36).slice(2, 10);
}

export function SshPanel({ workspacePath: _ }: SshPanelProps) {
 const [tab, setTab] = useState<"profiles" | "run">("profiles");
 const [profiles, setProfiles] = useState<SshProfile[]>([]);
 const [editingProfile, setEditingProfile] = useState<SshProfile | null>(null);
 const [isNew, setIsNew] = useState(false);
 const [selectedId, setSelectedId] = useState<string | null>(null);
 const [command, setCommand] = useState("");
 const [logs, setLogs] = useState<string[]>([]);
 const [running, setRunning] = useState(false);
 const [result, setResult] = useState<SshCommandResult | null>(null);
 const [error, setError] = useState<string | null>(null);
 const logRef = useRef<HTMLDivElement>(null);
 const unlistenRef = useRef<(() => void) | null>(null);

 useEffect(() => {
 loadProfiles();
 return () => { unlistenRef.current?.(); };
 }, []);

 useEffect(() => {
 if (logRef.current) logRef.current.scrollTop = logRef.current.scrollHeight;
 }, [logs]);

 const loadProfiles = async () => {
 try {
 const p = await invoke<SshProfile[]>("list_ssh_profiles");
 setProfiles(p);
 if (p.length > 0 && !selectedId) setSelectedId(p[0].id);
 } catch (e) {
 setError(String(e));
 }
 };

 const saveProfile = async () => {
 if (!editingProfile) return;
 if (!editingProfile.host.trim() || !editingProfile.user.trim()) {
 setError("Host and User are required.");
 return;
 }
 const profile: SshProfile = {
 ...editingProfile,
 id: editingProfile.id || newId(),
 name: editingProfile.name.trim() || `${editingProfile.user}@${editingProfile.host}`,
 };
 try {
 await invoke("save_ssh_profile", { profile });
 await loadProfiles();
 setSelectedId(profile.id);
 setEditingProfile(null);
 setIsNew(false);
 } catch (e) {
 setError(String(e));
 }
 };

 const deleteProfile = async (id: string) => {
 if (!confirm("Delete this SSH profile?")) return;
 try {
 await invoke("delete_ssh_profile", { id });
 await loadProfiles();
 if (selectedId === id) setSelectedId(null);
 } catch (e) {
 setError(String(e));
 }
 };

 const runCommand = async () => {
 const profile = profiles.find((p) => p.id === selectedId);
 if (!profile || !command.trim() || running) return;
 setRunning(true);
 setResult(null);
 setLogs([]);
 setError(null);
 setTab("run");

 unlistenRef.current?.();
 const unlisten = await listen<string>("ssh:log", (e) => {
 setLogs((prev) => [...prev, e.payload]);
 });
 unlistenRef.current = unlisten;

 try {
 const res = await invoke<SshCommandResult>("run_ssh_command", {
 host: profile.host,
 port: profile.port,
 user: profile.user,
 keyPath: profile.key_path,
 command: command.trim(),
 });
 setResult(res);
 } catch (e) {
 setError(String(e));
 } finally {
 setRunning(false);
 unlistenRef.current?.();
 unlistenRef.current = null;
 }
 };

 const selected = profiles.find((p) => p.id === selectedId);

 const TAB_STYLE = (active: boolean) => ({
 padding: "6px 14px", fontSize: 12, cursor: "pointer",
 background: active ? "var(--accent-color)" : "transparent",
 color: active ? "var(--text-primary, #e0e0e0)" : "var(--text-secondary)",
 border: "none", borderBottom: active ? "2px solid var(--accent-color)" : "2px solid transparent",
 fontWeight: active ? 600 : 400,
 });

 return (
 <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
 {/* Header */}
 <div style={{
 padding: "10px 12px", borderBottom: "1px solid var(--border-color)",
 background: "var(--bg-secondary)", flexShrink: 0,
 display: "flex", alignItems: "center", gap: 8,
 }}>
 <span style={{ fontSize: 16 }}></span>
 <div>
 <div style={{ fontSize: 13, fontWeight: 600 }}>SSH Remote Manager</div>
 <div style={{ fontSize: 11, color: "var(--text-muted)" }}>
 {profiles.length} profile{profiles.length !== 1 ? "s" : ""}
 </div>
 </div>
 </div>

 {/* Sub-tabs */}
 <div style={{ display: "flex", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" }}>
 <button onClick={() => setTab("profiles")} style={TAB_STYLE(tab === "profiles")}>Profiles</button>
 <button onClick={() => setTab("run")} style={TAB_STYLE(tab === "run")}>Run Command</button>
 </div>

 <div style={{ flex: 1, overflow: "auto" }}>
 {error && (
 <div style={{ margin: "8px 12px" }}>
 <StatusMessage variant="error" message={error} inline />
 <button onClick={() => setError(null)} style={{ position: "relative", top: -26, float: "right", background: "none", border: "none", color: "var(--text-danger, #f38ba8)", cursor: "pointer" }}>✕</button>
 </div>
 )}

 {/* ── Profiles tab ─────────────────────────────────────────────── */}
 {tab === "profiles" && (
 <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 10 }}>
 {/* Toolbar */}
 <div style={{ display: "flex", gap: 6 }}>
 <button
 onClick={() => { setEditingProfile({ ...BLANK_PROFILE }); setIsNew(true); }}
 style={{
 padding: "5px 12px", fontSize: 11, background: "var(--accent-color, #007acc)", color: "var(--text-primary, #e0e0e0)",
 border: "none", borderRadius: 4, cursor: "pointer",
 }}
 >
 + New Profile
 </button>
 {selected && !editingProfile && (
 <>
 <button
 onClick={() => { setEditingProfile({ ...selected }); setIsNew(false); }}
 style={{ padding: "5px 12px", fontSize: 11, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-secondary)", cursor: "pointer" }}
 >
 Edit
 </button>
 <button
 onClick={() => deleteProfile(selected.id)}
 style={{ padding: "5px 12px", fontSize: 11, background: "rgba(244, 67, 54, 0.1)", border: "1px solid var(--error-color, #f44336)", borderRadius: 4, color: "var(--error-color, #f44336)", cursor: "pointer" }}
 >
 Delete
 </button>
 </>
 )}
 </div>

 {/* Profile list */}
 {profiles.length === 0 && !editingProfile ? (
 <EmptyState
   icon="🖥"
   title="No SSH profiles yet"
   description='Click "+ New Profile" to add one.'
 />
 ) : (
 <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
 {profiles.map((p) => (
 <div
 key={p.id}
 onClick={() => { setSelectedId(p.id); setEditingProfile(null); }}
 style={{
 padding: "9px 12px", borderRadius: 6, cursor: "pointer",
 background: selectedId === p.id ? "rgba(99,102,241,0.12)" : "var(--bg-secondary)",
 border: `1px solid ${selectedId === p.id ? "var(--accent-color, #007acc)" : "var(--border-color)"}`,
 display: "flex", alignItems: "center", gap: 10,
 }}
 >
 <span style={{ fontSize: 18 }}></span>
 <div style={{ flex: 1 }}>
 <div style={{ fontSize: 12, fontWeight: 600 }}>{p.name}</div>
 <div style={{ fontSize: 11, color: "var(--text-muted)", fontFamily: "monospace" }}>
 {p.user}@{p.host}:{p.port}
 {p.key_path && <span style={{ marginLeft: 6, color: "var(--text-success, #a6e3a1)" }}></span>}
 </div>
 </div>
 <button
 onClick={(e) => { e.stopPropagation(); setSelectedId(p.id); setTab("run"); }}
 style={{ padding: "3px 10px", fontSize: 10, background: "var(--accent-color, #007acc)", color: "var(--text-primary, #e0e0e0)", border: "none", borderRadius: 4, cursor: "pointer" }}
 >
 Connect →
 </button>
 </div>
 ))}
 </div>
 )}

 {/* Edit / New form */}
 {editingProfile && (
 <div style={{
 padding: 12, background: "var(--bg-secondary)", borderRadius: 6,
 border: "1px solid var(--border-color)", display: "flex", flexDirection: "column", gap: 8,
 }}>
 <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 2 }}>
 {isNew ? "New Profile" : "Edit Profile"}
 </div>
 {[
 { label: "Name (optional)", key: "name", placeholder: "My Server" },
 { label: "Host *", key: "host", placeholder: "192.168.1.1 or example.com" },
 { label: "User *", key: "user", placeholder: "ubuntu" },
 { label: "Port", key: "port", placeholder: "22" },
 { label: "SSH Key Path", key: "key_path", placeholder: "~/.ssh/id_rsa" },
 { label: "Notes", key: "notes", placeholder: "" },
 ].map(({ label, key, placeholder }) => (
 <div key={key} style={{ display: "flex", flexDirection: "column", gap: 3 }}>
 <label style={{ fontSize: 10, color: "var(--text-muted)", fontWeight: 600 }}>{label}</label>
 <input
 type={key === "port" ? "number" : "text"}
 value={(editingProfile[key as keyof SshProfile] ?? "") as string}
 onChange={(e) => setEditingProfile({
 ...editingProfile,
 [key]: key === "port" ? parseInt(e.target.value) || 22 : e.target.value || null,
 })}
 placeholder={placeholder}
 style={{
 padding: "5px 8px", fontSize: 11,
 background: "var(--bg-primary)", border: "1px solid var(--border-color)",
 borderRadius: 4, color: "var(--text-primary)", outline: "none",
 }}
 />
 </div>
 ))}
 <div style={{ display: "flex", gap: 6, marginTop: 4 }}>
 <button
 onClick={saveProfile}
 style={{ padding: "6px 16px", fontSize: 12, background: "var(--accent-color, #007acc)", color: "var(--text-primary, #e0e0e0)", border: "none", borderRadius: 4, cursor: "pointer", fontWeight: 600 }}
 >
 Save
 </button>
 <button
 onClick={() => { setEditingProfile(null); setIsNew(false); }}
 style={{ padding: "6px 12px", fontSize: 12, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4, color: "var(--text-secondary)", cursor: "pointer" }}
 >
 Cancel
 </button>
 </div>
 </div>
 )}
 </div>
 )}

 {/* ── Run Command tab ───────────────────────────────────────────── */}
 {tab === "run" && (
 <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 10 }}>
 {/* Connection selector */}
 <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
 <label style={{ fontSize: 11, color: "var(--text-muted)", fontWeight: 600, flexShrink: 0 }}>Connect to:</label>
 <select
 value={selectedId ?? ""}
 onChange={(e) => setSelectedId(e.target.value)}
 style={{
 flex: 1, padding: "5px 8px", fontSize: 12,
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 borderRadius: 4, color: "var(--text-primary)", outline: "none",
 }}
 >
 <option value="">— Select a profile —</option>
 {profiles.map((p) => (
 <option key={p.id} value={p.id}>{p.name}</option>
 ))}
 </select>
 </div>

 {selected && (
 <div style={{ fontSize: 11, color: "var(--text-muted)", fontFamily: "monospace", padding: "4px 8px", background: "var(--bg-secondary)", borderRadius: 4, border: "1px solid var(--border-color)" }}>
 {selected.user}@{selected.host}:{selected.port}
 {selected.key_path && ` (-i ${selected.key_path})`}
 </div>
 )}

 {/* Quick commands */}
 <div style={{ display: "flex", gap: 5, flexWrap: "wrap" }}>
 {["whoami", "uname -a", "df -h", "free -m", "ps aux | head -20", "ls -la", "uptime"].map((cmd) => (
 <button
 key={cmd}
 onClick={() => setCommand(cmd)}
 style={{
 padding: "3px 8px", fontSize: 10, borderRadius: 12,
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 color: "var(--text-muted)", cursor: "pointer",
 }}
 >
 {cmd}
 </button>
 ))}
 </div>

 {/* Command input */}
 <div style={{ display: "flex", gap: 6 }}>
 <input
 value={command}
 onChange={(e) => setCommand(e.target.value)}
 onKeyDown={(e) => e.key === "Enter" && runCommand()}
 placeholder="Remote command to run…"
 disabled={running}
 style={{
 flex: 1, padding: "6px 10px", fontSize: 12,
 background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
 borderRadius: 4, color: "var(--text-primary)", outline: "none",
 fontFamily: "monospace",
 }}
 />
 <button
 onClick={runCommand}
 disabled={running || !selectedId || !command.trim()}
 style={{
 padding: "6px 16px", fontSize: 12, fontWeight: 600,
 background: running ? "var(--bg-secondary)" : "var(--accent-color, #007acc)",
 color: running ? "var(--text-muted)" : "var(--text-primary, #e0e0e0)",
 border: "none", borderRadius: 4,
 cursor: running || !selectedId ? "not-allowed" : "pointer",
 }}
 >
 {running ? "" : "Run"}
 </button>
 </div>

 {/* Result summary */}
 {result && (
 <div style={{
 padding: "5px 10px", borderRadius: 4, fontSize: 11, fontWeight: 600,
 background: result.success ? "rgba(166,227,161,0.1)" : "rgba(243,139,168,0.1)",
 color: result.success ? "var(--success-color, #4caf50)" : "var(--error-color, #f44336)",
 border: `1px solid ${result.success ? "var(--success-color, #4caf50)" : "var(--error-color, #f44336)"}`,
 display: "flex", justifyContent: "space-between",
 }}>
 <span>{result.success ? "Success" : ` Exit ${result.exit_code}`}</span>
 <span style={{ opacity: 0.8 }}>{(result.duration_ms / 1000).toFixed(2)}s</span>
 </div>
 )}

 {/* Output */}
 {logs.length > 0 && (
 <div
 ref={logRef}
 style={{
 background: "var(--bg-primary, #0d1117)", borderRadius: 6, padding: "8px 10px",
 fontFamily: "monospace", fontSize: 11, lineHeight: 1.5,
 overflow: "auto", maxHeight: 320,
 border: "1px solid var(--border-color)",
 whiteSpace: "pre-wrap", wordBreak: "break-all",
 }}
 >
 {logs.map((line, i) => (
 <div
 key={i}
 style={{
 color: line.startsWith("$ ssh") ? "var(--info-color, #2196f3)"
 : line.startsWith("[Exit") ? (line.includes("Exit 0") ? "var(--success-color, #4caf50)" : "var(--error-color, #f44336)")
 : "var(--text-primary, #e0e0e0)",
 }}
 >
 {line}
 </div>
 ))}
 {running && <div style={{ color: "var(--info-color, #2196f3)" }}>▌</div>}
 </div>
 )}

 {profiles.length === 0 && (
 <EmptyState
   title="No profiles available"
   description="Add an SSH profile in the Profiles tab first."
 />
 )}
 </div>
 )}
 </div>
 </div>
 );
}
