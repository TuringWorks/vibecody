---
layout: page
title: "Demo 18: MCP Plugin Directory"
permalink: /demos/18-mcp-directory/
nav_order: 18
parent: Demos
---

# Demo 18: MCP Plugin Directory

## Overview

The MCP Plugin Directory is VibeCody's curated marketplace for MCP servers. Instead of manually configuring servers from scratch, you can browse verified plugins by category, install them with a single command, and benefit from community ratings and a security verification pipeline. This demo walks through browsing, installing, verifying, and managing MCP plugins.

**Time to complete:** ~15 minutes

## Prerequisites

- VibeCLI installed and configured ([Demo 1](../first-run/))
- MCP basics understood ([Demo 16](../16-mcp-servers/))
- Node.js 18+ (most plugins use npx)
- (Optional) VibeUI for the desktop panel experience

## Step-by-Step Walkthrough

### Step 1: Browse the plugin directory

Use the `/mcp search` command with a category filter to browse available plugins.

```bash
vibecli repl
> /mcp search --browse
```

```
MCP Plugin Directory
=====================
10 categories | 127 verified plugins

Category                Plugins  Description
-----------             -------  --------------------------------
Data & Databases        18       PostgreSQL, MySQL, MongoDB, Redis, Snowflake, DuckDB
Cloud & Infrastructure  15       AWS, GCP, Azure, Terraform, K8s
Developer Tools         22       Git, GitHub, GitLab, Docker, CI/CD
Communication           12       Slack, Discord, Email, Teams, Twilio
Search & Knowledge      11       Brave Search, Google, Wikipedia, Arxiv
Files & Storage         9        Filesystem, S3, GCS, Dropbox
AI & ML                 8        Hugging Face, Weights & Biases, LangSmith
Monitoring              10       Datadog, PagerDuty, Sentry, Prometheus
Productivity            14       Notion, Linear, Jira, Asana, Calendar
Utilities               8        Weather, Maps, Currency, Translation

Browse a category: /mcp search --category "Developer Tools"
Search by name:    /mcp search github
```

### Step 2: Search for a specific plugin

```bash
> /mcp search postgres
```

```
Search Results: "postgres" (3 matches)
========================================

1. mcp-postgres                         v1.2.0  ★★★★★ (4.8, 342 ratings)
   Official PostgreSQL MCP server
   Author: @modelcontextprotocol  |  Verified ✓  |  Downloads: 12,450
   Tools: query, list_tables, describe_table, create_table, alter_table
   Transport: stdio  |  Requires: Node.js 18+

2. mcp-supabase                         v0.9.1  ★★★★☆ (4.2, 89 ratings)
   Supabase (PostgreSQL + Auth + Realtime)
   Author: @supabase  |  Verified ✓  |  Downloads: 3,210
   Tools: query, auth_users, storage_upload, realtime_subscribe
   Transport: stdio  |  Requires: Node.js 18+

3. mcp-pg-admin                         v0.5.0  ★★★★☆ (4.0, 28 ratings)
   PostgreSQL admin tools (vacuum, reindex, roles)
   Author: @community  |  Community ○  |  Downloads: 890
   Tools: vacuum, reindex, manage_roles, pg_stat, explain_analyze
   Transport: stdio  |  Requires: Node.js 18+
```

### Step 3: View plugin details

```bash
> /mcp search --detail mcp-postgres
```

```
Plugin: mcp-postgres v1.2.0
=============================
Author:       @modelcontextprotocol (Official)
License:      MIT
Verified:     ✓ (checksum verified, permissions audited, sandbox tested)
Published:    2026-02-15
Downloads:    12,450
Rating:       ★★★★★ 4.8/5.0 (342 ratings)

Description:
  Connect to PostgreSQL databases. Execute queries, inspect schemas,
  manage tables, and run migrations through MCP tools.

Tools (5):
  query              Execute a SQL query (SELECT, INSERT, UPDATE, DELETE)
  list_tables        List all tables in the database
  describe_table     Get column types, constraints, and indexes
  create_table       Create a new table with DDL
  alter_table        Modify table schema (add/drop/rename columns)

Resources (2):
  schema://tables    Current database tables and their schemas
  schema://indexes   All indexes across the database

Required Configuration:
  connection_string  PostgreSQL connection URL (required)

Permissions Requested:
  network            Connect to database server
  (no filesystem, no shell)

Install: /mcp install mcp-postgres
```

