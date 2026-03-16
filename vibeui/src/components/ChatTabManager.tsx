import { useState, useEffect, useRef } from "react";
import { AIChat } from "./AIChat";

interface ChatTab {
    id: string;
    title: string;
    provider: string;
    /** True if the user manually changed the provider via the per-tab dropdown */
    manualOverride: boolean;
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
        { id: "tab-1", title: "Chat 1", provider: defaultProvider, manualOverride: false },
    ]);
    const [activeTabId, setActiveTabId] = useState("tab-1");

    // When the top-bar provider changes, update all tabs that haven't been
    // manually overridden by the user.
    const prevProvider = useRef(defaultProvider);
    useEffect(() => {
        if (defaultProvider && defaultProvider !== prevProvider.current) {
            prevProvider.current = defaultProvider;
            setTabs((prev) =>
                prev.map((t) =>
                    t.manualOverride ? t : { ...t, provider: defaultProvider }
                )
            );
        }
        // Also fix initial empty provider on first load
        if (defaultProvider) {
            setTabs((prev) =>
                prev.map((t) =>
                    t.provider === "" ? { ...t, provider: defaultProvider } : t
                )
            );
        }
    }, [defaultProvider]);

    const addTab = () => {
        nextTabId++;
        const newTab: ChatTab = {
            id: `tab-${nextTabId}`,
            title: `Chat ${nextTabId}`,
            provider: defaultProvider,
            manualOverride: false,
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
            prev.map((t) =>
                t.id === id ? { ...t, provider, manualOverride: true } : t
            )
        );
    };

    /** Reset a tab back to following the top-bar provider */
    const resetTabProvider = (id: string) => {
        setTabs((prev) =>
            prev.map((t) =>
                t.id === id
                    ? { ...t, provider: defaultProvider, manualOverride: false }
                    : t
            )
        );
    };

    const activeTab = tabs.find((t) => t.id === activeTabId);

    // Per-tab injected context (from Cascade panel "Inject into chat")
    const [injectedText, setInjectedText] = useState<Record<string, string>>({});

    useEffect(() => {
        const handler = (e: Event) => {
            const text = (e as CustomEvent<string>).detail;
            if (!text) return;
            setInjectedText((prev) => ({
                ...prev,
                [activeTabId]: (prev[activeTabId] ? prev[activeTabId] + "\n" : "") + text,
            }));
        };
        window.addEventListener("vibeui:inject-context", handler);
        return () => window.removeEventListener("vibeui:inject-context", handler);
    }, [activeTabId]);

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
                                ? "2px solid var(--accent-blue)"
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
                    <div style={{ marginLeft: "auto", marginRight: "6px", display: "flex", alignItems: "center", gap: "4px" }}>
                        <select
                            value={activeTab.provider}
                            onChange={(e) => setTabProvider(activeTab.id, e.target.value)}
                            style={{
                                padding: "2px 4px", fontSize: "11px",
                                background: "var(--bg-tertiary)", border: "1px solid var(--border-color)",
                                color: activeTab.manualOverride ? "var(--accent-gold)" : "var(--text-secondary)",
                                borderRadius: "3px",
                                maxWidth: "160px",
                            }}
                            title={activeTab.manualOverride
                                ? "Manually overridden — click reset to follow top bar"
                                : "Following top bar selection"
                            }
                        >
                            {availableProviders.map((p) => (
                                <option key={p} value={p}>{p}</option>
                            ))}
                        </select>
                        {activeTab.manualOverride && (
                            <button
                                onClick={() => resetTabProvider(activeTab.id)}
                                style={{
                                    background: "none", border: "none",
                                    color: "var(--text-secondary)", cursor: "pointer",
                                    fontSize: "10px", padding: "2px 4px",
                                    borderRadius: "3px",
                                }}
                                title="Reset to follow top bar selection"
                            >
                                reset
                            </button>
                        )}
                    </div>
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
                            pendingInput={activeTabId === tab.id ? injectedText[tab.id] : undefined}
                            onPendingInputConsumed={() =>
                                setInjectedText((prev) => { const next = { ...prev }; delete next[tab.id]; return next; })
                            }
                        />
                    </div>
                ))}
            </div>
        </div>
    );
}
