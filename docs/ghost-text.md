---
layout: page
title: Ghost text (⌥\)
permalink: /ghost-text/
---

> Inline AI completion that only appears when you ask for it. Press **⌥\\** at the cursor, read the grey suggestion, press **Tab** to accept or **Esc** to dismiss.

Ghost text is VibeCody's short-form AI completion. It renders inline at the cursor and accepts with Tab — the ergonomics people expect from a modern editor — but it is **explicit-trigger only**: nothing is requested, and nothing appears, while you type.

For a larger change you want to review hunk-by-hunk, use [Diffcomplete (⌘.)](/vibecody/diffcomplete/) instead.

---

## Quick start

1. Open a file in **VibeCoder**, or any file in **VS Code** with the VibeCLI extension installed.
2. Put the cursor where the code should go.
3. Press **⌥\\** (Alt-Backslash on every platform). Or run **AI: Inline Completion at Cursor** from the command palette.
4. A grey suggestion appears inline.
5. **Tab** accepts it. **Esc** dismisses it. Typing anything else dismisses it too.

If nothing appears, the model decided nothing belonged at that cursor — that is a normal answer, not an error.

---

## Why it looks like other tools but isn't

VibeCody removed its previous ghost-text implementation on purpose: that one ran on a debounce timer after every keystroke, kept a rolling buffer of your recent edits, and used them to predict the next one. This one keeps the presentation and drops all of that.

| | Removed (keystroke-driven) | Current (explicit-trigger) |
|---|---|---|
| Trigger | debounce timer after each keystroke | **⌥\\ only** |
| Sees your edit history | yes, a rolling buffer | **no — nothing is retained between requests** |
| Auto-retrieval of other files | yes, embedding search | **no** |
| Requests while you type | continuously | **zero** |
| Accept | Tab | Tab |

The guarantee is one line of code in each editor, and it is the same line conceptually: the inline-completion provider is consulted by the editor constantly, and it returns nothing unless the trigger kind is the explicit one.

> **For contributors:** the enum is inverted between the two hosts. Monaco has `InlineCompletionTriggerKind = { Automatic: 0, Explicit: 1 }`; VS Code has `{ Invoke: 0, Automatic: 1 }`. The same names carry opposite numbers, so a literal comparison copied from one host to the other gates on exactly the wrong half and fires on every keystroke. Always compare against the named member. Each host has its own gate for this reason — `vibecoder/src/lib/ghostText.ts` and `vscode-extension/src/ghost-text.ts`.

---

## What gets sent

Per press, one request carrying:

- **Prefix** — up to 160 lines before the cursor.
- **Suffix** — up to 60 lines after the cursor.
- **File path and language**, for the prompt header.
- **Project memory** — your author-written `VIBECLI.md` / `AGENTS.md` / `CLAUDE.md`, the same audit-restricted source diffcomplete uses. Never auto-extracted state.

Nothing else. No edit history, no telemetry about what you accepted or rejected, no embedding search.

The response is capped at 12 lines (`vibe_ai::ghost::MAX_COMPLETION_LINES`). When the cap clips a suggestion the editor says so — accept what you have and press ⌥\\ again to continue.

---

## Provider and model

Ghost text is provider-agnostic and never defaults to a single vendor.

- **VibeCoder** uses the provider selected in the toolbar, with that provider's registry default model. With no provider selected, ⌥\\ tells you to pick one rather than silently calling anything.
- **VS Code** uses the `vibecli.provider` and `vibecli.model` settings. The daemon only honours the override when **both** are set — leave `vibecli.model` empty and the daemon uses whichever provider and model it was started with.

---

## Architecture

```
[ ⌥\ press ]
   │
   ▼
editor.action.inlineSuggest.trigger        (host built-in)
   │
   ▼
inline-completion provider  ── trigger kind is Automatic? ──► return nothing
   │
   │ explicit
   ▼
VibeCoder: ghost_complete (Tauri command)
VS Code:   POST /v1/ghost/complete (daemon, bearer auth)
   │
   ▼
vibe_ai::ghost::generate()
   │
   ├─► system prompt demanding bare insertion text
   ├─► prefix / cursor / suffix as labeled regions
   ├─► project memory as a separate system message
   │
   ▼
sanitize_completion() — unwraps stray code fences, caps at 12 lines,
                        preserves leading indentation
```

Leading whitespace is deliberately preserved: at a cursor sitting at column 0 of an indented block, the indentation *is* the first thing that belongs there.

---

## What ghost text is NOT

- **Not** keystroke-driven. There is no debounce timer to tune and no on-type path to disable, because none is installed.
- **Not** FIM. The model receives prefix and suffix as discrete labeled regions, not a single fill-in-middle template.
- **Not** next-edit prediction. Each request is independent; nothing about your previous edits or previous suggestions is carried forward.
- **Not** auto-retrieval. No embedding search, no call-graph walk, no symbol-server probe.

---

## Related

- **Source:** [`vibecoder/crates/vibe-ai/src/ghost.rs`](https://github.com/TuringWorks/vibecody/blob/main/vibecoder/crates/vibe-ai/src/ghost.rs) · [`vibecoder/src/lib/ghostText.ts`](https://github.com/TuringWorks/vibecody/blob/main/vibecoder/src/lib/ghostText.ts) · [`vscode-extension/src/ghost-text.ts`](https://github.com/TuringWorks/vibecody/blob/main/vscode-extension/src/ghost-text.ts)
- **Route:** `POST /v1/ghost/complete` (requires the daemon bearer token)
- **Companion surface:** [Diffcomplete (⌘.)](/vibecody/diffcomplete/)
