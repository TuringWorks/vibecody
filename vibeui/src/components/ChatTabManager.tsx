import { useState } from "react";
import { AIChat } from "./AIChat";

interface ChatTab {
    id: string;
    title: string;
    provider: string;
}

interface ChatTabManagerProps {
    defaultProvider: string;
    availableProviders: string[];
    context?: string;
    fileTree?: string[];
    currentFile?: string | null;
    onPendingWrite?: (path: string, content: string) => void;
}

let nextTabId = 1;

export function ChatTabManager({
    defaultProvider,
    availableProviders,
    context,
    fileTree,
    currentFile,
    onPendingWrite,
}: ChatTabManagerProps) {
    const [tabs, setTabs] = useState<ChatTab[]>([
        { id: "tab-1", title: "Chat 1", provider: defaultProvider },
    ]);
    const [activeTabId, setActiveTabId] = useState("tab-1");

    const addTab = () => {
        nextTabId++;
        const newTab: ChatTab = {
            id: `tab-${nextTabId}`,
            title: `Chat ${nextTabId}`,
            provider: defaultProvider,
        };
        setTabs((prev) => [...prev, newTab]);
        setActiveTabId(newTab.id);
    };

    const closeTab = (id: string, e: React.MouseEvent) => {
        e.stopPropagation();
        if (tabs.length === 1) return; // keep at least one tab
        const idx = tabs.findIndex((t) => t.id === id);
        const newTabs = tabs.filter((t) => t.id !== id);
        setTabs(newTabs);
        if (activeTabId === id) {
            const nextIdx = Math.min(idx, newTabs.length - 1);
            setActiveTabId(newTabs[nextIdx].id);
        }
    };

    const setTabProvider = (id: string, provider: string) => {
        setTabs((prev) =>
            prev.map((t) => (t.id === id ? { ...t, provider } : t))
        );
    };

    const activeTab = tabs.find((t) => t.id === activeTabId);

    return (
        <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
            {/* Tab strip */}
            <div style={{
                display: "flex", alignItems: "center", gap: "1px",
                background: "var(--bg-secondary)", borderBottom: "1px solid var(--border-color)",
                flexShrink: 0, overflowX: "auto", minHeight: "32px",
            }}>
                {tabs.map((tab) => (
                    <div
                        key={tab.id}
                        onClick={() => setActiveTabId(tab.id)}
                        style={{
                            display: "flex", alignItems: "center", gap: "4px",
                            padding: "4px 10px", cursor: "pointer", flexShrink: 0,
                            fontSize: "12px", userSelect: "none",
                            background: activeTabId === tab.id ? "var(--bg-primary)" : "transparent",
                            color: activeTabId === tab.id ? "var(--text-primary)" : "var(--text-secondary)",
                            borderBottom: activeTabId === tab.id
                                ? "2px solid var(--accent-blue, #007acc)"
                                : "2px solid transparent",
                        }}
                    >
                        <span>{tab.title}</span>
                        {tabs.length > 1 && (
                            <button
                                onClick={(e) => closeTab(tab.id, e)}
                                style={{
                                    background: "none", border: "none", color: "inherit",
                                    cursor: "pointer", padding: "0 2px", fontSize: "14px",
                                    lineHeight: 1,
                                }}
                                title="Close tab"
                            >
                                ×
                            </button>
                        )}
                    </div>
                ))}
                <button
                    onClick={addTab}
                    title="New chat tab"
                    style={{
                        background: "none", border: "none", color: "var(--text-secondary)",
                        cursor: "pointer", padding: "4px 8px", fontSize: "16px",
                        lineHeight: 1, flexShrink: 0,
                    }}
                >
                    +
                </button>

                {/* Per-tab model selector */}
                {activeTab && availableProviders.length > 1 && (
                    <select
                        value={activeTab.provider}
                        onChange={(e) => setTabProvider(activeTab.id, e.target.value)}
                        style={{
                            marginLeft: "auto", marginRight: "6px",
                            padding: "2px 4px", fontSize: "11px",
                            background: "var(--bg-tertiary)", border: "1px solid var(--border-color)",
                            color: "var(--text-secondary)", borderRadius: "3px",
                            maxWidth: "140px",
                        }}
                        title="Model for this chat tab"
                    >
                        {availableProviders.map((p) => (
                            <option key={p} value={p}>{p}</option>
                        ))}
                    </select>
                )}
            </div>

            {/* Tab content — render only active tab, but mount all to preserve history */}
            <div style={{ flex: 1, overflow: "hidden" }}>
                {tabs.map((tab) => (
                    <div
                        key={tab.id}
                        style={{
                            display: activeTabId === tab.id ? "flex" : "none",
                            flexDirection: "column",
                            height: "100%",
                        }}
                    >
                        <AIChat
                            provider={tab.provider}
                            context={context}
                            fileTree={fileTree}
                            currentFile={currentFile}
                            onPendingWrite={onPendingWrite}
                        />
                    </div>
                ))}
            </div>
        </div>
    );
}