### Step 4: Install a plugin

```bash
> /mcp install mcp-postgres
```

```
Installing mcp-postgres v1.2.0...

  [1/4] Downloading package         OK  (1.2 MB)
  [2/4] Verifying checksum          OK  (SHA-256 match)
  [3/4] Permission audit            OK  (network only, no filesystem/shell)
  [4/4] Sandbox test                OK  (started and responded to list_tools)

Installed successfully!

Configure connection string:
  > Enter PostgreSQL URL: postgresql://user:pass@localhost:5432/myapp

Added to config.toml:
  [mcp.servers.mcp-postgres]
  command = "npx"
  args = ["-y", "@modelcontextprotocol/server-postgres", "postgresql://user:pass@localhost:5432/myapp"]
  plugin_version = "1.2.0"

Server connected. Tools available: 5
Use /mcp tools mcp-postgres to see available tools.
```

### Step 5: Understand the verification pipeline

Every plugin in the directory goes through a four-stage verification process before it receives the "Verified" badge.

```
Verification Pipeline
======================

Stage 1: Checksum Verification
  - Package downloaded from npm/PyPI registry
  - SHA-256 hash compared against the directory manifest
  - Ensures the package has not been tampered with

Stage 2: Permission Audit
  - Static analysis of the server code for permission usage
  - Categorized into: filesystem, network, shell, environment
  - Must match declared permissions in mcp-plugin.toml
  - Undeclared permissions trigger a warning flag

Stage 3: Sandbox Test
  - Server started in an isolated container
  - list_tools and list_resources calls verified
  - Each tool invoked with sample inputs
  - No network egress allowed during sandbox test
  - Must respond within 10 seconds per call

Stage 4: Community Review
  - Minimum 5 ratings required for "Community" badge
  - Minimum 20 ratings + author verification for "Verified" badge
  - Reports of malicious behavior trigger immediate delisting
```

### Step 6: Manage installed plugins

```bash
# List installed plugins
> /mcp installed
```

```
Installed MCP Plugins
======================
Plugin             Version  Status       Auto-Update
-----------        -------  ----------   -----------
mcp-postgres       1.2.0    connected    enabled
mcp-github         1.1.3    connected    enabled
mcp-filesystem     1.0.5    connected    disabled
mcp-slack          0.8.2    disconnected enabled

4 installed | 3 connected
```

```bash
# Update a specific plugin
> /mcp update mcp-slack
```

```
Updating mcp-slack...
  Current: v0.8.2 -> Available: v0.9.0

  Changelog (v0.9.0):
    - Added: send_dm tool for direct messages
    - Fixed: Channel list pagination
    - Improved: Rate limit handling

  [1/4] Downloading             OK
  [2/4] Verifying checksum      OK
  [3/4] Permission audit        OK (no new permissions)
  [4/4] Sandbox test            OK

Updated to v0.9.0. Server reconnected.
New tool available: slack:send_dm
```

```bash
# Uninstall a plugin
> /mcp uninstall mcp-pg-admin
```

```
Uninstalling mcp-pg-admin...
  Server disconnected.
  Removed from config.toml.
  Package cache cleared.

Uninstalled mcp-pg-admin.
```

### Step 7: Verify a plugin manually

Run the verification pipeline on any plugin (useful for community plugins).

```bash
> /mcp verify mcp-pg-admin
```

```
Verifying mcp-pg-admin v0.5.0...

  [1/4] Checksum          ✓ SHA-256 match
  [2/4] Permissions       ⚠ Requests 'shell' access (undeclared in manifest)
                            -> vacuum and reindex use pg CLI commands
  [3/4] Sandbox           ✓ All 5 tools responded correctly
  [4/4] Community         ○ 28 ratings (below 'Verified' threshold)

Result: PASS with warnings
  Warning: 'shell' permission is used but not declared in mcp-plugin.toml.
  This means the plugin can execute shell commands. Review the source code
  at https://github.com/community/mcp-pg-admin before installing.

Install anyway? [y/n]:
```

### Step 8: Plugin manifest format

Plugins use `mcp-plugin.toml` to declare metadata, permissions, and configuration.

