---
layout: default
title: Hooks Reference
nav_order: 15
---

# Hooks Reference

All React hooks live in `vibeui/src/hooks/`. They provide shared state, persistence, and integration with the Tauri backend.

---

## useApiKeyMonitor

```ts
useApiKeyMonitor({ toast, addNotification, osNotifications? })
```

**Purpose:** Periodically validates all configured API keys and notifies on status changes (valid to invalid, or recovery).

**Parameters:**

- `toast` (`ToastApi`) -- toast instance from `useToast`
- `addNotification` (`(opts: AddNotificationOpts) => void`) -- from `useNotifications`
- `osNotifications?` (`boolean`) -- also send OS-level notifications for failures

**Returns:** `{ validations, lastChecked, revalidate }` -- current validation map, timestamp of last check, and a function to force re-check.

**Example:**

```ts
const { validations, revalidate } = useApiKeyMonitor({ toast, addNotification });
```

---

## useCollab

```ts
useCollab()
```

**Purpose:** Manages a WebSocket-based CRDT multiplayer collaboration session with peer awareness and cursor tracking.

**Parameters:** None.

**Returns:** `{ connected, roomId, peerId, peers, wsUrl, connect, disconnect, sendAwareness, ws }`

- `connect(wsUrl, userName)` -- join a collaboration room
- `disconnect()` -- leave the room
- `sendAwareness(file, line, column, selectionEnd?)` -- broadcast cursor position
- `peers` -- array of `CollabPeer` with name, color, and cursor info

**Example:**

```ts
const { connect, peers, sendAwareness } = useCollab();
connect("ws://localhost:4444?room=abc", "Alice");
```

---

## useDaemonMonitor

```ts
useDaemonMonitor({ toast, addNotification, daemonUrl? })
```

**Purpose:** Polls the VibeCLI daemon health endpoint and auto-starts it via Tauri if offline, notifying on status transitions.

**Parameters:**

- `toast` (`ToastApi`) -- toast instance
- `addNotification` (`(opts: AddNotificationOpts) => void`)
- `daemonUrl?` (`string`) -- defaults to `"http://localhost:7878"`

**Returns:** `{ online, lastChecked, recheck }` -- daemon online status, timestamp, and manual re-check function.

**Example:**

```ts
const { online } = useDaemonMonitor({ toast, addNotification });
```

---

## useEditorTheme

```ts
useEditorTheme()
```

**Purpose:** Generates and registers a Monaco editor theme from the active VibeUI CSS variable theme, keeping them in sync.

**Parameters:** None.

**Returns:** `{ themeName, defineTheme }`

- `themeName` (`string`) -- the Monaco theme name to pass to `<Editor theme={...}>`
- `defineTheme(monaco)` -- call from `onMount` to capture the monaco instance and apply the initial theme

**Example:**

```tsx
const { themeName, defineTheme } = useEditorTheme();
<Editor theme={themeName} onMount={(_, monaco) => defineTheme(monaco)} />
```

---

## useModelRegistry

```ts
useModelRegistry()
```

**Purpose:** Provides a cached provider-to-model matrix (2-hour TTL in localStorage) consumed by all model-selection dropdowns.

**Parameters:** None.

**Returns:** `{ providers, modelsForProvider, loading, refresh, lastUpdated }`

- `providers` (`string[]`) -- all known provider names
- `modelsForProvider(provider)` (`string[]`) -- models available for a provider
- `refresh()` -- force-refresh from the backend (e.g., re-fetch Ollama models)

**Example:**

```ts
const { providers, modelsForProvider } = useModelRegistry();
const models = modelsForProvider("openai"); // ["gpt-4o", "gpt-4o-mini", ...]
```

---

## useNotifications

```ts
useNotifications()
```

**Purpose:** Centralized in-memory notification store (up to 100) for app-level alerts that persist across panel switches.

**Parameters:** None.

**Returns:** `{ notifications, add, markRead, markAllRead, dismiss, clearCategory, unreadCount }`

