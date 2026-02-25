import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface McpServer {
    name: string;
    command: string;
    args: string[];
    env: Record<string, string>;
}

interface McpToolInfo {
    name: string;
    description: string;
}

const EMPTY_SERVER: McpServer = { name: "", command: "", args: [], env: {} };

export function McpPanel() {
    const [servers, setServers] = useState<McpServer[]>([]);
    const [editing, setEditing] = useState<McpServer | null>(null);
    const [editIdx, setEditIdx] = useState<number | null>(null);
    const [saving, setSaving] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [testing, setTesting] = useState<number | null>(null);
    const [testResult, setTestResult] = useState<Record<number, McpToolInfo[] | string>>({});
    const [confirmDelete, setConfirmDelete] = useState<number | null>(null);

    useEffect(() => {
        loadServers();
    }, []);

    async function loadServers() {
        setError(null);
        try {
            const list = await invoke<McpServer[]>("get_mcp_servers");
            setServers(list);
        } catch (e) {
            setError(String(e));
        }
    }

    async function save(list: McpServer[]) {
        setSaving(true);
        setError(null);
        try {
            await invoke("save_mcp_servers", { servers: list });
            setServers(list);
        } catch (e) {
            setError(String(e));
        } finally {
            setSaving(false);
        }
    }

    function startAdd() {
        setEditing({ ...EMPTY_SERVER });
        setEditIdx(null);
    }

    function startEdit(idx: number) {
        setEditing({ ...servers[idx], args: [...servers[idx].args] });
        setEditIdx(idx);
    }

    async function commitEdit() {
        if (!editing || !editing.name.trim() || !editing.command.trim()) return;
        const updated = [...servers];
        if (editIdx === null) {
            updated.push({ ...editing });
        } else {
            updated[editIdx] = { ...editing };
        }
        await save(updated);
        setEditing(null);
        setEditIdx(null);
    }

    async function deleteServer(idx: number) {
        const updated = servers.filter((_, i) => i !== idx);
        await save(updated);
        setConfirmDelete(null);
        // Clear any test result for deleted server
        setTestResult((prev) => {
            const next = { ...prev };
            delete next[idx];
            return next;
        });
    }

    async function testServer(idx: number) {
        setTesting(idx);
        setTestResult((prev) => ({ ...prev, [idx]: [] }));
        try {
            const tools = await invoke<McpToolInfo[]>("test_mcp_server", { server: servers[idx] });
            setTestResult((prev) => ({ ...prev, [idx]: tools }));
        } catch (e) {
            setTestResult((prev) => ({ ...prev, [idx]: String(e) }));
        } finally {
            setTesting(null);
        }
    }

    const result = (idx: number) => testResult[idx];

    return (
        <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "12px", gap: "10px", fontFamily: "var(--font-mono, monospace)", position: "relative" }}>
            {/* Header */}
            <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ fontWeight: 600, fontSize: "14px" }}>🔌 MCP Servers</span>
                <button
                    onClick={startAdd}
                    style={{ marginLeft: "auto", padding: "4px 10px", fontSize: "12px", background: "var(--accent-blue, #007acc)", color: "#fff", border: "none", borderRadius: "4px", cursor: "pointer" }}
                >
                    + Add Server
                </button>
            </div>

            <p style={{ fontSize: "12px", color: "var(--text-secondary)", margin: 0 }}>
                Configure external MCP servers. Their tools are injected into the agent context as{" "}
                <code style={{ fontSize: "11px" }}>mcp__&lt;server&gt;__&lt;tool&gt;</code>.
            </p>

            {error && (
                <div style={{ fontSize: "12px", color: "#f44", padding: "6px 8px", background: "rgba(220,50,50,0.15)", borderRadius: "4px" }}>
                    {error}
                </div>
            )}

            {/* Server list */}
            <div style={{ flex: 1, overflowY: "auto", display: "flex", flexDirection: "column", gap: "8px" }}>
                {servers.length === 0 && (
                    <div style={{ fontSize: "12px", color: "var(--text-secondary)", textAlign: "center", padding: "32px 16px" }}>
                        No MCP servers configured.<br />
                        <span style={{ opacity: 0.7 }}>Click "+ Add Server" to add one.</span>
                    </div>
                )}

                {servers.map((srv, idx) => {
                    const res = result(idx);
                    const isTools = Array.isArray(res);
                    const isErr = typeof res === "string";
                    return (
                        <div
                            key={idx}
                            style={{
                                border: "1px solid var(--border-color)",
                                borderRadius: "6px",
                                padding: "10px 12px",
                                background: "var(--bg-secondary)",
                                display: "flex",
                                flexDirection: "column",
                                gap: "6px",
                            }}
                        >
                            {/* Server header row */}
                            <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                                <span style={{ fontSize: "13px", fontWeight: 600, color: "var(--text-primary)", flex: 1 }}>
                                    {srv.name}
                                </span>
                                <button
                                    onClick={() => testServer(idx)}
                                    disabled={testing === idx}
                                    style={{ padding: "2px 8px", fontSize: "11px", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "3px", color: "var(--text-primary)", cursor: "pointer" }}
                                >
                                    {testing === idx ? "Testing…" : "Test"}
                                </button>
                                <button
                                    onClick={() => startEdit(idx)}
                                    style={{ padding: "2px 8px", fontSize: "11px", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: "3px", color: "var(--text-primary)", cursor: "pointer" }}
                                >
                                    Edit
                                </button>
                                <button
                                    onClick={() => setConfirmDelete(idx)}
                                    style={{ padding: "2px 8px", fontSize: "11px", background: "transparent", border: "1px solid #c0392b", borderRadius: "3px", color: "#c0392b", cursor: "pointer" }}
                                >
                                    ✕
                                </button>
                            </div>

                            {/* Command */}
                            <div style={{ fontSize: "11px", color: "var(--text-secondary)", fontFamily: "monospace", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                                $ {srv.command}{srv.args.length > 0 ? " " + srv.args.join(" ") : ""}
                            </div>

                            {/* Tool test results */}
                            {isErr && (
                                <div style={{ fontSize: "11px", color: "#f44", padding: "4px 6px", background: "rgba(220,50,50,0.1)", borderRadius: "3px" }}>
                                    ❌ {res}
                                </div>
                            )}
                            {isTools && res.length === 0 && (
                                <div style={{ fontSize: "11px", color: "var(--text-secondary)" }}>No tools exposed.</div>
                            )}
                            {isTools && res.length > 0 && (
                                <div style={{ display: "flex", flexDirection: "column", gap: "2px", marginTop: "2px" }}>
                                    <div style={{ fontSize: "10px", color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.05em" }}>
                                        {res.length} tool{res.length !== 1 ? "s" : ""}
                                    </div>
                                    {res.map((t) => (
                                        <div key={t.name} style={{ fontSize: "11px", display: "flex", gap: "6px" }}>
                                            <code style={{ color: "var(--accent-blue, #007acc)", flexShrink: 0 }}>{t.name}</code>
                                            <span style={{ color: "var(--text-secondary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{t.description}</span>
                                        </div>
                                    ))}
                                </div>
                            )}
                        </div>
                    );
                })}
            </div>

            {/* Edit / Add form */}
            {editing && (
                <div style={{
                    position: "absolute",
                    inset: 0,
                    background: "rgba(0,0,0,0.5)",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    zIndex: 100,
                }}>
                    <div style={{
                        background: "var(--bg-secondary)",
                        border: "1px solid var(--border-color)",
                        borderRadius: "8px",
                        padding: "20px",
                        width: "360px",
                        display: "flex",
                        flexDirection: "column",
                        gap: "10px",
                    }}>
                        <div style={{ fontSize: "13px", fontWeight: 600 }}>
                            {editIdx === null ? "Add MCP Server" : "Edit MCP Server"}
                        </div>

                        <label style={{ fontSize: "12px", display: "flex", flexDirection: "column", gap: "4px" }}>
                            Name
                            <input
                                autoFocus
                                type="text"
                                value={editing.name}
                                onChange={(e) => setEditing({ ...editing, name: e.target.value })}
                                placeholder="e.g. github"
                                style={inputStyle}
                            />
                        </label>

                        <label style={{ fontSize: "12px", display: "flex", flexDirection: "column", gap: "4px" }}>
                            Command
                            <input
                                type="text"
                                value={editing.command}
                                onChange={(e) => setEditing({ ...editing, command: e.target.value })}
                                placeholder="e.g. npx @modelcontextprotocol/server-github"
                                style={inputStyle}
                            />
                        </label>

                        <label style={{ fontSize: "12px", display: "flex", flexDirection: "column", gap: "4px" }}>
                            Extra args (space-separated)
                            <input
                                type="text"
                                value={editing.args.join(" ")}
                                onChange={(e) => setEditing({ ...editing, args: e.target.value ? e.target.value.split(" ") : [] })}
                                placeholder="optional"
                                style={inputStyle}
                            />
                        </label>

                        <div style={{ display: "flex", gap: "8px", justifyContent: "flex-end", marginTop: "4px" }}>
                            <button
                                onClick={() => { setEditing(null); setEditIdx(null); }}
                                style={{ padding: "6px 14px", fontSize: "12px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", color: "var(--text-primary)", cursor: "pointer" }}
                            >
                                Cancel
                            </button>
                            <button
                                onClick={commitEdit}
                                disabled={!editing.name.trim() || !editing.command.trim() || saving}
                                style={{ padding: "6px 14px", fontSize: "12px", background: "var(--accent-blue, #007acc)", border: "none", borderRadius: "4px", color: "#fff", cursor: "pointer" }}
                            >
                                {saving ? "Saving…" : editIdx === null ? "Add" : "Save"}
                            </button>
                        </div>
                    </div>
                </div>
            )}

            {/* Confirm delete modal */}
            {confirmDelete !== null && (
                <div style={{ position: "absolute", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}>
                    <div style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "8px", padding: "20px", maxWidth: "300px", width: "90%", display: "flex", flexDirection: "column", gap: "12px" }}>
                        <div style={{ fontSize: "13px", fontWeight: 600 }}>Remove Server?</div>
                        <div style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
                            Remove <strong>{servers[confirmDelete]?.name}</strong> from the list?
                        </div>
                        <div style={{ display: "flex", gap: "8px", justifyContent: "flex-end" }}>
                            <button onClick={() => setConfirmDelete(null)}
                                style={{ padding: "6px 14px", fontSize: "12px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", color: "var(--text-primary)", cursor: "pointer" }}>
                                Cancel
                            </button>
                            <button onClick={() => deleteServer(confirmDelete)}
                                style={{ padding: "6px 14px", fontSize: "12px", background: "#c0392b", border: "none", borderRadius: "4px", color: "#fff", cursor: "pointer" }}>
                                Remove
                            </button>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}

const inputStyle: React.CSSProperties = {
    padding: "5px 8px",
    fontSize: "12px",
    background: "var(--bg-input, var(--bg-primary))",
    border: "1px solid var(--border-color)",
    borderRadius: "4px",
    color: "var(--text-primary)",
    outline: "none",
    fontFamily: "monospace",
};
