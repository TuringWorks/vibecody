import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface MemoryPanelProps {
    workspacePath?: string | null;
}

type RulesTab = "workspace" | "global";

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

    return (
        <div style={{ display: "flex", flexDirection: "column", height: "100%", padding: "12px", gap: "8px" }}>
            <div style={{ fontWeight: 600, fontSize: "14px" }}>📋 AI Rules / Memory</div>
            <p style={{ fontSize: "12px", color: "var(--text-secondary)", margin: 0 }}>
                Persistent instructions injected into every AI request.
            </p>

            {/* Tab selector */}
            <div style={{ display: "flex", gap: "4px" }}>
                {(["workspace", "global"] as RulesTab[]).map((tab) => (
                    <button
                        key={tab}
                        onClick={() => setActiveTab(tab)}
                        style={{
                            padding: "4px 10px",
                            fontSize: "12px",
                            borderRadius: "4px",
                            background:
                                activeTab === tab
                                    ? "var(--accent-blue, #007acc)"
                                    : "var(--bg-tertiary)",
                            color: activeTab === tab ? "#fff" : "var(--text-primary)",
                            border: "1px solid var(--border-color)",
                            cursor: "pointer",
                        }}
                    >
                        {tab === "workspace"
                            ? "Project (.vibeui.md)"
                            : "Global (~/.vibeui/rules.md)"}
                    </button>
                ))}
            </div>

            {activeTab === "workspace" && !workspacePath && (
                <div
                    style={{
                        fontSize: "12px",
                        color: "#f4a",
                        padding: "6px",
                        background: "rgba(255,68,170,0.1)",
                        borderRadius: "4px",
                    }}
                >
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
        </div>
    );
}