- `add(opts)` -- create a notification with `title`, `body`, `severity`, `category`, and optional `action`
- `dismiss(id)` -- remove a notification
- `clearCategory(category)` -- remove all notifications in a category

**Example:**

```ts
const { add, unreadCount } = useNotifications();
add({ title: "Build failed", body: "Exit code 1", severity: "error", category: "build" });
```

---

## usePanelSettings

```ts
usePanelSettings(panelName: string)
```

**Purpose:** Reads and writes per-panel settings from the encrypted SQLite backend, scoped by profile and panel name.

**Parameters:**

- `panelName` (`string`) -- identifies which panel's settings to load

**Returns:** `{ settings, loading, profileId, setSetting, deleteSetting, resetPanel, reload, switchProfile, error }`

- `settings` (`Record<string, any>`) -- all saved key-value pairs for this panel
- `setSetting(key, value)` -- persist a single value immediately
- `resetPanel()` -- delete all settings for this panel

**Example:**

```ts
const { settings, setSetting } = usePanelSettings("terminal");
await setSetting("fontSize", 14);
```

---

## usePersistentState

```ts
usePersistentState<T>(key: string, initialValue: T)
```

**Purpose:** Drop-in replacement for `useState` that persists values to localStorage, surviving tab switches and app restarts.

**Parameters:**

- `key` (`string`) -- storage key (auto-prefixed with `"vibeui-panel:"`)
- `initialValue` (`T`) -- fallback when no stored value exists

**Returns:** `[value, setValue]` -- same API as `useState`.

**Example:**

```ts
const [filter, setFilter] = usePersistentState<string>("search.filter", "");
```

---

## useSessionMemory

```ts
useSessionMemory()
```

**Purpose:** Extracts and manages per-tab memory facts from AI assistant messages, with pinning support for cross-tab persistence.

**Parameters:** None.

**Returns:** `{ facts, factsForTab, extractFromMessages, addManual, pinFact, unpinFact, deleteFact, editFact, clearTabFacts, getPinnedSystemPromptText }`

- `extractFromMessages(messages, tabId)` -- scan new messages for memorable facts
- `pinFact(id)` / `unpinFact(id)` -- pinned facts persist in localStorage and are injected into the AI system prompt
- `getPinnedSystemPromptText()` -- formatted string of pinned facts for prompt injection

**Example:**

```ts
const { extractFromMessages, getPinnedSystemPromptText } = useSessionMemory();
extractFromMessages(chatMessages, "tab-1");
```

---

## useToast

```ts
useToast()
```

**Purpose:** Lightweight ephemeral toast notification system with auto-dismiss (3s for success/info, 4s for warn, 6s for error).

**Parameters:** None.

**Returns:** `{ toasts, toast, dismiss }`

- `toast.success(msg)`, `toast.error(msg)`, `toast.info(msg)`, `toast.warn(msg)` -- show a toast
- `dismiss(id)` -- manually dismiss a toast
- `toasts` (`Toast[]`) -- current list for rendering

**Example:**

```ts
const { toast } = useToast();
toast.success("File saved!");
```

---

## useVoiceInput

```ts
useVoiceInput(onTranscript: (text: string) => void)
```

**Purpose:** Provides voice-to-text input using the Web Speech API, with a fallback to MediaRecorder + Groq Whisper transcription via Tauri.

**Parameters:**

- `onTranscript` (`(text: string) => void`) -- callback invoked with transcribed text

**Returns:** `{ isListening, isTranscribing, interimText, toggle }`

- `toggle()` -- start or stop listening
- `isListening` -- whether the microphone is active
- `isTranscribing` -- whether the fallback recorder is processing audio
- `interimText` -- partial transcript while speaking (Speech API only)

**Example:**

```ts
const { toggle, isListening } = useVoiceInput((text) => setInput(prev => prev + text));
<button onClick={toggle}>{isListening ? "Stop" : "Speak"}</button>
```
