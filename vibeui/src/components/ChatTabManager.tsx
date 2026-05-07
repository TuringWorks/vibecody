import { useState, useEffect, useRef, useCallback } from "react";
import { Icon } from "./Icon";
import { invoke } from "@tauri-apps/api/core";
import { AIChat, Message } from "./AIChat";
import { ChatMemoryPanel } from "./ChatMemoryPanel";
import { RecapCard } from "./RecapCard";
import { useSessionMemory } from "../hooks/useSessionMemory";
import { useWatchActiveSession } from "../hooks/useWatchSync";
import type { Recap } from "../types/recap";

/** Last error surfaced to the user — recap-resume failure messages, etc.
 * Cleared automatically after 6s or when the user dismisses. Only one
 * banner at a time; new errors replace the previous one. */
interface InlineError {
    id: number;
    message: string;
}

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
    /** F2.2 — daemon-side `subject_id` for the matching `Recap` row.
     * Optional: history entries created before recap-resume shipped
     * won't have one, and the UI degrades to no-card. */
    recapSubjectId?: string;
}

interface ChatTabManagerProps {
    defaultProvider: string;
    availableProviders: string[];
    context?: string;
    fileTree?: string[];
    currentFile?: string | null;
    onPendingWrite?: (path: string, content: string) => void;
}

const LEGACY_SESSIONS_KEY = "vibecody:chat-sessions";
const HISTORY_KEY = "vibecody:chat-history";
const MAX_HISTORY = 50;

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

// Module-level cache: seeded with defaults so tab creation is always synchronous.
// Updated from ~/.vibeui/adventure-names.json once the backend responds.
let adventureNames: string[] = [
  "Uncharted Waters", "The Lost Meridian", "Edge of the Map", "Stormbreak",
  "The Iron Compass", "Ember Ridge", "Voidtide", "Last Horizon",
  "The Silent Expanse", "Frostfall", "Ironwood Vale", "The Amber Route",
  "Deeprun", "Skyrift", "Thornwatch", "The Wandering Star",
  "Ashgate", "Duskward", "The Ruined Coast", "Brightfall",
  "Mistkeep", "Sunken Archive", "The Final League", "Coldveil",
  "Hearthless", "Dawnseeker", "The Forgotten Shore", "Ironclad Run",
  "The Open Reach", "Starfall Pass",
];
let adventureIdx = Math.floor(Math.random() * adventureNames.length);

function nextAdventureName(): string {
  const name = adventureNames[adventureIdx % adventureNames.length];
  adventureIdx++;
  return name;
}

/** Fetch names from the backend and refresh the module cache. */
async function refreshAdventureNames(): Promise<void> {
  try {
    const names = await invoke<string[]>("get_adventure_names");
    if (names.length > 0) adventureNames = names;
  } catch { /* backend unavailable during dev — keep defaults */ }
}

