---
layout: page
title: "Demo 2: TUI Interface"
permalink: /demos/tui-interface/
nav_order: 2
parent: Demos
---


## Overview

VibeCLI includes a full terminal user interface (TUI) built with Ratatui, offering a panel-based layout with chat, agent, history, and diff views -- all within your terminal. This demo covers launching the TUI, navigating panels, using keyboard shortcuts, and working with the diff view.

**Time to complete:** ~8 minutes

## Prerequisites

- VibeCLI installed and configured (see [Demo 1: First Run](../first-run/))
- A terminal emulator with 256-color or true-color support (iTerm2, Alacritty, WezTerm, Kitty, or Windows Terminal)
- Terminal size of at least 120x40 characters recommended

## Step-by-Step Walkthrough

### Step 1: Launch the TUI

```bash
vibecli tui
```

The TUI opens with a default layout: the chat panel in the main area, a sidebar for navigation, and a status bar at the bottom.

<!-- Screenshot placeholder: TUI initial layout -->

To launch with a specific provider:

```bash
vibecli tui --provider claude --model claude-sonnet-4-20250514
```

### Step 2: Navigate panels with tabs

The TUI organizes functionality into panels accessible via the tab bar at the top. Use these keys to switch:

| Key | Panel |
|-----|-------|
| `Cmd+1` / `Alt+1` | Chat |
| `Cmd+2` / `Alt+2` | Agent |
| `Cmd+3` / `Alt+3` | History |
| `Cmd+4` / `Alt+4` | Files |
| `Cmd+5` / `Alt+5` | Diff View |
| `Cmd+6` / `Alt+6` | Settings |
| `Cmd+7` / `Alt+7` | Tools |
| `Cmd+8` / `Alt+8` | Hooks |
| `Cmd+9` / `Alt+9` | Sessions |

You can also cycle through panels with `Tab` (forward) and `Shift+Tab` (backward).

<!-- Screenshot placeholder: Tab bar with panels highlighted -->

### Step 3: Chat panel

The chat panel is the primary interaction surface. Type your message at the input area at the bottom and press `Enter` to send.

```
> Explain the difference between a Vec and a HashMap in Rust
```

The response streams in real time. While the response is streaming:

- Press `Esc` to cancel generation
- Use `Up/Down` arrows to scroll through the response
- Press `PageUp/PageDown` for faster scrolling

<!-- Screenshot placeholder: Chat panel with streaming response -->

### Step 4: Agent panel

Switch to the agent panel with `Cmd+2` (or `Alt+2`). The agent panel shows the full agent loop: thinking, tool calls, results, and responses.

Start an agent task by typing in the input area:

```
> Fix the typo in src/main.rs on line 42
```

The agent panel displays:

1. **Thinking** section -- the AI's reasoning process
2. **Tool calls** -- each tool invocation (ReadFile, EditFile, Shell, etc.)
3. **Results** -- output from each tool
4. **Summary** -- final response with changes made

<!-- Screenshot placeholder: Agent panel showing tool execution -->

### Step 5: History panel

Switch to the history panel with `Cmd+3`. This panel shows all past conversations and sessions.

- Use `Up/Down` to browse sessions
- Press `Enter` to load a session
- Press `d` to delete a session
- Press `/` to search through session history

<!-- Screenshot placeholder: History panel with session list -->

### Step 6: Keyboard shortcuts

VibeCLI TUI provides a comprehensive set of keyboard shortcuts. Press `?` at any time to see the full shortcut reference.

**Global shortcuts:**

| Shortcut | Action |
|----------|--------|
| `Cmd+J` / `Ctrl+J` | Toggle AI panel |
| `` Cmd+` `` / `` Ctrl+` `` | Toggle embedded terminal |
| `Shift+P` / `Ctrl+Shift+P` | Open command palette |
| `Cmd+K` / `Ctrl+K` | Quick command input |
| `Ctrl+C` | Cancel current operation / Exit |
| `?` | Show keyboard shortcut reference |
| `Esc` | Close overlay / cancel |

**Navigation shortcuts:**

| Shortcut | Action |
|----------|--------|
| `Tab` | Next panel |
| `Shift+Tab` | Previous panel |
| `Cmd+1-9` | Jump to panel by number |
| `Ctrl+W` | Close current panel |
| `Ctrl+N` | New conversation |

**Chat shortcuts:**

| Shortcut | Action |
|----------|--------|
| `Enter` | Send message |
| `Shift+Enter` | New line in input |
| `Up` | Previous message / scroll up |
| `Down` | Next message / scroll down |
| `Ctrl+L` | Clear chat |
| `Ctrl+S` | Save conversation |

### Step 7: Split views and layout customization

The TUI supports split views for working with multiple panels simultaneously.

