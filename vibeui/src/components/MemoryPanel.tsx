import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface MemoryPanelProps {
    workspacePath?: string | null;
}

type RulesTab = "workspace" | "global" | "directory";

interface RuleFileMeta {
    filename: string;
    name: string;
    path_pattern: string | null;
}

// ── Directory Rules Sub-panel ─────────────────────────────────────────────────

function DirRulesTab({ workspacePath }: { workspacePath?: string | null }) {
    const [scope, setScope] = useState<"workspace" | "global">("workspace");
    const [files, setFiles] = useState<RuleFileMeta[]>([]);
    const [selected, setSelected] = useState<string | null>(null);
    const [content, setContent] = useState("");
    const [loading, setLoading] = useState(false);
    const [saving, setSaving] = useState(false);
    const [saved, setSaved] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [creating, setCreating] = useState(false);
    const [newName, setNewName] = useState("");
    const [newPattern, setNewPattern] = useState("");
    const [confirmDelete, setConfirmDelete] = useState<string | null>(null);

    useEffect(() => {
        loadFiles();
        setSelected(null);
        setContent("");
        setError(null);
    }, [scope, workspacePath]);

    async function loadFiles() {
        setLoading(true);
        setError(null);
        try {
            const list = await invoke<RuleFileMeta[]>("list_rule_files", { scope });
            setFiles(list);
        } catch (e) {
            setError(String(e));
        } finally {
            setLoading(false);
        }
    }

    async function selectFile(filename: string) {
        setSelected(filename);
        setSaved(false);
        try {
            const text = await invoke<string>("get_rule_file", { scope, filename });
            setContent(text);
        } catch (e) {
            setError(String(e));
        }
    }

    async function saveFile() {
        if (!selected) return;
        setSaving(true);
        setSaved(false);
        setError(null);
        try {
            await invoke("save_rule_file", { scope, filename: selected, content });
            setSaved(true);
            setTimeout(() => setSaved(false), 2000);
            await loadFiles();
        } catch (e) {
            setError(String(e));
        } finally {
            setSaving(false);
        }
    }

    async function createFile() {
        const rawName = newName.trim();
        if (!rawName) return;
        const filename = rawName.endsWith(".md") ? rawName : `${rawName}.md`;
        const frontmatter = newPattern.trim()
            ? `---\nname: ${rawName.replace(/\.md$/, "")}\npath_pattern: "${newPattern.trim()}"\n---\n\n`
            : `---\nname: ${rawName.replace(/\.md$/, "")}\n---\n\n`;
        setSaving(true);
        setError(null);
        try {
            await invoke("save_rule_file", { scope, filename, content: frontmatter });
            setCreating(false);
            setNewName("");
            setNewPattern("");
            await loadFiles();
            await selectFile(filename);
        } catch (e) {
            setError(String(e));
        } finally {
            setSaving(false);
        }
    }

    async function deleteFile(filename: string) {
        setError(null);
        try {
            await invoke("delete_rule_file", { scope, filename });
            setConfirmDelete(null);
            if (selected === filename) {
                setSelected(null);
                setContent("");
            }
            await loadFiles();
        } catch (e) {
            setError(String(e));
        }
    }

    const dirLabel = scope === "workspace"
        ? "<workspace>/.vibecli/rules/"
        : "~/.vibecli/rules/";

    return (
        <div style={{ display: "flex", flexDirection: "column", gap: "8px", height: "100%" }}>
            {/* Scope selector */}
            <div style={{ display: "flex", gap: "4px" }}>
                {(["workspace", "global"] as const).map((s) => (
                    <button
                        key={s}
                        onClick={() => setScope(s)}
                        style={{
                            padding: "3px 8px",
                            fontSize: "11px",
                            borderRadius: "4px",
                            background: scope === s ? "var(--accent-blue, #007acc)" : "var(--bg-tertiary)",
                            color: scope === s ? "#fff" : "var(--text-primary)",
                            border: "1px solid var(--border-color)",
                            cursor: "pointer",
                        }}
                    >
                        {s === "workspace" ? "Project" : "Global"}
                    </button>
                ))}
                <button
                    onClick={() => { setCreating(true); setNewName(""); setNewPattern(""); }}
                    style={{
                        marginLeft: "auto",
                        padding: "3px 8px",
                        fontSize: "11px",
                        background: "var(--bg-tertiary)",
                        border: "1px solid var(--border-color)",
                        borderRadius: "4px",
                        color: "var(--text-primary)",
                        cursor: "pointer",
                    }}
                >
                    + New Rule
                </button>
            </div>

            {scope === "workspace" && !workspacePath && (
                <div style={{ fontSize: "12px", color: "#f4a", padding: "6px", background: "rgba(255,68,170,0.1)", borderRadius: "4px" }}>
                    ⚠️ Open a folder to manage project rules.
                </div>
            )}

            {error && (
                <div style={{ fontSize: "12px", color: "#f44", padding: "6px 8px", background: "rgba(220,50,50,0.15)", borderRadius: "4px" }}>
                    {error}
                </div>
            )}

            {/* New rule form */}
            {creating && (
                <div style={{ padding: "8px", background: "var(--bg-tertiary)", borderRadius: "4px", border: "1px solid var(--border-color)", display: "flex", flexDirection: "column", gap: "6px" }}>
                    <div style={{ fontSize: "11px", fontWeight: 600, color: "var(--text-secondary)" }}>New Rule File</div>
                    <input
                        autoFocus
                        type="text"
                        value={newName}
                        onChange={(e) => setNewName(e.target.value)}
                        onKeyDown={(e) => { if (e.key === "Enter") createFile(); if (e.key === "Escape") setCreating(false); }}
                        placeholder="filename (e.g. rust-safety)"
                        style={{ padding: "4px 8px", fontSize: "12px", background: "var(--bg-input, var(--bg-primary))", border: "1px solid var(--border-color)", borderRadius: "4px", color: "var(--text-primary)", outline: "none" }}
                    />
                    <input
                        type="text"
                        value={newPattern}
                        onChange={(e) => setNewPattern(e.target.value)}
                        placeholder="path_pattern (optional, e.g. **/*.rs)"
                        style={{ padding: "4px 8px", fontSize: "12px", background: "var(--bg-input, var(--bg-primary))", border: "1px solid var(--border-color)", borderRadius: "4px", color: "var(--text-primary)", outline: "none" }}
                    />
                    <div style={{ display: "flex", gap: "6px" }}>
                        <button onClick={createFile} disabled={!newName.trim() || saving}
                            style={{ padding: "4px 10px", fontSize: "12px", background: "var(--accent-blue, #007acc)", color: "#fff", border: "none", borderRadius: "4px", cursor: "pointer" }}>
                            Create
                        </button>
                        <button onClick={() => setCreating(false)}
                            style={{ padding: "4px 10px", fontSize: "12px", background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "4px", color: "var(--text-primary)", cursor: "pointer" }}>
                            Cancel
                        </button>
                    </div>
                </div>
            )}

            {/* Main two-column layout */}
            <div style={{ display: "flex", gap: "8px", flex: 1, minHeight: 0 }}>
                {/* File list */}
                <div style={{ width: "140px", flexShrink: 0, overflowY: "auto", border: "1px solid var(--border-color)", borderRadius: "4px", background: "var(--bg-tertiary)" }}>
                    {loading && (
                        <div style={{ padding: "8px", fontSize: "12px", color: "var(--text-secondary)", textAlign: "center" }}>…</div>
                    )}
                    {!loading && files.length === 0 && (
                        <div style={{ padding: "8px", fontSize: "11px", color: "var(--text-secondary)", textAlign: "center", lineHeight: 1.4 }}>
                            No rules yet.<br />Click + New Rule.
                        </div>
                    )}
                    {files.map((f) => (
                        <div
                            key={f.filename}
                            onClick={() => selectFile(f.filename)}
                            style={{
                                padding: "6px 8px",
                                cursor: "pointer",
                                background: selected === f.filename ? "var(--accent-blue, #007acc)" : "transparent",
                                color: selected === f.filename ? "#fff" : "var(--text-primary)",
                                borderBottom: "1px solid var(--border-color)",
                                display: "flex",
                                flexDirection: "column",
                                gap: "2px",
                            }}
                        >
                            <div style={{ fontSize: "12px", fontWeight: selected === f.filename ? 600 : 400, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                                {f.name}
                            </div>
                            {f.path_pattern && (
                                <div style={{ fontSize: "10px", opacity: 0.7, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                                    {f.path_pattern}
                                </div>
                            )}
                            {!f.path_pattern && (
                                <div style={{ fontSize: "10px", opacity: 0.5 }}>always</div>
                            )}
                        </div>
                    ))}
                </div>

                {/* Editor */}
                {selected ? (
                    <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: "6px", minWidth: 0 }}>
                        <div style={{ fontSize: "11px", color: "var(--text-secondary)", fontFamily: "monospace" }}>
                            {dirLabel}{selected}
                        </div>
                        <textarea
                            value={content}
                            onChange={(e) => setContent(e.target.value)}
                            style={{
                                flex: 1,
                                background: "var(--bg-tertiary)",
                                border: "1px solid var(--border-color)",
                                color: "var(--text-primary)",
                                borderRadius: "4px",
                                padding: "8px",
                                fontSize: "12px",
                                fontFamily: "monospace",
                                resize: "none",
                                outline: "none",
                            }}
                            placeholder="Write your rule content here…"
                        />
                        <div style={{ display: "flex", gap: "6px" }}>
                            <button
                                onClick={saveFile}
                                disabled={saving}
                                style={{ padding: "5px 12px", fontSize: "12px", background: "var(--accent-blue, #007acc)", color: "#fff", border: "none", borderRadius: "4px", cursor: "pointer" }}
                            >
                                {saving ? "Saving…" : saved ? "✓ Saved" : "💾 Save"}
                            </button>
                            <button
                                onClick={() => setConfirmDelete(selected)}
                                style={{ padding: "5px 12px", fontSize: "12px", background: "transparent", border: "1px solid #c0392b", borderRadius: "4px", color: "#c0392b", cursor: "pointer", marginLeft: "auto" }}
                            >
                                Delete
                            </button>
                        </div>
                    </div>
                ) : (
                    <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center", fontSize: "12px", color: "var(--text-secondary)" }}>
                        Select a rule to edit
                    </div>
                )}
            </div>

            {/* Dir label */}
            <div style={{ fontSize: "11px", color: "var(--text-secondary)" }}>
                Files in <code style={{ fontSize: "10px" }}>{dirLabel}</code>
            </div>

            {/* Confirm delete modal */}
            {confirmDelete && (
                <div style={{ position: "absolute", inset: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100 }}>
                    <div style={{ background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: "8px", padding: "20px", maxWidth: "300px", width: "90%" }}>
                        <div style={{ fontSize: "13px", fontWeight: 600, marginBottom: "10px" }}>Delete Rule?</div>
                        <div style={{ fontSize: "12px", color: "var(--text-secondary)", marginBottom: "16px" }}>
                            Permanently delete <strong>{confirmDelete}</strong>?
                        </div>
                        <div style={{ display: "flex", gap: "8px", justifyContent: "flex-end" }}>
                            <button onClick={() => setConfirmDelete(null)}
                                style={{ padding: "6px 14px", fontSize: "12px", background: "var(--bg-primary)", border: "1px solid var(--border-color)", borderRadius: "4px", color: "var(--text-primary)", cursor: "pointer" }}>
                                Cancel
                            </button>
                            <button onClick={() => deleteFile(confirmDelete)}
                                style={{ padding: "6px 14px", fontSize: "12px", background: "#c0392b", border: "none", borderRadius: "4px", color: "#fff", cursor: "pointer" }}>
                                Delete
                            </button>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}

// ── MemoryPanel ───────────────────────────────────────────────────────────────

export function MemoryPanel({ workspacePath }: MemoryPanelProps) {
    const [activeTab, setActiveTab] = useState<RulesTab>("workspace");
    const [workspaceRules, setWorkspaceRules] = useState("");
    const [globalRules, setGlobalRules] = useState("");
    const [saving, setSaving] = useState(false);
    const [saved, setSaved] = useState(false);

    useEffect(() => {
        if (workspacePath) {
            invoke<string>("get_vibeui_rules")
                .then(setWorkspaceRules)
                .catch(() => setWorkspaceRules(""));
        }
        invoke<string>("get_global_rules")
            .then(setGlobalRules)
            .catch(() => setGlobalRules(""));
    }, [workspacePath]);

    const save = async () => {
        setSaving(true);
        setSaved(false);
        try {
            if (activeTab === "workspace") {
                await invoke("save_vibeui_rules", { content: workspaceRules });
            } else {
                await invoke("save_global_rules", { content: globalRules });
            }
            setSaved(true);
            setTimeout(() => setSaved(false), 2000);
        } catch (e) {
            alert("Failed to save: " + e);
        } finally {
            setSaving(false);
        }
    };

    const placeholder =
        activeTab === "workspace"
            ? `# Project AI Rules\n\nInstructions injected into every AI request for this project.\n\nExamples:\n- Always use TypeScript strict mode\n- Prefer async/await over .then()\n- Use PostgreSQL for database operations\n- Follow the existing folder structure in src/`
            : `# Global AI Rules\n\nPersonal defaults applied to all projects.\n\nExamples:\n- Always add error handling\n- Write tests for every new function\n- Use descriptive variable names\n- Prefer immutability`;

    const tabs: { id: RulesTab; label: string }[] = [
        { id: "workspace", label: "Project" },
        { id: "global", label: "Global" },
        { id: "directory", label: "Dir Rules" },
    ];

    return (
        <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "12px", gap: "8px" }}>
            <div style={{ fontWeight: 600, fontSize: "14px" }}>📋 AI Rules / Memory</div>
            <p style={{ fontSize: "12px", color: "var(--text-secondary)", margin: 0 }}>
                Persistent instructions injected into every AI request.
            </p>

            {/* Tab selector */}
            <div style={{ display: "flex", gap: "4px" }}>
                {tabs.map((t) => (
                    <button
                        key={t.id}
                        onClick={() => setActiveTab(t.id)}
                        style={{
                            padding: "4px 10px",
                            fontSize: "12px",
                            borderRadius: "4px",
                            background: activeTab === t.id ? "var(--accent-blue, #007acc)" : "var(--bg-tertiary)",
                            color: activeTab === t.id ? "#fff" : "var(--text-primary)",
                            border: "1px solid var(--border-color)",
                            cursor: "pointer",
                        }}
                    >
                        {t.label}
                    </button>
                ))}
            </div>

            {/* Directory rules tab */}
            {activeTab === "directory" && (
                <div style={{ flex: 1, minHeight: 0, position: "relative" }}>
                    <DirRulesTab workspacePath={workspacePath} />
                </div>
            )}

            {/* Single-file rules tabs */}
            {activeTab !== "directory" && (
                <>
                    {activeTab === "workspace" && !workspacePath && (
                        <div style={{ fontSize: "12px", color: "#f4a", padding: "6px", background: "rgba(255,68,170,0.1)", borderRadius: "4px" }}>
                            ⚠️ Open a folder to manage project rules.
                        </div>
                    )}

                    <textarea
                        value={activeTab === "workspace" ? workspaceRules : globalRules}
                        onChange={(e) =>
                            activeTab === "workspace"
                                ? setWorkspaceRules(e.target.value)
                                : setGlobalRules(e.target.value)
                        }
                        placeholder={placeholder}
                        disabled={activeTab === "workspace" && !workspacePath}
                        style={{
                            flex: 1,
                            background: "var(--bg-tertiary)",
                            border: "1px solid var(--border-color)",
                            color: "var(--text-primary)",
                            borderRadius: "4px",
                            padding: "8px",
                            fontSize: "13px",
                            fontFamily: "monospace",
                            resize: "none",
                            opacity: activeTab === "workspace" && !workspacePath ? 0.5 : 1,
                        }}
                    />

                    <button
                        className="btn-primary"
                        onClick={save}
                        disabled={saving || (activeTab === "workspace" && !workspacePath)}
                    >
                        {saving
                            ? "Saving…"
                            : saved
                            ? "✓ Saved!"
                            : `💾 Save ${activeTab === "workspace" ? "Project" : "Global"} Rules`}
                    </button>

                    <div style={{ fontSize: "11px", color: "var(--text-secondary)" }}>
                        {activeTab === "workspace"
                            ? "Saved to <workspace>/.vibeui.md — commit it with your project."
                            : "Saved to ~/.vibeui/rules.md — applies to all projects."}
                    </div>
                </>
            )}
        </div>
    );
}