export function ChatTabManager({
    defaultProvider,
    availableProviders,
    context,
    fileTree,
    currentFile,
    onPendingWrite,
}: ChatTabManagerProps) {
    // Refresh adventure names from backend once on mount (non-blocking).
    // Also drop the legacy per-tab message blob — we no longer auto-restore
    // tab messages across app launches; History is the source of truth for
    // past chats, and the user expects a fresh window on open / "+".
    useEffect(() => {
        refreshAdventureNames();
        try { localStorage.removeItem(LEGACY_SESSIONS_KEY); } catch { /* ignore */ }
    }, []);

    // ── Session memory ─────────────────────────────────────────────────────────
    const memory = useSessionMemory();

    const [tabs, setTabs] = useState<ChatTab[]>([
        { id: "tab-1", title: nextAdventureName(), provider: defaultProvider, manualOverride: false },
    ]);
    const [activeTabId, setActiveTabId] = useState("tab-1");

    // Google Docs-style sync: when Watch switches to a session, VibeUI follows.
    useWatchActiveSession((watchSessionId) => {
        // Only switch if the session exists as a tab
        if (tabs.some(t => t.id === watchSessionId)) {
            setActiveTabId(watchSessionId);
        }
    });

    const [showHistory, setShowHistory] = useState(false);
    const [showMemoryDialog, setShowMemoryDialog] = useState(false);
    const [history, setHistory] = useState<ChatSession[]>(loadHistory);
    const [inlineError, setInlineError] = useState<InlineError | null>(null);
    const errorIdRef = useRef(0);

    /** Show a transient error banner. Auto-dismisses after 6s.
     * Wired into recap-generate / recap-resume failure paths so users
     * actually see when the daemon is offline instead of silently losing
     * the action. */
    const showError = useCallback((message: string) => {
        const id = ++errorIdRef.current;
        setInlineError({ id, message });
        setTimeout(() => {
            setInlineError(prev => (prev && prev.id === id ? null : prev));
        }, 6000);
    }, []);

    const dismissError = useCallback(() => setInlineError(null), []);

    // Per-tab message storage (lifted from AIChat). Lives in React state for
    // the lifetime of the panel — not persisted to localStorage. Closed tabs
    // with messages are auto-saved to History (see closeTab below).
    const [tabMessages, setTabMessages] = useState<Record<string, Message[]>>({});

    // tab.id → history session id. A tab gets bound to a history entry the
    // first time it's saved (or when restored from history). Subsequent saves
    // update that same entry instead of stacking duplicates.
    const [tabHistoryIds, setTabHistoryIds] = useState<Record<string, string>>({});

    // F2.2 — tab.id → currently-pinned Recap (or null). Set by restoreSession
    // when the history entry has a `recapSubjectId`. Cleared when the user
    // dismisses the card or resumes from it.
    const [tabRecaps, setTabRecaps] = useState<Record<string, Recap | null>>({});

    // tab.id → agent-loop toggle. When true, AIChat routes sendMessage through
    // start_agent_task instead of stream_chat_message. Default off; opt-in only
    // until Phase 3 flips the global default. Not persisted across app launches
    // because there is only one global agent task slot — surfacing it as a
    // sticky setting would let two tabs both think they own it.
    const [tabAgentLoop, setTabAgentLoop] = useState<Record<string, boolean>>({});
    const setAgentLoopForTab = useCallback((tabId: string, on: boolean) => {
        setTabAgentLoop(prev => ({ ...prev, [tabId]: on }));
    }, []);

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

    // When the top-bar provider changes, update ALL tabs and clear any
    // per-tab manual override. The top-bar is the global default and an
    // explicit user action — it always wins. Users who want per-tab
    // divergence can re-pick from the per-tab dropdown afterwards.
    const prevProvider = useRef(defaultProvider);
    useEffect(() => {
        if (defaultProvider && defaultProvider !== prevProvider.current) {
            prevProvider.current = defaultProvider;
            setTabs((prev) =>
                prev.map((t) => ({ ...t, provider: defaultProvider, manualOverride: false }))
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

    /**
     * Save (or update) a tab's messages to history.
     *
     * If the tab is already bound to a history entry — because it was
     * restored from history, or saved earlier in this session — that entry
     * is updated in place and moved to the top. Otherwise a new entry is
     * created and the tab is bound to it. Returns the history id used.
     *
     * `firstMessageTitle` controls how the title is derived on the FIRST
     * save only (for the explicit Save button, which prefers the first user
     * message; closeTab uses the tab title). Updates never touch the title.
     */
    const persistTabToHistory = (tabId: string, firstMessageTitle: boolean): string | null => {
        const msgs = tabMessages[tabId] ?? [];
        if (msgs.length === 0) return null;
        const tab = tabs.find(t => t.id === tabId);
        const provider = tab?.provider ?? defaultProvider;
        const existingId = tabHistoryIds[tabId];

        if (existingId && history.some(h => h.id === existingId)) {
            const existing = history.find(h => h.id === existingId)!;
            const updatedSession: ChatSession = {
                ...existing,
                provider,
                messages: msgs,
                savedAt: Date.now(),
            };
            const updated = [
                updatedSession,
                ...history.filter(h => h.id !== existingId),
            ].slice(0, MAX_HISTORY);
            setHistory(updated);
            saveHistory(updated);
            return existingId;
        }

        const newId = `session-${Date.now()}`;
        let title = tab?.title ?? tabId;
        if (firstMessageTitle) {
            const firstUserMsg = msgs.find(m => m.role === "user");
            if (firstUserMsg) {
                title = firstUserMsg.content.slice(0, 50) + (firstUserMsg.content.length > 50 ? "..." : "");
            }
        }
        const session: ChatSession = { id: newId, title, provider, messages: msgs, savedAt: Date.now() };
        const updated = [session, ...history].slice(0, MAX_HISTORY);
        setHistory(updated);
        saveHistory(updated);
        setTabHistoryIds(prev => ({ ...prev, [tabId]: newId }));
        return newId;
    };

    /** F2.3 — read the user's "Recap on tab close" preference from
     * the F2.1 localStorage blob. Defaults to true when the key is
     * missing or corrupt so closing a tab still leaves a recap behind
     * for users who never visited Settings → Sessions. */
    const isRecapOnCloseEnabled = (): boolean => {
        try {
            const raw = localStorage.getItem("vibeui-sessions");
            if (!raw) return true;
            const parsed = JSON.parse(raw) as { recapOnTabClose?: boolean };
            return typeof parsed.recapOnTabClose === "boolean" ? parsed.recapOnTabClose : true;
        } catch {
            return true;
        }
    };

    const closeTab = (id: string, e: React.MouseEvent) => {
        e.stopPropagation();
        if (tabs.length === 1) return;

        // Save (or update) the tab's history entry on close so the latest
        // messages survive after the tab is gone.
        const historyId = persistTabToHistory(id, false);
        const msgsAtClose = tabMessages[id] ?? [];

        // F2.3 — best-effort recap generation on close. Idempotent on
        // (subject_id, last_message_id), so a no-change close is a cheap
        // server-side no-op. Skipped when the user toggled it off in
        // Settings → Sessions, or when there's nothing to recap.
        if (historyId && msgsAtClose.length > 0 && isRecapOnCloseEnabled()) {
            invoke("recap_generate", { subjectId: id })
                .then(() => {
                    // Write the recap subject back into the history entry so
                    // a future restore can fetch the matching Recap via
                    // recap_get_for_session (F2.2).
                    setHistory(prev => {
                        const next = prev.map(h => h.id === historyId ? { ...h, recapSubjectId: id } : h);
                        saveHistory(next);
                        return next;
                    });
                })
                .catch(() => { /* daemon offline or session not in sessions.db — skip */ });
        }

        // Clean up tab-scoped state
        setTabMessages(prev => {
            const next = { ...prev };
            delete next[id];
            return next;
        });
        setTabHistoryIds(prev => {
            if (!(id in prev)) return prev;
            const next = { ...prev };
            delete next[id];
            return next;
        });
        setTabRecaps(prev => {
            if (!(id in prev)) return prev;
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

    /** Restore a session from history into a new tab. The new tab is bound
     * to the same history entry, so future saves update it in place. If the
     * history entry has a `recapSubjectId`, also fetch + pin the recap card
     * above the transcript (F2.2). Recap fetch is best-effort: a daemon
     * that doesn't yet expose `recap_get_for_session` simply leaves the
     * tab without a card. */
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
        setTabHistoryIds(prev => ({ ...prev, [newTab.id]: session.id }));
        setActiveTabId(newTab.id);
        setShowHistory(false);

        if (session.recapSubjectId) {
            invoke<Recap | null>("recap_get_for_session", { subjectId: session.recapSubjectId })
                .then(rcp => { if (rcp) setTabRecaps(prev => ({ ...prev, [newTab.id]: rcp })); })
                .catch(() => { /* daemon offline or command absent — degrade silently */ });
        }
    };

    /** F2.2 — invoked by RecapCard's "Resume from here". Asks the daemon
     * to materialise a primed session via /v1/resume, then dismisses the
     * card. Failure surfaces an inline error banner so the user knows
     * the resume didn't graft a fresh prompt — the underlying transcript
     * is already restored, only the resume step failed. */
    const resumeFromRecap = useCallback((tabId: string, recap: Recap) => {
        invoke("recap_resume_session", { recapId: recap.id, branch: false })
            .catch(() => showError("Couldn't resume from recap — the daemon may be offline. Your messages are still here."));
        setTabRecaps(prev => ({ ...prev, [tabId]: null }));
    }, [showError]);

    const dismissRecap = useCallback((tabId: string) => {
        setTabRecaps(prev => ({ ...prev, [tabId]: null }));
    }, []);
    void dismissRecap; // wired in F2.2b (close-button on the card)

    const deleteHistorySession = (sessionId: string, e: React.MouseEvent) => {
        e.stopPropagation();
        const updated = history.filter(h => h.id !== sessionId);
        setHistory(updated);
        saveHistory(updated);
        // Drop any tab→history binding pointing at the deleted entry so a
        // subsequent save creates a fresh entry rather than reviving the id.
        setTabHistoryIds(prev => {
            let changed = false;
            const next: Record<string, string> = {};
            for (const [tabId, hId] of Object.entries(prev)) {
                if (hId === sessionId) { changed = true; continue; }
                next[tabId] = hId;
            }
            return changed ? next : prev;
        });
    };

    const clearHistory = () => {
        setHistory([]);
        saveHistory([]);
        setTabHistoryIds({});
    };

    /** Save current tab to history explicitly. Updates an existing entry if
     * this tab has been saved (or restored) before; otherwise creates one. */
    const saveCurrentToHistory = () => {
        persistTabToHistory(activeTabId, true);
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

    /** Keyboard navigation for the tab strip. Left/Right cycle tabs;
     * Home/End jump to first/last. Mirrors the WAI-ARIA tablist pattern.
     * Called from each tab's onKeyDown so focus stays where the user is. */
    const handleTabKeyDown = (e: React.KeyboardEvent, tabId: string) => {
        if (editingTabId === tabId) return; // rename input owns the keys
        const idx = tabs.findIndex(t => t.id === tabId);
        if (idx < 0) return;
        let next = idx;
        if (e.key === "ArrowRight") next = (idx + 1) % tabs.length;
        else if (e.key === "ArrowLeft") next = (idx - 1 + tabs.length) % tabs.length;
        else if (e.key === "Home") next = 0;
        else if (e.key === "End") next = tabs.length - 1;
        else return;
        e.preventDefault();
        const nextTab = tabs[next];
        setActiveTabId(nextTab.id);
        setShowHistory(false);
        // Defer focus to after re-render
        requestAnimationFrame(() => {
            const el = document.querySelector<HTMLElement>(`[data-tab-id="${nextTab.id}"]`);
            el?.focus();
        });
    };

    return (
        <div className="panel-container">
            {/* Tab strip — proper WAI-ARIA tablist with arrow-key navigation */}
            <div
                className="panel-tab-bar"
                role="tablist"
                aria-label="Chat sessions"
                style={{ alignItems: "center", gap: "1px", overflowX: "auto", minHeight: "32px" }}
            >
                {tabs.map((tab) => {
                    const msgCount = (tabMessages[tab.id] ?? []).length;
                    const isActive = activeTabId === tab.id && !showHistory;
                    return (
                        <div
                            key={tab.id}
                            data-tab-id={tab.id}
                            role="tab"
                            aria-selected={isActive}
                            aria-controls={`chat-tab-panel-${tab.id}`}
                            tabIndex={isActive ? 0 : -1}
                            onClick={() => { if (editingTabId !== tab.id) { setActiveTabId(tab.id); setShowHistory(false); } }}
                            onKeyDown={(e) => handleTabKeyDown(e, tab.id)}
                            className={`panel-tab ${isActive ? "active" : ""}`}
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
                                        fontSize: "var(--font-size-base)", width: `${Math.max(editingTitle.length, 6)}ch`,
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
                                <span style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", opacity: 0.7 }}>({msgCount})</span>
                            )}
                            {tabs.length > 1 && (
                                <button
                                    onClick={(e) => closeTab(tab.id, e)}
                                    style={{
                                        background: "none", border: "none", color: "inherit",
                                        cursor: "pointer", padding: "0 2px",
                                        display: "flex", alignItems: "center",
                                    }}
                                    title="Close tab"
                                    aria-label={`Close ${tab.title}`}
                                >
                                    <Icon name="x" size={12} />
                                </button>
                            )}
                        </div>
                    );
                })}
                <button
                    onClick={addTab}
                    title="New chat tab"
                    aria-label="New chat tab"
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
                    aria-pressed={showHistory}
                    aria-label={`Session history${history.length > 0 ? ` — ${history.length} saved` : ""}`}
                    style={{
                        background: showHistory ? "var(--bg-primary)" : "none",
                        border: "none", color: showHistory ? "var(--accent-color)" : "var(--text-secondary)",
                        cursor: "pointer", padding: "4px 8px", fontSize: "var(--font-size-base)",
                        lineHeight: 1, flexShrink: 0,
                        borderBottom: showHistory ? "2px solid var(--accent-color)" : "2px solid transparent",
                    }}
                >
                    History{history.length > 0 ? ` (${history.length})` : ""}
                </button>

                {/* Memory button */}
                {!showHistory && (() => {
                    const memFacts = memory.factsForTab(activeTabId);
                    const pinned = memFacts.filter((f) => f.pinned).length;
                    const total = memFacts.length;
                    return (
                        <button
                            onClick={() => setShowMemoryDialog(true)}
                            title="Chat memory"
                            aria-pressed={showMemoryDialog}
                            aria-label={`Chat memory${total > 0 ? ` — ${total} fact${total === 1 ? "" : "s"}${pinned > 0 ? `, ${pinned} pinned` : ""}` : ""}`}
                            style={{
                                background: showMemoryDialog ? "var(--bg-primary)" : "none",
                                border: "none",
                                color: pinned > 0 ? "var(--accent-blue, #3b82f6)" : "var(--text-secondary)",
                                cursor: "pointer", padding: "4px 8px", fontSize: "var(--font-size-base)",
                                lineHeight: 1, flexShrink: 0, display: "flex", alignItems: "center", gap: 4,
                                borderBottom: showMemoryDialog ? "2px solid var(--accent-blue, #3b82f6)" : "2px solid transparent",
                            }}
                        >
                            Memory
                            {total > 0 && (
                                <span style={{
                                    background: pinned > 0 ? "var(--accent-blue, #3b82f6)" : "var(--bg-tertiary)",
                                    color: pinned > 0 ? "#fff" : "var(--text-secondary)",
                                    borderRadius: "var(--radius-md)", padding: "0 5px", fontSize: "var(--font-size-xs)", lineHeight: "16px",
                                }}>
                                    {total}
                                </span>
                            )}
                        </button>
                    );
                })()}

                {/* Save current session button */}
                {!showHistory && (tabMessages[activeTabId] ?? []).length > 0 && (
                    <button
                        onClick={saveCurrentToHistory}
                        title="Save current session to history"
                        style={{
                            background: "none", border: "none", color: "var(--text-secondary)",
                            cursor: "pointer", padding: "4px 8px", fontSize: "var(--font-size-sm)",
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
                                padding: "2px 4px", fontSize: "var(--font-size-sm)",
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
                                    fontSize: "var(--font-size-xs)", padding: "2px 4px",
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
                        <h3 style={{ margin: 0, fontSize: "var(--font-size-lg)", fontWeight: 600, color: "var(--text-primary)" }}>
                            Session History
                        </h3>
                        {history.length > 0 && (
                            <button
                                onClick={clearHistory}
                                style={{
                                    background: "none", border: "1px solid var(--border-color)",
                                    color: "var(--text-secondary)", cursor: "pointer",
                                    padding: "2px 8px", fontSize: "var(--font-size-sm)", borderRadius: "var(--radius-xs-plus)",
                                }}
                            >
                                Clear All
                            </button>
                        )}
                    </div>
                    {history.length === 0 ? (
                        <div style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-md)", textAlign: "center", padding: 24 }}>
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
                                        background: "var(--bg-secondary)", borderRadius: "var(--radius-sm)",
                                        padding: "10px 12px", marginBottom: 6,
                                        border: "1px solid var(--border-color)", cursor: "pointer",
                                    }}
                                >
                                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                                        <div style={{ flex: 1, minWidth: 0 }}>
                                            <div style={{ fontWeight: 600, fontSize: "var(--font-size-md)", color: "var(--text-primary)", marginBottom: 2, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                                                {session.title}
                                            </div>
                                            <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: 4 }}>
                                                {session.provider} · {userMsgs} questions · {assistantMsgs} responses · {formatDate(session.savedAt)}
                                            </div>
                                            {preview && (
                                                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", opacity: 0.7, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
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
                                                    padding: "2px 8px", fontSize: "var(--font-size-sm)", borderRadius: 3,
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
                                                    padding: "2px 6px", borderRadius: 3,
                                                    display: "flex", alignItems: "center",
                                                }}
                                            >
                                                <Icon name="x" size={11} />
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            );
                        })
                    )}
                </div>
            )}

            {/* Inline error banner — recap-resume failure, etc. role="alert"
                so AT announces the message immediately. The transcript is
                always still intact below; this banner only signals that an
                async background action couldn't be completed. */}
            {inlineError && (
                <div
                    role="alert"
                    style={{
                        margin: "6px 12px 0",
                        padding: "8px 12px",
                        background: "var(--error-bg, #5b1a1a)",
                        border: "1px solid var(--accent-rose, #ef4444)",
                        borderRadius: "var(--radius-xs-plus)",
                        color: "var(--text-primary)",
                        fontSize: "var(--font-size-sm)",
                        display: "flex", justifyContent: "space-between", alignItems: "center",
                        gap: 8,
                    }}
                >
                    <span>{inlineError.message}</span>
                    <button
                        type="button"
                        onClick={dismissError}
                        aria-label="Dismiss error"
                        style={{
                            background: "none", border: "none", color: "inherit",
                            cursor: "pointer", padding: "2px 6px", fontSize: "var(--font-size-sm)",
                        }}
                    >
                        Dismiss
                    </button>
                </div>
            )}

            {/* Tab content — render all tabs but only show active, preserving state */}
            <div style={{ flex: 1, overflow: "hidden", display: showHistory ? "none" : "block" }}>
                {tabs.map((tab) => (
                    <div
                        key={tab.id}
                        id={`chat-tab-panel-${tab.id}`}
                        role="tabpanel"
                        aria-label={tab.title}
                        hidden={activeTabId !== tab.id}
                        style={{
                            display: activeTabId === tab.id ? "flex" : "none",
                            flexDirection: "column",
                            height: "100%",
                        }}
                    >
                        {tabRecaps[tab.id] && (
                            <RecapCard
                                recap={tabRecaps[tab.id]!}
                                onResume={(r) => resumeFromRecap(tab.id, r)}
                            />
                        )}
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
                            sessionId={tab.id}
                            sessionTitle={tab.title}
                            useAgentLoop={!!tabAgentLoop[tab.id]}
                            onUseAgentLoopChange={(on) => setAgentLoopForTab(tab.id, on)}
                        />
                    </div>
                ))}
            </div>

            {/* Memory dialog — floats over chat content */}
            {showMemoryDialog && (
                <div
                    onClick={() => setShowMemoryDialog(false)}
                    style={{
                        position: "fixed", inset: 0, zIndex: 1000,
                        background: "rgba(0,0,0,0.4)",
                        display: "flex", alignItems: "flex-start", justifyContent: "flex-end",
                    }}
                >
                    <div
                        onClick={(e) => e.stopPropagation()}
                        style={{
                            marginTop: 40, marginRight: 12,
                            width: 340, maxHeight: "calc(100vh - 60px)",
                            background: "var(--bg-secondary)",
                            border: "1px solid var(--border-color)",
                            borderRadius: "var(--radius-sm)", overflow: "hidden",
                            boxShadow: "0 8px 32px rgba(0,0,0,0.4)",
                            display: "flex", flexDirection: "column",
                        }}
                    >
                        <ChatMemoryPanel
                            facts={memory.factsForTab(activeTabId)}
                            tabId={activeTabId}
                            onPin={memory.pinFact}
                            onUnpin={memory.unpinFact}
                            onDelete={memory.deleteFact}
                            onEdit={memory.editFact}
                            onAddManual={(text) => memory.addManual(text, activeTabId)}
                            onClose={() => setShowMemoryDialog(false)}
                        />
                    </div>
                </div>
            )}
        </div>
    );
}
