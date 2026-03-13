---
layout: page
title: "Demo 19: Context Bundles"
permalink: /demos/context-bundles/
nav_order: 19
parent: Demos
---

# Demo 19: Context Bundles

## Overview

Context Bundles let you package a curated set of files, instructions, model preferences, and exclusion rules into a portable `.vibebundle.toml` file. When a bundle is active, its contents are automatically injected into the agent system prompt so every AI interaction is grounded in the right context. Teams can share bundles via export/import to ensure consistent project understanding across collaborators.

**Time to complete:** ~10 minutes

## Prerequisites

- VibeCody installed and configured with at least one AI provider
- A project directory with source files you want to pin
- (Optional) VibeUI installed for the graphical Context Bundle panel

## Step-by-Step Walkthrough

### Step 1: Create your first context bundle

Use the `/bundle create` REPL command to define a new bundle. Provide a name and the files or directories you want pinned.

```bash
vibecli repl
```

Inside the REPL:

```
/bundle create "backend-api"
```

VibeCLI will create a `.vibebundle.toml` file in your project root and open an interactive prompt to select pinned files.

### Step 2: Add pinned files and instructions

You can add files interactively or edit the TOML directly.

**Interactive (REPL):**

```
/bundle pin src/api/routes.rs src/api/handlers.rs src/models/
```

**Direct TOML editing:**

```toml
# .vibebundle.toml
[bundle]
name = "backend-api"
version = "1.0.0"
description = "Backend API context for route and handler development"
priority = 10

[bundle.pinned_files]
include = [
    "src/api/routes.rs",
    "src/api/handlers.rs",
    "src/models/*.rs",
    "Cargo.toml",
]

[bundle.excludes]
patterns = [
    "target/",
    "*.log",
    "node_modules/",
    ".git/",
]

[bundle.instructions]
system = """
You are working on a Rust Actix-web backend API.
Follow RESTful conventions. All handlers return JSON.
Use the repository pattern for database access.
Error responses use the ApiError type from src/errors.rs.
"""

[bundle.model_preferences]
provider = "claude"
model = "claude-sonnet-4-20250514"
temperature = 0.3
max_tokens = 4096
```

### Step 3: Activate the bundle

```
/bundle activate backend-api
```

Once activated, every AI interaction in this project session will include the pinned files and instructions in the system prompt. The agent sees the actual content of pinned files, not just their paths.

### Step 4: Verify active bundles

```
/bundle list
```

Expected output:

```
Active Bundles:
  1. [*] backend-api    (priority: 10)  4 pinned files

Available Bundles:
  2. [ ] frontend-ui    (priority: 5)   6 pinned files
  3. [ ] database       (priority: 8)   3 pinned files
```

### Step 5: Work with multiple active bundles

You can activate more than one bundle at a time. Priority ordering determines which instructions take precedence when bundles overlap.

```
/bundle activate database
/bundle list
```

```
Active Bundles:
  1. [*] backend-api    (priority: 10)  4 pinned files
  2. [*] database       (priority: 8)   3 pinned files
```

Higher-priority bundles appear first in the system prompt. If two bundles pin the same file, the higher-priority bundle's version is used.

### Step 6: Deactivate a bundle

```
/bundle deactivate database
```

### Step 7: Share bundles with your team

**Export a bundle:**

```
/bundle share backend-api --output backend-api.vibebundle.toml
```

This writes a self-contained TOML file that teammates can import.

**Import a bundle:**

```
/bundle import ./shared/backend-api.vibebundle.toml
```

### Step 8: Using Context Bundles in VibeUI

Open the **Context Bundles** panel from the sidebar. It has three tabs:

| Tab | Description |
|-----|-------------|
| **My Bundles** | View, activate, deactivate, and delete bundles |
| **Create** | Visual editor for building new bundles with file picker |
| **Import/Export** | Drag-and-drop import, one-click export to file or clipboard |

1. Click the **Create** tab.
2. Enter a bundle name and description.
3. Use the file tree to check the files you want to pin.
4. Add system instructions in the text area.
5. Set model preferences with the provider/model dropdowns.
6. Click **Save Bundle**.

To activate, switch to **My Bundles** and toggle the switch next to the bundle name.

## CLI Command Reference

