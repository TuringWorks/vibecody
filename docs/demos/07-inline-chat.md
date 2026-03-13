---
layout: page
title: "Demo 07 — Inline Chat & Completions"
permalink: /demos/07-inline-chat/
---

# Demo 07 — Inline Chat & Completions

## Overview

VibeCody brings AI directly into the editor with **inline chat**, **context-aware completions**, **Supercomplete**, and **edit predictions**. Instead of switching between a chat panel and your code, you interact with AI at the cursor. Supercomplete generates multi-line predictions based on your editing patterns, and the RL-based edit predictor anticipates your next change before you make it.

This demo covers the VibeUI Monaco editor experience and the underlying CLI commands that power it.

---

## Prerequisites

- VibeCody installed (`vibecli --version` returns 0.1+)
- At least one AI provider configured in `~/.vibecli/config.toml`:

```toml
[provider]
default = "claude"

[provider.claude]
api_key = "sk-ant-..."
```

- VibeUI built and running:

```bash
cd vibeui && npm install && npm run tauri dev
```

- A project open in VibeUI (File > Open Folder)

---

## Step-by-Step Walkthrough

### 1. Open a Source File

Open any source file in the Monaco editor. For this demo we will use a Rust file.

**VibeUI:**
Click a file in the Explorer sidebar, e.g., `src/main.rs`.

**CLI (HTTP daemon for API access):**

```bash
vibecli serve --port 7878 --provider claude
```

### 2. Trigger Inline Chat

Select a block of code in the editor (or place the cursor on a line) and invoke inline chat.

**VibeUI:**
- **Keyboard:** Press `Cmd+I` (macOS) or `Ctrl+I` (Linux/Windows)
- **Mouse:** Select code, right-click, choose **VibeCody > Inline Chat**

A small input field appears at the cursor position.

### 3. Give an Instruction

Type a natural-language instruction into the inline chat prompt:

```
Convert this function to async and add error handling
```

Press `Enter`. The AI reads the surrounding code context (function scope, imports, LSP symbols) and generates a replacement.

### 4. Accept or Reject the Suggestion

The editor shows a diff overlay with the proposed change:

- **Accept:** Press `Cmd+Enter` or click the green checkmark
- **Reject:** Press `Escape` or click the red X
- **Edit further:** Type a follow-up instruction without leaving inline chat

### 5. Context-Aware Completions

As you type normally, VibeCody provides AI-enhanced completions that combine:

- **LSP completions** (types, methods, imports)
- **AI completions** (natural-language-aware, multi-token)

Completions appear in the standard autocomplete dropdown. Items with the VibeCody icon are AI-generated.

**VibeUI Settings:**

```
Settings > VibeCody > Completions > Enable AI Completions: true
Settings > VibeCody > Completions > Debounce (ms): 300
```

### 6. Supercomplete (Multi-Line Predictions)

Supercomplete predicts the next several lines you are about to write based on the file context and your recent edits.

**VibeUI:**
- Ghost text appears in gray after your cursor
- Press `Tab` to accept the entire prediction
- Press `Cmd+Right` to accept word-by-word
- Press `Escape` to dismiss

**CLI equivalent (for API integrations):**

```bash
curl -X POST http://localhost:7878/api/v1/supercomplete \
  -H "Content-Type: application/json" \
  -d '{
    "file": "src/main.rs",
    "cursor_line": 42,
    "cursor_col": 0,
    "context_before": "fn process_request(req: Request) -> Result<Response> {\n",
    "context_after": "\n}\n",
    "max_lines": 10
  }'
```

Response:

```json
{
  "prediction": "    let body = req.body().await?;\n    let parsed: RequestBody = serde_json::from_slice(&body)?;\n    let result = handle_parsed(parsed).await?;\n    Ok(Response::json(&result))\n",
  "confidence": 0.87,
  "model": "claude-sonnet-4-20250514"
}
```

### 7. Edit Predictions (RL-Based Next-Edit Predictor)

VibeCody's edit prediction engine uses reinforcement learning (Q-learning) to anticipate your next edit location and action based on your editing history.

**VibeUI:**
- A subtle highlight appears on the line the predictor believes you will edit next
- A small tooltip shows the predicted action (e.g., "Add error handling", "Rename variable")
- Click the tooltip or press `Cmd+Shift+Enter` to apply the predicted edit
- The model learns from your accept/reject decisions

**CLI — train and query the predictor:**

```bash
# View current prediction state
vibecli edit-predict status

# Feed an editing event
vibecli edit-predict event --file src/main.rs --line 42 --action "add_error_handling"

# Get the next predicted edit
vibecli edit-predict next --file src/main.rs
```

Example output:

```
Next predicted edit:
  File: src/main.rs
  Line: 58
  Action: add_error_handling
  Confidence: 0.74
  Q-value: 2.31
```

### 8. Ghost Text Rendering