```toml
# mcp-plugin.toml

[plugin]
name = "mcp-postgres"
version = "1.2.0"
description = "Official PostgreSQL MCP server"
author = "@modelcontextprotocol"
license = "MIT"
repository = "https://github.com/modelcontextprotocol/servers"
homepage = "https://modelcontextprotocol.io/servers/postgres"

[plugin.categories]
primary = "Data & Databases"
secondary = ["Developer Tools"]

[plugin.runtime]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-postgres"]
transport = "stdio"
requires_node = ">=18.0.0"

[plugin.permissions]
network = true
filesystem = false
shell = false
environment = ["PGPASSWORD"]

[plugin.config]
connection_string = { type = "string", required = true, description = "PostgreSQL connection URL" }
read_only = { type = "boolean", required = false, default = false, description = "Restrict to SELECT queries" }
max_rows = { type = "integer", required = false, default = 1000, description = "Maximum rows per query" }

[plugin.tools]
count = 5
names = ["query", "list_tables", "describe_table", "create_table", "alter_table"]

[plugin.resources]
count = 2
names = ["schema://tables", "schema://indexes"]
```

### Step 9: Rating and review system

Rate plugins you have used.

```bash
> /mcp rate mcp-postgres 5 --review "Excellent PostgreSQL integration. Fast queries, good schema inspection."
```

```
Rating submitted!
  Plugin:  mcp-postgres
  Rating:  ★★★★★ (5/5)
  Review:  "Excellent PostgreSQL integration. Fast queries, good schema inspection."

Thank you for your review! Community reviews help others discover quality plugins.
```

View reviews:

```bash
> /mcp reviews mcp-postgres
```

```
Reviews for mcp-postgres (★★★★★ 4.8, 343 ratings)
=====================================================

★★★★★  "Excellent PostgreSQL integration. Fast queries..."
  — you, just now

★★★★★  "Works perfectly with Supabase-hosted Postgres too."
  — @dev_sarah, 2 days ago

★★★★☆  "Great tools, but would love a 'run migration' tool."
  — @backend_bob, 1 week ago

★★★★★  "Handles large result sets well. describe_table is super useful."
  — @data_analyst, 2 weeks ago

Showing 4 of 343 reviews. Use --all to see more.
```

### Step 10: Use the MCP Directory panel in VibeUI

Open VibeUI and navigate to the **MCP** panel, then select the **Directory** sub-tab.

```bash
cd vibeui && npm run tauri dev
```

The MCP Directory panel has three tabs:

1. **Browse** -- Grid of plugin cards organized by category. Each card shows the plugin name, author, version, rating, download count, and verification badge. Click a card for the detail view with tools, permissions, and install button.

2. **Installed** -- List of installed plugins with version, connection status, and auto-update toggle. One-click update and uninstall buttons. Shows changelogs inline when updates are available.

3. **Search** -- Full-text search across all plugins. Filter by category, minimum rating, verification status, and transport type. Sort by relevance, downloads, rating, or recently updated.

## Demo Recording