| Command | Description |
|---------|-------------|
| `/bundle create <name>` | Create a new context bundle |
| `/bundle activate <name>` | Activate a bundle for the current session |
| `/bundle deactivate <name>` | Deactivate a bundle |
| `/bundle list` | List all bundles and their status |
| `/bundle pin <files...>` | Add files to the active bundle |
| `/bundle unpin <files...>` | Remove files from the active bundle |
| `/bundle share <name> --output <path>` | Export bundle to a TOML file |
| `/bundle import <path>` | Import a bundle from a TOML file |
| `/bundle delete <name>` | Delete a bundle |

## `.vibebundle.toml` Format Reference

```toml
[bundle]
name = "bundle-name"
version = "1.0.0"
description = "What this bundle provides"
priority = 10                    # Higher = injected first

[bundle.pinned_files]
include = [
    "src/specific-file.rs",      # Exact file
    "src/models/*.rs",           # Glob pattern
    "docs/api/**/*.md",          # Recursive glob
]

[bundle.excludes]
patterns = [
    "target/",
    "*.generated.*",
]

[bundle.instructions]
system = """
Instructions injected into the agent system prompt.
"""

[bundle.model_preferences]
provider = "claude"              # Preferred provider
model = "claude-sonnet-4-20250514"       # Preferred model
temperature = 0.3
max_tokens = 4096
```

## Demo Recording

The following JSON represents a recorded demo session that can be replayed in VibeUI's Recording panel.

```json
{
  "demoRecording": {
    "version": "1.0",
    "title": "Context Bundles Demo",
    "description": "Create, activate, and share context bundles for consistent AI interactions",
    "duration_seconds": 180,
    "steps": [
      {
        "timestamp": 0,
        "action": "repl_command",
        "command": "/bundle create \"backend-api\"",
        "output": "Created bundle 'backend-api' at .vibebundle.toml",
        "narration": "Create a new context bundle named backend-api"
      },
      {
        "timestamp": 15,
        "action": "repl_command",
        "command": "/bundle pin src/api/routes.rs src/api/handlers.rs src/models/",
        "output": "Pinned 4 files to bundle 'backend-api':\n  src/api/routes.rs\n  src/api/handlers.rs\n  src/models/user.rs\n  src/models/order.rs",
        "narration": "Pin relevant source files to the bundle"
      },
      {
        "timestamp": 35,
        "action": "file_edit",
        "file": ".vibebundle.toml",
        "section": "bundle.instructions",
        "content": "system = \"You are working on a Rust Actix-web backend API...\"",
        "narration": "Add system instructions to the bundle"
      },
      {
        "timestamp": 55,
        "action": "repl_command",
        "command": "/bundle activate backend-api",
        "output": "Activated bundle 'backend-api' (priority 10, 4 pinned files)\nBundle context will be injected into all AI interactions.",
        "narration": "Activate the bundle so its context is injected into every AI call"
      },
      {
        "timestamp": 70,
        "action": "repl_command",
        "command": "/bundle list",
        "output": "Active Bundles:\n  1. [*] backend-api  (priority: 10)  4 pinned files\n\nAvailable Bundles:\n  (none)",
        "narration": "Verify the bundle is active"
      },
      {
        "timestamp": 85,
        "action": "chat_message",
        "message": "Add a new GET /users/:id endpoint",
        "output": "I can see from the pinned context that routes are defined in src/api/routes.rs using Actix-web...\n[generates code with full project context]",
        "narration": "The AI now has full context from pinned files and instructions"
      },
      {
        "timestamp": 120,
        "action": "repl_command",
        "command": "/bundle share backend-api --output shared/backend-api.vibebundle.toml",
        "output": "Exported bundle 'backend-api' to shared/backend-api.vibebundle.toml",
        "narration": "Export the bundle for team sharing"
      },
      {
        "timestamp": 140,
        "action": "repl_command",
        "command": "/bundle import shared/backend-api.vibebundle.toml",
        "output": "Imported bundle 'backend-api' (4 pinned files, 1 instruction block)",
        "narration": "A teammate imports the shared bundle"
      },
      {
        "timestamp": 160,
        "action": "ui_interaction",
        "panel": "ContextBundles",
        "tab": "My Bundles",
        "action_detail": "toggle_activate",
        "bundle": "backend-api",
        "narration": "Activate the bundle from the VibeUI panel"
      }
    ]
  }
}
```

## What's Next

- [Demo 20: Agent Teams](../agent-teams/) -- Coordinate multiple AI agents with specialized roles
- [Demo 21: CRDT Collaboration](../crdt-collab/) -- Real-time collaborative editing with conflict resolution
- Combine context bundles with agent teams so each agent role inherits the right project context