All AI suggestions (completions, Supercomplete, edit predictions) render as **ghost text** — translucent characters that show what the AI proposes without modifying your buffer.

Ghost text styling is controlled by CSS variables:

```css
--vibe-ghost-text-color: #6b7280;
--vibe-ghost-text-opacity: 0.6;
--vibe-ghost-text-font-style: italic;
```

Override these in VibeUI's Settings > Appearance > Theme Customization.

---

## CLI-Only Workflow

If you prefer working entirely in the terminal, the TUI provides inline-chat-like functionality:

```bash
# Start TUI
vibecli tui

# In the TUI editor view, press `i` to enter inline chat mode
# Type your instruction and press Enter
# Use `y` to accept, `n` to reject
```

For batch inline edits across multiple files:

```bash
vibecli inline-edit \
  --files "src/**/*.rs" \
  --instruction "Add #[derive(Debug)] to all structs missing it" \
  --dry-run
```

Remove `--dry-run` to apply changes.

---

## Demo Recording

```json
{
  "id": "demo-inline-chat",
  "title": "Inline Chat & Completions",
  "description": "Demonstrates inline chat, Supercomplete, edit predictions, and ghost text in the VibeUI editor",
  "estimated_duration_s": 120,
  "steps": [
    {
      "action": "Navigate",
      "target": "vibeui://open?file=src/main.rs"
    },
    {
      "action": "Narrate",
      "value": "We have a Rust source file open in VibeUI. Let's use inline chat to refactor a function."
    },
    {
      "action": "Click",
      "target": ".editor-line:nth-child(15)",
      "description": "Click on line 15 to position the cursor"
    },
    {
      "action": "Screenshot",
      "label": "editor-before-inline-chat"
    },
    {
      "action": "Type",
      "target": "keyboard",
      "value": "Cmd+I",
      "description": "Trigger inline chat with keyboard shortcut"
    },
    {
      "action": "WaitForSelector",
      "target": ".inline-chat-input",
      "timeout_ms": 2000
    },
    {
      "action": "Type",
      "target": ".inline-chat-input",
      "value": "Convert this function to async and add proper error handling with anyhow"
    },
    {
      "action": "Screenshot",
      "label": "inline-chat-prompt-filled"
    },
    {
      "action": "Type",
      "target": "keyboard",
      "value": "Enter",
      "description": "Submit the inline chat instruction"
    },
    {
      "action": "Narrate",
      "value": "The AI analyzes the function, its callsites, and the project's error handling patterns. A diff overlay appears showing proposed changes."
    },
    {
      "action": "Wait",
      "duration_ms": 3000
    },
    {
      "action": "Screenshot",
      "label": "inline-chat-diff-overlay"
    },
    {
      "action": "Assert",
      "target": ".diff-overlay",
      "value": "contains:async fn"
    },
    {
      "action": "Type",
      "target": "keyboard",
      "value": "Cmd+Enter",
      "description": "Accept the suggested change"
    },
    {
      "action": "Narrate",
      "value": "Change accepted. Now let's see Supercomplete in action. We start typing a new function."
    },
    {
      "action": "Click",
      "target": ".editor-line:last-child",
      "description": "Move cursor to end of file"
    },
    {
      "action": "Type",
      "target": ".monaco-editor textarea",
      "value": "fn validate_"
    },
    {
      "action": "Wait",
      "duration_ms": 1500
    },
    {
      "action": "Screenshot",
      "label": "supercomplete-ghost-text"
    },
    {
      "action": "Assert",
      "target": ".ghost-text",
      "value": "exists"
    },
    {
      "action": "Narrate",
      "value": "Supercomplete has predicted the full function body as ghost text. Press Tab to accept it all at once."
    },
    {
      "action": "Type",
      "target": "keyboard",
      "value": "Tab",
      "description": "Accept the Supercomplete prediction"
    },
    {
      "action": "Screenshot",
      "label": "supercomplete-accepted"
    },
    {
      "action": "Narrate",
      "value": "Finally, notice the edit prediction highlight. The RL model predicts where we will edit next."
    },
    {
      "action": "Wait",
      "duration_ms": 1000
    },
    {
      "action": "Assert",
      "target": ".edit-prediction-highlight",
      "value": "exists"
    },
    {
      "action": "Screenshot",
      "label": "edit-prediction-highlight"
    },
    {
      "action": "Narrate",
      "value": "The predictor highlighted line 22, suggesting we add error handling there. This prediction improves over time as it learns from our editing patterns."
    }
  ],
  "tags": ["inline-chat", "completions", "supercomplete", "edit-prediction", "ghost-text", "editor"]
}
```

---

## What's Next

- [Demo 08 — Code Search & Embeddings](08-code-search.md) — Find code semantically across your project
- [Demo 09 — Autofix & Diagnostics](09-autofix.md) — Let AI fix errors detected by inline chat
- [Demo 10 — Code Transforms](10-code-transforms.md) — AST-level refactoring beyond inline edits