```json
{
  "meta": {
    "title": "MCP Plugin Directory",
    "description": "Browse, install, verify, rate, and manage MCP plugins from VibeCody's curated directory.",
    "duration_seconds": 360,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/mcp search --browse", "delay_ms": 3000 }
      ],
      "description": "Browse the plugin directory categories"
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/mcp search --category \"Developer Tools\"", "delay_ms": 3000 }
      ],
      "description": "List all plugins in the Developer Tools category"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/mcp search postgres", "delay_ms": 2000 }
      ],
      "description": "Search for PostgreSQL-related plugins"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/mcp search --detail mcp-postgres", "delay_ms": 2000 }
      ],
      "description": "View detailed information about the mcp-postgres plugin"
    },
    {
      "id": 5,
      "action": "Narrate",
      "value": "The plugin is verified with checksum, permission audit, and sandbox test. It only requests network access. Let's install it."
    },
    {
      "id": 6,
      "action": "repl",
      "commands": [
        { "input": "/mcp install mcp-postgres", "delay_ms": 5000 }
      ],
      "description": "Install the mcp-postgres plugin with full verification pipeline"
    },
    {
      "id": 7,
      "action": "Narrate",
      "value": "Installation complete. The plugin was verified (checksum, permissions, sandbox) and added to config.toml automatically."
    },
    {
      "id": 8,
      "action": "repl",
      "commands": [
        { "input": "/mcp installed", "delay_ms": 2000 }
      ],
      "description": "List all installed plugins with their status"
    },
    {
      "id": 9,
      "action": "repl",
      "commands": [
        { "input": "/mcp verify mcp-pg-admin", "delay_ms": 4000 }
      ],
      "description": "Manually verify a community plugin before installing"
    },
    {
      "id": 10,
      "action": "Narrate",
      "value": "The community plugin has a warning: it uses shell access that is not declared in its manifest. Always review warnings before installing."
    },
    {
      "id": 11,
      "action": "repl",
      "commands": [
        { "input": "/mcp update mcp-slack", "delay_ms": 4000 }
      ],
      "description": "Update an installed plugin to the latest version"
    },
    {
      "id": 12,
      "action": "repl",
      "commands": [
        { "input": "/mcp rate mcp-postgres 5 --review \"Works great with VibeCody agent loops.\"", "delay_ms": 2000 }
      ],
      "description": "Rate and review an installed plugin"
    },
    {
      "id": 13,
      "action": "repl",
      "commands": [
        { "input": "/mcp reviews mcp-postgres", "delay_ms": 2000 }
      ],
      "description": "View community reviews for a plugin"
    },
    {
      "id": 14,
      "action": "repl",
      "commands": [
        { "input": "/mcp uninstall mcp-pg-admin", "delay_ms": 2000 }
      ],
      "description": "Uninstall a plugin and remove it from config"
    },
    {
      "id": 15,
      "action": "shell",
      "command": "cd vibeui && npm run tauri dev",
      "description": "Launch VibeUI to explore the MCP Directory panel",
      "delay_ms": 8000
    },
    {
      "id": 16,
      "action": "Navigate",
      "target": "panel://mcp",
      "description": "Open the MCP panel in VibeUI"
    },
    {
      "id": 17,
      "action": "Click",
      "target": ".tab-directory-browse",
      "description": "Open the Browse tab with category grid"
    },
    {
      "id": 18,
      "action": "Screenshot",
      "label": "mcp-directory-browse",
      "description": "Capture the plugin directory browse view with category cards"
    },
    {
      "id": 19,
      "action": "Click",
      "target": ".category-card[data-category='Communication']",
      "description": "Click the Communication category to see Slack, Discord, etc."
    },
    {
      "id": 20,
      "action": "Screenshot",
      "label": "mcp-directory-category",
      "description": "Capture the Communication category plugin list"
    },
    {
      "id": 21,
      "action": "Click",
      "target": ".tab-directory-installed",
      "description": "Switch to the Installed tab"
    },
    {
      "id": 22,
      "action": "Screenshot",
      "label": "mcp-directory-installed",
      "description": "Capture installed plugins with update and uninstall controls"
    },
    {
      "id": 23,
      "action": "Click",
      "target": ".tab-directory-search",
      "description": "Switch to the Search tab"
    },
    {
      "id": 24,
      "action": "Type",
      "target": ".directory-search-input",
      "value": "slack",
      "description": "Search for Slack-related plugins"
    },
    {
      "id": 25,
      "action": "Screenshot",
      "label": "mcp-directory-search",
      "description": "Capture search results with rating and verification filters"
    }
  ]
}
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `Plugin not found` | Check the plugin name spelling; use `/mcp search` to find the correct name |
| Checksum verification failed | The package may have been updated since the directory was last synced; try `/mcp search --refresh` |
| Permission audit warning | Review the warning carefully; undeclared permissions mean the plugin accesses capabilities it did not advertise |
| Sandbox test failed | The plugin may have a bug or require configuration; check the plugin's issue tracker |
| Install hangs | Ensure `npx` or `pip` is in your PATH and has network access; some plugins download dependencies on first run |
| Rating not saved | You must have used the plugin at least once before rating it |

## What's Next

- [Demo 16: MCP Server Integration](../16-mcp-servers/) -- MCP fundamentals and building custom servers
- [Demo 17: MCP Lazy Loading](../17-mcp-lazy-loading/) -- Scale to 100+ servers with on-demand loading
- [Demo 19: Context Bundles](../context-bundles/) -- Create shareable context sets for teams