- `Ctrl+\` -- Split vertically (side by side)
- `Ctrl+-` -- Split horizontally (top and bottom)
- `Ctrl+W` -- Close the active split
- `Ctrl+Arrow` -- Resize splits
- `Ctrl+Tab` -- Switch focus between splits

Example workflow: Chat on the left, Diff View on the right:

```
1. Press Cmd+1 to open Chat
2. Press Ctrl+\ to split vertically
3. Press Cmd+5 in the right split to open Diff View
```

<!-- Screenshot placeholder: Split view with chat and diff -->

### Step 8: Diff view

The diff view panel (`Cmd+5`) shows file changes with two display modes.

**Unified diff mode (default):**

Added lines appear with a `+` prefix in green, removed lines with a `-` prefix in red. Context lines are shown in the default color.

**Side-by-side mode:**

Press `s` to toggle to side-by-side mode. The left pane shows the original file and the right pane shows the modified version, with change highlighting.

**Diff view controls:**

| Key | Action |
|-----|--------|
| `s` | Toggle unified / side-by-side |
| `Up/Down` | Scroll through changes |
| `n` | Jump to next hunk |
| `p` | Jump to previous hunk |
| `a` | Accept all changes |
| `r` | Reject all changes |
| `Enter` | Accept/reject current hunk |
| `g` | Go to specific line |
| `q` | Close diff view |

Line gutters show line numbers for both old and new versions. Scroll position is displayed in the status bar.

<!-- Screenshot placeholder: Diff view in side-by-side mode -->

<!-- Screenshot placeholder: Diff view in unified mode -->

### Step 9: Command palette

Press `Shift+P` to open the command palette. This fuzzy-search interface gives you access to every TUI command.

```
> diff: toggle mode
> provider: switch
> theme: dark
> session: save
```

Start typing to filter commands. Press `Enter` to execute, `Esc` to close.

<!-- Screenshot placeholder: Command palette overlay -->

### Step 10: Embedded terminal

Press `` Cmd+` `` to toggle the embedded terminal at the bottom of the TUI. This gives you a shell without leaving the interface.

```bash
$ git status
$ cargo test
$ ls -la src/
```

The terminal supports full ANSI colors and can be resized by dragging the divider or pressing `Ctrl+Arrow`.

## VibeUI Equivalent

In VibeUI (the desktop IDE), the same panels are available in the AI sidebar. Open it with `Cmd+J` and use the tab bar to switch between Chat, Agent, History, and other panels. The diff view is integrated into the Monaco editor with inline change highlighting.

## Demo Recording

```json
{
  "meta": {
    "title": "TUI Interface Tour",
    "description": "Navigate the VibeCLI terminal UI, explore panels, use keyboard shortcuts, and work with the diff view.",
    "duration_seconds": 180,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "shell",
      "command": "vibecli tui",
      "description": "Launch the TUI",
      "delay_ms": 3000
    },
    {
      "id": 2,
      "action": "keypress",
      "keys": ["alt+1"],
      "description": "Switch to Chat panel",
      "delay_ms": 1500
    },
    {
      "id": 3,
      "action": "type",
      "text": "Hello from the TUI! What panels are available?",
      "description": "Type a chat message",
      "typing_speed_ms": 40,
      "delay_ms": 500
    },
    {
      "id": 4,
      "action": "keypress",
      "keys": ["enter"],
      "description": "Send the message",
      "delay_ms": 5000
    },
    {
      "id": 5,
      "action": "keypress",
      "keys": ["alt+2"],
      "description": "Switch to Agent panel",
      "delay_ms": 2000
    },
    {
      "id": 6,
      "action": "keypress",
      "keys": ["alt+3"],
      "description": "Switch to History panel",
      "delay_ms": 2000
    },
    {
      "id": 7,
      "action": "keypress",
      "keys": ["alt+5"],
      "description": "Switch to Diff View panel",
      "delay_ms": 2000
    },
    {
      "id": 8,
      "action": "keypress",
      "keys": ["s"],
      "description": "Toggle diff view to side-by-side mode",
      "delay_ms": 2000
    },
    {
      "id": 9,
      "action": "keypress",
      "keys": ["s"],
      "description": "Toggle diff view back to unified mode",
      "delay_ms": 2000
    },
    {
      "id": 10,
      "action": "keypress",
      "keys": ["shift+p"],
      "description": "Open command palette",
      "delay_ms": 1500
    },
    {
      "id": 11,
      "action": "type",
      "text": "provider",
      "description": "Search for provider commands in palette",
      "typing_speed_ms": 60,
      "delay_ms": 2000
    },
    {
      "id": 12,
      "action": "keypress",
      "keys": ["escape"],
      "description": "Close command palette",
      "delay_ms": 1000
    },
    {
      "id": 13,
      "action": "keypress",
      "keys": ["ctrl+\\"],
      "description": "Split view vertically",
      "delay_ms": 2000
    },
    {
      "id": 14,
      "action": "keypress",
      "keys": ["alt+1"],
      "description": "Open Chat in left split",
      "delay_ms": 1000
    },
    {
      "id": 15,
      "action": "keypress",
      "keys": ["ctrl+tab"],
      "description": "Switch to right split",
      "delay_ms": 1000
    },
    {
      "id": 16,
      "action": "keypress",
      "keys": ["alt+5"],
      "description": "Open Diff View in right split",
      "delay_ms": 2000
    },
    {
      "id": 17,
      "action": "keypress",
      "keys": ["ctrl+`"],
      "description": "Toggle embedded terminal",
      "delay_ms": 2000
    },
    {
      "id": 18,
      "action": "type",
      "text": "echo 'Embedded terminal works!'",
      "description": "Run a command in embedded terminal",
      "typing_speed_ms": 40,
      "delay_ms": 500
    },
    {
      "id": 19,
      "action": "keypress",
      "keys": ["enter"],
      "description": "Execute terminal command",
      "delay_ms": 1500
    },
    {
      "id": 20,
      "action": "keypress",
      "keys": ["ctrl+c"],
      "description": "Exit TUI",
      "delay_ms": 1000
    }
  ]
}
```

## What's Next

- [Demo 3: Multi-Provider Chat](../multi-provider-chat/) -- Use 17 different AI providers
- [Demo 4: Agent Loop](../agent-loop/) -- Let the AI edit your code
- [Demo 5: Model Arena](../model-arena/) -- Compare models side by side
