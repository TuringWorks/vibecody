---
layout: default
title: Utilities Reference
nav_order: 16
---

# Utilities Reference

Five shared utility modules live in `vibeui/src/utils/`. They provide common functionality consumed by panels, hooks, and the main App shell.

---

## DocsResolver

**Purpose**: Fetches library documentation for `@docs:<name>` context references, auto-detecting the package registry (docs.rs, npmjs.com, PyPI) and caching results in `sessionStorage` for 24 hours.

**Key exports**:

| Export | Description |
|---|---|
| `DocRegistry` | Type alias: `"rs" \| "npm" \| "py"` |
| `DocResult` | Interface with `name`, `registry`, `summary`, optional `version` |
| `detectRegistry(name)` | Heuristic registry detection from a package name string |
| `resolveDoc(rawName)` | Async - fetches docs via `fetch_doc_content` Tauri command, returns `DocResult` |
| `formatDocForContext(doc)` | Formats a `DocResult` as a labeled text block for AI prompt injection |

**Used by**: No direct panel imports found; called indirectly through chat context resolution.

---

## fileUtils

**Purpose**: File-type detection helpers for Monaco language mapping, human-readable file sizes, and file tree icons.

**Key exports**:

| Export | Description |
|---|---|
| `detectLanguage(filename)` | Maps file extension to Monaco language ID (100+ extensions covered) |
| `formatFileSize(bytes)` | Returns human-readable size string (B / KB / MB / GB) |
| `getFileIcon(filename, isDirectory)` | Returns a React `<Icon>` element appropriate for the file type |

**Used by**: `App.tsx`

---

## FlowContext

**Purpose**: In-memory chronological timeline of all AI interactions (chat, inline edits, agent steps, terminal commands, file edits). Provides context summaries for injection into AI prompts with a configurable token budget.

**Key exports**:

| Export | Description |
|---|---|
| `FlowEventKind` | Union type: `"chat"`, `"inline_edit"`, `"inline_generate"`, `"agent_step"`, `"agent_complete"`, `"agent_partial"`, `"terminal_cmd"`, `"file_edit"` |
| `FlowEvent` | Interface with `id`, `kind`, `summary`, `detail`, `timestamp`, optional `filePath`, `approxTokens` |
| `flowContext` | Singleton `FlowContextManager` instance (import this, never construct directly) |

**FlowContextManager methods**:

- `add(params)` - Record a new event (max 200 retained, detail truncated at 2000 chars)
- `getAll()` / `getByKind(kind)` / `getRecent(n)` - Query events
- `getContextSummary(tokenBudget?, kinds?)` - Build a compact text summary within a token budget
- `subscribe(fn)` - Listen for changes; returns unsubscribe function
- `clear()` - Remove all events

**Used by**: `AgentPanel.tsx`, `CascadePanel.tsx`, `AIChat.tsx`, `App.tsx`

---

## LinterIntegration

**Purpose**: Runs language-appropriate linters (ESLint, cargo check, flake8, go vet) after agent file writes via the `run_linter` Tauri command, and formats results for agent context injection.

**Key exports**:

| Export | Description |
|---|---|
| `LintError` | Interface: `line`, `col`, `severity`, `message`, optional `rule` |
| `LintResult` | Interface: `filePath`, `errors`, `warnings`, `rawOutput`, `linterAvailable` |
| `linterForFile(filePath)` | Returns linter name from file extension, or `null` if unsupported |
| `runLinter(filePath)` | Async - invokes `run_linter` Tauri command, returns `LintResult` (never throws) |
| `formatLintForAgent(result)` | Formats errors/warnings as a text block for agent context, returns `null` if clean |

**Used by**: `AgentPanel.tsx`

---

## SupercompleteEngine

**Purpose**: Cross-file multi-line edit prediction. Extends inline completion by enriching prompts with semantically related code from the workspace embedding index. Activates only during active editing sessions (3+ edits in 30 seconds).

**Key exports**:

| Export | Description |
|---|---|
| `SupercompleteResult` | Interface: `insertText`, `contextFiles`, `confidence` (0-1) |
| `supercompleteEngine` | Singleton engine instance |

**SupercompleteEngine methods**:

- `predict(params)` - Async - runs semantic search, builds enriched prompt, requests completion. Returns `null` if activation gate fails or no relevant cross-file context found.
- `invalidate()` - Clear cached result (call on significant document changes)

**Tauri commands used**: `semantic_search_codebase`, `request_inline_completion`

**Used by**: `App.tsx`
