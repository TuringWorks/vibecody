---
layout: page
title: "Demo 07 — DiffComplete (⌘.)"
permalink: /demos/07-inline-chat/
---

> **Note:** This demo previously covered Inline Chat (⌘K), inline AI completions, Supercomplete, and next-edit prediction. All of those surfaces were removed from VibeCody on **2026-04-26** as part of the inline-completion patent-distance work. The only AI editing surface in VibeCody is now **DiffComplete (⌘.)**, described below. (For background on the removal, see `notes/PATENT_AUDIT_INLINE.md` if you have a local working copy.)

## Overview

DiffComplete is VibeCody's explicit-chord, diff-mode AI editing surface. You select code (or none for a whole-file edit), describe the change, and the model returns a unified diff that you review hunk-by-hunk in a modal. You can edit the result in Monaco before applying, and you can refine the diff with a follow-up instruction without re-entering the original prompt.

This is deliberately **not** a ghost-text / inline-completion / FIM-style flow.

## Prerequisites

- VibeCody installed (`vibecli --version` returns 0.5.5+)
- At least one AI provider configured in `~/.vibecli/config.toml`:

```toml
[claude]
enabled = true
api_key = "sk-ant-..."
model = "claude-sonnet-4-6"
```

- VibeUI built and running:

```bash
cd vibeui && npm install && npm run tauri dev
```

- A project open in VibeUI (File > Open Folder)

## Step-by-Step Walkthrough

### 1. Open a source file

Click a file in the Explorer sidebar, e.g., `src/main.rs`.

### 2. Trigger DiffComplete

Optionally select a region of code, then press **`Cmd+.`** (macOS) or **`Ctrl+.`** (Linux/Windows). With no selection the request applies to the whole file.

The DiffComplete modal opens.

### 3. Write your instruction

Type a natural-language instruction:

```
Convert this function to async and add proper error handling
```

### 4. (Optional) Attach extra files as context

Click **`+ Add file…`** to open the OS file picker and select related files. The selected files are forwarded to the model as explicit context — there is no automatic embedding search or hidden retrieval.

The picker is the only path for cross-file context. Attached files appear as removable chips.

### 5. Submit (`Cmd+Enter`)

The backend calls `diffcomplete_generate` with your instruction, the selection (or whole file), the surrounding 200-line context window, and any attached files. The model returns a unified diff which is parsed by `applyUnifiedDiff` and handed to `DiffReviewPanel`.

### 6. Review hunk-by-hunk

`DiffReviewPanel` shows each hunk with **Accept**, **Reject**, **Accept All**, **Reject All** buttons. The currently-assembled result preview updates as you change accept/reject choices.

### 7. (Optional) Edit before applying

Click **Edit ✎** to switch into a Monaco editor seeded with the currently-accepted hunks. Make any final tweaks, then press **Apply (edited)** to write the modified text. Use **← Hunks** to discard your edits and return to the hunk view.

### 8. (Optional) Refine and regenerate

Below the diff view there's a **Refine** input. Type a follow-up like _"tighten the error path"_ or _"use the helper from utils"_, then press **Regenerate (`Cmd+Enter`)**. The previous diff and your refinement are layered on top of the original instruction — the chain stays visible in the prompt.

### 9. Apply

When you're satisfied, click **Apply** (or **Apply (edited)** if you went through the edit step). The buffer is updated via `editor.executeEdits` and a `diffcomplete` event is recorded in the Cascade Flow timeline.

## What's not in VibeCody anymore

The following Tauri commands and frontend modules were removed on 2026-04-26 and are intentionally not coming back:

- `request_inline_completion` / `request_ai_completion` — FIM-style code completion
- `predict_next_edit` — next-edit prediction
- `inline_edit` / `generate_code` — Cmd+K backend
- `semantic_search_codebase` / `build_embedding_index` — orphan after `SupercompleteEngine` was removed
- `vibe-ai/src/completion.rs` (`CompletionEngine`)
- `vibeui/src/components/InlineChat.tsx`
- `vibeui/src/utils/SupercompleteEngine.ts`
- `vibeui-ai-inline-completion-enabled` localStorage toggle
- `Monaco.languages.registerInlineCompletionsProvider` registration in `App.tsx`

If you need an alternative, use DiffComplete (⌘.) — it is the supported AI editing surface.
