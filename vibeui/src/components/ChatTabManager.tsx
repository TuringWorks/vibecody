import { useState, useEffect, useRef, useCallback } from "react";
import { AIChat, Message } from "./AIChat";
import { ChatMemoryPanel } from "./ChatMemoryPanel";
import { useSessionMemory } from "../hooks/useSessionMemory";

interface ChatTab {
    id: string;
    title: string;
    provider: string;
    /** True if the user manually changed the provider via the per-tab dropdown */
    manualOverride: boolean;
}

/** Persisted session snapshot */
interface ChatSession {
    id: string;
    title: string;
    provider: string;
    messages: Message[];
    savedAt: number;
}

interface ChatTabManagerProps {
    defaultProvider: string;
    availableProviders: string[];
    context?: string;
    fileTree?: string[];
    currentFile?: string | null;
    onPendingWrite?: (path: string, content: string) => void;
}

const STORAGE_KEY = "vibecody:chat-sessions";
const HISTORY_KEY = "vibecody:chat-history";
const MAX_HISTORY = 50;

function loadPersistedSessions(): Record<string, Message[]> {
    try {
        const raw = localStorage.getItem(STORAGE_KEY);
        return raw ? JSON.parse(raw) : {};
    } catch { return {}; }
}

function savePersistedSessions(sessions: Record<string, Message[]>) {
    try { localStorage.setItem(STORAGE_KEY, JSON.stringify(sessions)); } catch { /* quota */ }
}

function loadHistory(): ChatSession[] {
    try {
        const raw = localStorage.getItem(HISTORY_KEY);
        return raw ? JSON.parse(raw) : [];
    } catch { return []; }
}

function saveHistory(history: ChatSession[]) {
    try { localStorage.setItem(HISTORY_KEY, JSON.stringify(history.slice(0, MAX_HISTORY))); } catch { /* quota */ }
}

let nextTabId = 1;

const ADVENTURE_NAMES = [
  "Uncharted Waters", "The Lost Meridian", "Edge of the Map", "Stormbreak",
  "The Iron Compass", "Ember Ridge", "Voidtide", "Last Horizon",
  "The Silent Expanse", "Frostfall", "Ironwood Vale", "The Amber Route",
  "Deeprun", "Skyrift", "Thornwatch", "The Wandering Star",
  "Ashgate", "Duskward", "The Ruined Coast", "Brightfall",
  "Mistkeep", "Sunken Archive", "The Final League", "Coldveil",
  "Hearthless", "Dawnseeker", "The Forgotten Shore", "Ironclad Run",
  "The Open Reach", "Starfall Pass",
];

let adventureIdx = Math.floor(Math.random() * ADVENTURE_NAMES.length);
function nextAdventureName(): string {
  const name = ADVENTURE_NAMES[adventureIdx % ADVENTURE_NAMES.length];
  adventureIdx++;
  return name;
}

export function ChatTabManager({
    defaultProvider,
    availableProviders,
    context,
    fileTree,
    currentFile,
    onPendingWrite,
}: ChatTabManagerProps) {
    // Restore persisted sessions on mount
    const initialSessions = useRef(loadPersistedSessions());

    // ── Session memory ─────────────────────────────────────────────────────────
    const memory = useSessionMemory();

    const [tabs, setTabs] = useState<ChatTab[]>([
        { id: "tab-1", title: nextAdventureName(), provider: defaultProvider, manualOverride: false },
    ]);
    const [activeTabId, setActiveTabId] = useState("tab-1");
    const [showHistory, setShowHistory] = useState(false);
    const [history, setHistory] = useState<ChatSession[]>(loadHistory);

    // Per-tab message storage (lifted from AIChat)
    const [tabMessages, setTabMessages] = useState<Record<string, Message[]>>(() => {
        return initialSessions.current;
    });

    // Persist messages to localStorage on change (debounced)
    const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
    useEffect(() => {
        if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
        saveTimerRef.current = setTimeout(() => {
            savePersistedSessions(tabMessages);
        }, 500);
        return () => { if (saveTimerRef.current) clearTimeout(saveTimerRef.current); };
    }, [tabMessages]);

    const tabMessagesRef = useRef(tabMessages);
    tabMessagesRef.current = tabMessages;

    const getMessages = useCallback((tabId: string): Message[] => {
        return tabMessagesRef.current[tabId] ?? [];
    }, []);

    const setMessagesForTab = useCallback((tabId: string, msgs: Message[] | ((prev: Message[]) => Message[])) => {
        setTabMessages(prev => {
            const current = prev[tabId] ?? [];
            const next = typeof msgs === "function" ? msgs(current) : msgs;
            // Extract new facts from any new assistant messages
            memory.extractFromMessages(next, tabId);
            return { ...prev, [tabId]: next };
        });
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

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
            title: nextAdventureName(),
            provider: defaultProvider,
            manualOverride: false,
        };
        setTabs((prev) => [...prev, newTab]);
        setActiveTabId(newTab.id);
    };

    // Track which tabs have already been saved to history to prevent duplicates.
    const savedTabsRef = useRef<Set<string>>(new Set());

    const closeTab = (id: string, e: React.MouseEvent) => {
        e.stopPropagation();
        if (tabs.length === 1) return;

        // Save to history before closing if there are messages and
        // the tab hasn't already been saved (prevents duplicates when
        // the user clicks "Save" then immediately closes the tab).
        const msgs = tabMessages[id] ?? [];
        if (msgs.length > 0 && !savedTabsRef.current.has(id)) {
            const tab = tabs.find(t => t.id === id);
            const session: ChatSession = {
                id: `session-${Date.now()}`,
                title: tab?.title ?? id,
                provider: tab?.provider ?? defaultProvider,
                messages: msgs,
                savedAt: Date.now(),
            };
            const updated = [session, ...history].slice(0, MAX_HISTORY);
            setHistory(updated);
            saveHistory(updated);
        }
        savedTabsRef.current.delete(id);

        // Clean up persisted messages
        setTabMessages(prev => {
            const next = { ...prev };
            delete next[id];
            return next;
        });

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

    const resetTabProvider = (id: string) => {
        setTabs((prev) =>
            prev.map((t) =>
                t.id === id
                    ? { ...t, provider: defaultProvider, manualOverride: false }
                    : t
            )
        );
    };

    /** Restore a session from history into a new tab */
    const restoreSession = (session: ChatSession) => {
        nextTabId++;
        const newTab: ChatTab = {
            id: `tab-${nextTabId}`,
            title: session.title,
            provider: session.provider || defaultProvider,
            manualOverride: !!session.provider,
        };
        setTabs(prev => [...prev, newTab]);
        setTabMessages(prev => ({ ...prev, [newTab.id]: session.messages }));
        setActiveTabId(newTab.id);
        setShowHistory(false);
    };

    const deleteHistorySession = (sessionId: string, e: React.MouseEvent) => {
        e.stopPropagation();
        const updated = history.filter(h => h.id !== sessionId);
        setHistory(updated);
        saveHistory(updated);
    };

    const clearHistory = () => {
        setHistory([]);
        saveHistory([]);
    };

    /** Save current tab to history explicitly */
    const saveCurrentToHistory = () => {
        const msgs = tabMessages[activeTabId] ?? [];
        if (msgs.length === 0) return;
        const tab = tabs.find(t => t.id === activeTabId);
        const firstUserMsg = msgs.find(m => m.role === "user");
        const title = firstUserMsg
            ? firstUserMsg.content.slice(0, 50) + (firstUserMsg.content.length > 50 ? "..." : "")
            : tab?.title ?? activeTabId;
        const session: ChatSession = {
            id: `session-${Date.now()}`,
            title,
            provider: tab?.provider ?? defaultProvider,
            messages: msgs,
            savedAt: Date.now(),
        };
        const updated = [session, ...history].slice(0, MAX_HISTORY);
        setHistory(updated);
        saveHistory(updated);
        // Mark tab as saved so closeTab won't create a duplicate entry
        savedTabsRef.current.add(activeTabId);
    };

    // ── Inline tab rename ──────────────────────────────────────────────────────
    const [editingTabId, setEditingTabId] = useState<string | null>(null);
    const [editingTitle, setEditingTitle] = useState("");
    const renameInputRef = useRef<HTMLInputElement>(null);

    const startRename = (tab: ChatTab, e: React.MouseEvent) => {
        e.stopPropagation();
        setEditingTabId(tab.id);
        setEditingTitle(tab.title);
        // Focus input on next tick after render
        setTimeout(() => renameInputRef.current?.select(), 0);
    };

    const commitRename = () => {
        if (editingTabId) {
            const trimmed = editingTitle.trim();
            if (trimmed) {
                setTabs(prev => prev.map(t => t.id === editingTabId ? { ...t, title: trimmed } : t));
            }
        }
        setEditingTabId(null);
    };

    const cancelRename = () => setEditingTabId(null);

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

    const formatDate = (ts: number) => {
        const d = new Date(ts);
        const now = new Date();
        if (d.toDateString() === now.toDateString()) {
            return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
        }
        return d.toLocaleDateString([], { month: "short", day: "numeric" }) + " " +
            d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    };

    return (
        <div className="panel-container">
            {/* Tab strip */}
            <div className="panel-tab-bar" style={{ alignItems: "center", gap: "1px", overflowX: "auto", minHeight: "32px" }}>
                {tabs.map((tab) => {
                    const msgCount = (tabMessages[tab.id] ?? []).length;
                    return (
                        <div
                            key={tab.id}
                            onClick={() => { if (editingTabId !== tab.id) { setActiveTabId(tab.id); setShowHistory(false); } }}
                            className={`panel-tab ${activeTabId === tab.id && !showHistory ? "active" : ""}`}
                            style={{ display: "flex", alignItems: "center", gap: "4px", flexShrink: 0, userSelect: "none" }}
                        >
                            {editingTabId === tab.id ? (
                                <input
                                    ref={renameInputRef}
                                    value={editingTitle}
                                    onChange={e => setEditingTitle(e.target.value)}
                                    onBlur={commitRename}
                                    onKeyDown={e => {
                                        if (e.key === "Enter") { e.preventDefault(); commitRename(); }
                                        if (e.key === "Escape") { e.preventDefault(); cancelRename(); }
                                    }}
                                    onClick={e => e.stopPropagation()}
                                    style={{
                                        background: "var(--bg-primary)",
                                        border: "1px solid var(--accent-blue)",
                                        color: "var(--text-primary)",
                                        borderRadius: 3, padding: "0 4px",
                                        fontSize: "12px", width: `${Math.max(editingTitle.length, 6)}ch`,
                                        outline: "none",
                                    }}
                                    autoFocus
                                />
                            ) : (
                                <span
                                    onDoubleClick={e => startRename(tab, e)}
                                    title="Double-click to rename"
                                >
                                    {tab.title}
                                </span>
                            )}
                            {msgCount > 0 && editingTabId !== tab.id && (
                                <span style={{ fontSize: "10px", color: "var(--text-secondary)", opacity: 0.7 }}>({msgCount})</span>
                            )}
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
                    );
                })}
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

                {/* History button */}
                <button
                    onClick={() => setShowHistory(prev => !prev)}
                    title="Session history"
                    style={{
                        background: showHistory ? "var(--bg-primary)" : "none",
                        border: "none", color: showHistory ? "var(--accent-color)" : "var(--text-secondary)",
                        cursor: "pointer", padding: "4px 8px", fontSize: "12px",
                        lineHeight: 1, flexShrink: 0,
                        borderBottom: showHistory ? "2px solid var(--accent-color)" : "2px solid transparent",
                    }}
                >
                    History{history.length > 0 ? ` (${history.length})` : ""}
                </button>

                {/* Save current session button */}
                {!showHistory && (tabMessages[activeTabId] ?? []).length > 0 && (
                    <button
                        onClick={saveCurrentToHistory}
                        title="Save current session to history"
                        style={{
                            background: "none", border: "none", color: "var(--text-secondary)",
                            cursor: "pointer", padding: "4px 8px", fontSize: "11px",
                            flexShrink: 0,
                        }}
                    >
                        Save
                    </button>
                )}

                {/* Per-tab model selector */}
                {!showHistory && activeTab && availableProviders.length > 1 && (
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

            {/* History panel */}
            {showHistory && (
                <div style={{ flex: 1, overflow: "auto", padding: 12 }}>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
                        <h3 style={{ margin: 0, fontSize: 14, fontWeight: 600, color: "var(--text-primary)" }}>
                            Session History
                        </h3>
                        {history.length > 0 && (
                            <button
                                onClick={clearHistory}
                                style={{
                                    background: "none", border: "1px solid var(--border-color)",
                                    color: "var(--text-secondary)", cursor: "pointer",
                                    padding: "2px 8px", fontSize: "11px", borderRadius: 4,
                                }}
                            >
                                Clear All
                            </button>
                        )}
                    </div>
                    {history.length === 0 ? (
                        <div style={{ color: "var(--text-secondary)", fontSize: 13, textAlign: "center", padding: 24 }}>
                            No saved sessions yet. Sessions are auto-saved when you close a tab with messages.
                        </div>
                    ) : (
                        history.map(session => {
                            const userMsgs = session.messages.filter(m => m.role === "user").length;
                            const assistantMsgs = session.messages.filter(m => m.role === "assistant").length;
                            const preview = session.messages.find(m => m.role === "user")?.content.slice(0, 80) ?? "";
                            return (
                                <div
                                    key={session.id}
                                    onClick={() => restoreSession(session)}
                                    style={{
                                        background: "var(--bg-secondary)", borderRadius: 6,
                                        padding: "10px 12px", marginBottom: 6,
                                        border: "1px solid var(--border-color)", cursor: "pointer",
                                    }}
                                >
                                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                                        <div style={{ flex: 1, minWidth: 0 }}>
                                            <div style={{ fontWeight: 600, fontSize: 13, color: "var(--text-primary)", marginBottom: 2, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                                                {session.title}
                                            </div>
                                            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 }}>
                                                {session.provider} · {userMsgs} questions · {assistantMsgs} responses · {formatDate(session.savedAt)}
                                            </div>
                                            {preview && (
                                                <div style={{ fontSize: 11, color: "var(--text-secondary)", opacity: 0.7, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                                                    {preview}
                                                </div>
                                            )}
                                        </div>
                                        <div style={{ display: "flex", gap: 4, flexShrink: 0, marginLeft: 8 }}>
                                            <button
                                                onClick={(e) => { e.stopPropagation(); restoreSession(session); }}
                                                title="Restore into new tab"
                                                style={{
                                                    background: "var(--bg-tertiary)", border: "1px solid var(--border-color)",
                                                    color: "var(--text-primary)", cursor: "pointer",
                                                    padding: "2px 8px", fontSize: "11px", borderRadius: 3,
                                                }}
                                            >
                                                Restore
                                            </button>
                                            <button
                                                onClick={(e) => deleteHistorySession(session.id, e)}
                                                title="Delete from history"
                                                style={{
                                                    background: "none", border: "1px solid var(--border-color)",
                                                    color: "var(--text-secondary)", cursor: "pointer",
                                                    padding: "2px 6px", fontSize: "11px", borderRadius: 3,
                                                }}
                                            >
                                                ×
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            );
                        })
                    )}
                </div>
            )}

            {/* Tab content — render all tabs but only show active, preserving state */}
            <div style={{ flex: 1, overflow: "hidden", display: showHistory ? "none" : "block" }}>
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
                            availableProviders={availableProviders}
                            onProviderChange={(p) => setTabProvider(tab.id, p)}
                            messages={getMessages(tab.id)}
                            onMessagesChange={(msgs) => setMessagesForTab(tab.id, msgs)}
                            pinnedMemory={memory.getPinnedSystemPromptText() || undefined}
                        />
                    </div>
                ))}
            </div>

            {/* Memory panel — shown below chat content, hidden while history panel is open */}
            {!showHistory && (
                <ChatMemoryPanel
                    facts={memory.factsForTab(activeTabId)}
                    tabId={activeTabId}
                    onPin={memory.pinFact}
                    onUnpin={memory.unpinFact}
                    onDelete={memory.deleteFact}
                    onEdit={memory.editFact}
                    onAddManual={(text) => memory.addManual(text, activeTabId)}
                />
            )}
        </div>
    );
}
