---
layout: page
title: "Demo 17: MCP Lazy Loading"
permalink: /demos/17-mcp-lazy-loading/
nav_order: 17
parent: Demos
---


## Overview

When you connect dozens or hundreds of MCP servers, each exposing multiple tools, the total tool schema can consume significant context window space. MCP Lazy Loading solves this by maintaining a lightweight tool registry and loading full schemas only when a tool is actually needed. This demo shows how to configure lazy loading, search the registry, and monitor context savings.

**Time to complete:** ~15 minutes

## Prerequisites

- VibeCLI installed and configured ([Demo 1](../first-run/))
- MCP basics understood ([Demo 16](../16-mcp-servers/))
- At least 3 MCP servers configured (more servers make the benefits more visible)
- (Optional) VibeUI for the desktop panel experience

## Why Lazy Loading Matters

Without lazy loading, every connected MCP server's full tool schemas are injected into the AI's system prompt at the start of each conversation. With many servers, this creates problems:

| Scenario | Without Lazy Loading | With Lazy Loading |
|----------|---------------------|-------------------|
| 5 servers, 30 tools | ~8 KB context | ~1 KB context |
| 20 servers, 150 tools | ~40 KB context | ~2 KB context |
| 100 servers, 800 tools | ~200 KB context | ~4 KB context |

At 100+ servers, the tool schemas alone can exceed model context limits. Lazy loading reduces this by **up to 95%** by only injecting the registry index (tool name + one-line description) into context, then loading full schemas on demand when the AI selects a tool.

## Step-by-Step Walkthrough

### Step 1: Enable lazy loading

Add the lazy loading configuration to your config.toml.

```toml
# ~/.vibecli/config.toml

[mcp]
enabled = true
lazy_loading = true

[mcp.lazy]
# Maximum number of full schemas to keep in memory
cache_size = 50

# Eviction policy: "lru" (least recently used) or "lfu" (least frequently used)
eviction_policy = "lru"

# Time-to-live for cached schemas (seconds, 0 = no expiry)
schema_ttl = 3600

# Log cache hits/misses for debugging
metrics_enabled = true
```

### Step 2: Configure multiple MCP servers

For this demo, assume you have several servers configured:

```toml
[mcp.servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/projects"]

[mcp.servers.github]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
env = { GITHUB_TOKEN = "ghp_..." }

[mcp.servers.postgres]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-postgres", "postgresql://localhost/myapp"]

[mcp.servers.slack]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-slack"]
env = { SLACK_BOT_TOKEN = "xoxb-..." }

[mcp.servers.browser]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-puppeteer"]

[mcp.servers.memory]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-memory"]

[mcp.servers.google-maps]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-google-maps"]
env = { GOOGLE_MAPS_KEY = "AIza..." }

[mcp.servers.brave-search]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-brave-search"]
env = { BRAVE_API_KEY = "BSA..." }
```

### Step 3: View the tool registry

With lazy loading enabled, `/mcp tools` shows the lightweight registry instead of full schemas.

```bash
vibecli
> /mcp tools
```

```
MCP Tool Registry (lazy loading enabled)
==========================================
53 tools across 8 servers | 42 KB saved vs eager loading

filesystem (11 tools)
  read_file          Read file contents
  write_file         Write content to file
  list_directory     List directory entries
  search_files       Search by pattern
  ... +7 more

github (8 tools)
  search_repos       Search repositories
  create_issue       Create an issue
  create_pr          Create a pull request
  ... +5 more

postgres (5 tools)
  query              Execute SQL query
  list_tables        List database tables
  ... +3 more

slack (6 tools)
  send_message       Send a message to a channel
  list_channels      List workspace channels
  ... +4 more

browser (4 tools)
  navigate           Navigate to a URL
  screenshot         Take a screenshot
  ... +2 more

memory (4 tools)
  store              Store a key-value pair
  retrieve           Retrieve by key
  ... +2 more

google-maps (5 tools)
  geocode            Convert address to coordinates
  directions         Get driving directions
  ... +3 more

brave-search (3 tools)
  web_search         Search the web
  ... +2 more

Schemas loaded: 0/53 | Cache: empty
```

Notice that only tool names and one-line descriptions are shown. No full JSON schemas are in memory yet.

### Step 4: Search the tool registry

Use keyword search to find tools across all servers without loading their schemas.

```bash
> /mcp search "send a message"
```

```
Tool Search: "send a message"
===============================
  1. slack:send_message        (score: 0.94)  Send a message to a channel
  2. slack:send_dm             (score: 0.81)  Send a direct message
  3. memory:store              (score: 0.32)  Store a key-value pair

Top match: slack:send_message
Load full schema? [y/n/auto]:
```

When the agent needs a tool, it searches the registry, and VibeCody loads the full schema on demand:

```bash
vibecli --agent "Send a Slack message to #engineering saying the deploy is complete"
```

```
Agent thinking...
  [Registry Search] "send message slack" -> slack:send_message (score: 0.94)
  [Schema Load] Loading slack:send_message schema (first use)...
  [Tool Call] slack:send_message
  {
    "channel": "#engineering",
    "text": "Deploy is complete! All services are healthy."
  }
  Message sent to #engineering

Done! Slack message sent.
  Schema cache: 1/53 loaded | Context saved: 41.2 KB
```

### Step 5: Monitor cache and context metrics

View real-time metrics on lazy loading performance.

```bash
> /mcp lazy metrics
```

```
MCP Lazy Loading Metrics
=========================
Registry size:     53 tools across 8 servers
Schemas cached:    7/53 (13%)
Cache hits:        23
Cache misses:      7 (first loads)
Cache evictions:   0
Schema TTL:        3600s (1 hour)
Eviction policy:   LRU

Context Savings:
  Eager loading would use:    43,720 bytes
  Current context usage:       2,140 bytes (registry index)
  Cached schemas in context:   4,890 bytes
  Total context:               7,030 bytes
  Savings:                    36,690 bytes (83.9%)

Most used tools (this session):
  filesystem:read_file       12 calls
  github:create_issue         5 calls
  postgres:query              4 calls
  slack:send_message          2 calls
  filesystem:list_directory   1 call
  filesystem:search_files     1 call
  brave-search:web_search     1 call
```

### Step 6: Manage the schema cache

```bash
# Clear the entire cache (schemas will be reloaded on next use)
> /mcp lazy clear

# Preload schemas for specific servers you know you'll need
> /mcp lazy preload filesystem github

# Pin schemas so they are never evicted
> /mcp lazy pin filesystem:read_file filesystem:write_file

# View cache contents
> /mcp lazy cache
```

```
Schema Cache Contents
======================
Tool                        Size     Last Used         Pinned
-----------------------     ------   ----------------  ------
filesystem:read_file        1.2 KB   2 min ago         YES
filesystem:write_file       1.1 KB   5 min ago         YES
github:create_issue         0.9 KB   8 min ago         no
postgres:query              0.8 KB   12 min ago        no
slack:send_message          0.7 KB   15 min ago        no

5 cached | 2 pinned | 4.7 KB total
Available capacity: 45/50 slots
```

### Step 7: Use the MCP Lazy panel in VibeUI

Open VibeUI and navigate to the **MCP** panel, which has a **Lazy** sub-tab when lazy loading is enabled.

```bash
cd vibeui && npm run tauri dev
```

The MCP Lazy panel has three tabs:

1. **Tool Registry** -- Searchable table of all registered tools with server name, tool name, description, and cached/loaded status. Click a tool to load its full schema and see parameter details.

2. **Search** -- Full-text and semantic search across the tool registry. Results are ranked by relevance score. Click "Load & Use" to inject a tool schema into the current agent context.

3. **Metrics** -- Dashboard showing context savings (bar chart: eager vs lazy), cache hit/miss ratio (donut chart), most-used tools (bar chart), and cache size over time (line chart).

### Step 8: Tune lazy loading for your workflow

Adjust settings based on your usage patterns.

```toml
[mcp.lazy]
# For small setups (< 20 tools): disable lazy loading
# cache_size = 0  (not needed)

# For medium setups (20-100 tools):
cache_size = 30
eviction_policy = "lru"
schema_ttl = 3600

# For large setups (100+ tools):
cache_size = 100
eviction_policy = "lfu"
schema_ttl = 7200

# Always preload critical tools
preload = ["filesystem:read_file", "filesystem:write_file", "github:create_issue"]

# Pin tools that should never be evicted
pinned = ["filesystem:read_file", "filesystem:write_file"]
```

## Demo Recording

```json
{
  "meta": {
    "title": "MCP Lazy Loading",
    "description": "Scale to 100+ MCP servers with on-demand tool schema loading, LRU caching, and context savings.",
    "duration_seconds": 360,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "write_file",
      "path": "~/.vibecli/config.toml",
      "content": "[mcp]\nenabled = true\nlazy_loading = true\n\n[mcp.lazy]\ncache_size = 50\neviction_policy = \"lru\"\nschema_ttl = 3600\nmetrics_enabled = true\n\n[mcp.servers.filesystem]\ncommand = \"npx\"\nargs = [\"-y\", \"@modelcontextprotocol/server-filesystem\", \"/tmp/demo\"]\n\n[mcp.servers.github]\ncommand = \"npx\"\nargs = [\"-y\", \"@modelcontextprotocol/server-github\"]\nenv = { GITHUB_TOKEN = \"ghp_demo\" }\n\n[mcp.servers.slack]\ncommand = \"npx\"\nargs = [\"-y\", \"@modelcontextprotocol/server-slack\"]\nenv = { SLACK_BOT_TOKEN = \"xoxb-demo\" }\n\n[mcp.servers.postgres]\ncommand = \"npx\"\nargs = [\"-y\", \"@modelcontextprotocol/server-postgres\", \"postgresql://localhost/demo\"]\n\n[mcp.servers.memory]\ncommand = \"npx\"\nargs = [\"-y\", \"@modelcontextprotocol/server-memory\"]\n",
      "description": "Configure lazy loading with 5 MCP servers",
      "delay_ms": 1000
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/mcp list", "delay_ms": 3000 }
      ],
      "description": "List all configured servers and verify connections"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/mcp tools", "delay_ms": 3000 }
      ],
      "description": "View the lightweight tool registry (no full schemas loaded yet)"
    },
    {
      "id": 4,
      "action": "Narrate",
      "value": "Notice that only tool names and one-line descriptions are shown. Zero full schemas are in memory -- this is lazy loading in action."
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/mcp search \"send a message\"", "delay_ms": 2000 }
      ],
      "description": "Search the tool registry by keyword"
    },
    {
      "id": 6,
      "action": "repl",
      "commands": [
        { "input": "/mcp search \"query database\"", "delay_ms": 2000 }
      ],
      "description": "Search for database-related tools"
    },
    {
      "id": 7,
      "action": "shell",
      "command": "vibecli --agent \"Read the file /tmp/demo/readme.txt and summarize it\"",
      "description": "Run an agent that triggers on-demand schema loading for filesystem tools",
      "delay_ms": 8000
    },
    {
      "id": 8,
      "action": "repl",
      "commands": [
        { "input": "/mcp lazy metrics", "delay_ms": 2000 }
      ],
      "description": "View cache metrics and context savings after the agent run"
    },
    {
      "id": 9,
      "action": "Narrate",
      "value": "The agent loaded only the filesystem:read_file schema on demand. Context savings are over 80%."
    },
    {
      "id": 10,
      "action": "repl",
      "commands": [
        { "input": "/mcp lazy cache", "delay_ms": 1500 }
      ],
      "description": "View which schemas are currently cached"
    },
    {
      "id": 11,
      "action": "repl",
      "commands": [
        { "input": "/mcp lazy pin filesystem:read_file filesystem:write_file", "delay_ms": 1500 }
      ],
      "description": "Pin frequently used tools so they are never evicted"
    },
    {
      "id": 12,
      "action": "repl",
      "commands": [
        { "input": "/mcp lazy preload github", "delay_ms": 3000 }
      ],
      "description": "Preload all GitHub tool schemas into the cache"
    },
    {
      "id": 13,
      "action": "repl",
      "commands": [
        { "input": "/mcp lazy metrics", "delay_ms": 2000 }
      ],
      "description": "View updated metrics after preloading and pinning"
    },
    {
      "id": 14,
      "action": "shell",
      "command": "cd vibeui && npm run tauri dev",
      "description": "Launch VibeUI to explore the MCP Lazy panel",
      "delay_ms": 8000
    },
    {
      "id": 15,
      "action": "Navigate",
      "target": "panel://mcp",
      "description": "Open the MCP panel in VibeUI"
    },
    {
      "id": 16,
      "action": "Click",
      "target": ".tab-lazy-registry",
      "description": "View the Tool Registry tab with loaded/unloaded status indicators"
    },
    {
      "id": 17,
      "action": "Screenshot",
      "label": "mcp-lazy-registry",
      "description": "Capture the tool registry showing cached vs uncached tools"
    },
    {
      "id": 18,
      "action": "Click",
      "target": ".tab-lazy-search",
      "description": "Switch to the Search tab"
    },
    {
      "id": 19,
      "action": "Type",
      "target": ".lazy-search-input",
      "value": "create issue",
      "description": "Search for issue-creation tools"
    },
    {
      "id": 20,
      "action": "Screenshot",
      "label": "mcp-lazy-search",
      "description": "Capture search results ranked by relevance"
    },
    {
      "id": 21,
      "action": "Click",
      "target": ".tab-lazy-metrics",
      "description": "Switch to the Metrics dashboard"
    },
    {
      "id": 22,
      "action": "Screenshot",
      "label": "mcp-lazy-metrics",
      "description": "Capture the context savings chart and cache statistics"
    },
    {
      "id": 23,
      "action": "repl",
      "commands": [
        { "input": "/mcp lazy clear", "delay_ms": 1000 },
        { "input": "/mcp lazy metrics", "delay_ms": 1500 }
      ],
      "description": "Clear the cache and verify it is empty"
    }
  ]
}
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Agent cannot find the right tool | Use `/mcp search` to verify the tool exists in the registry; check server connections |
| Cache evictions causing re-loads | Increase `cache_size` or pin critical tools with `/mcp lazy pin` |
| Metrics show 0% savings | Verify `lazy_loading = true` in config.toml and restart the REPL |
| Schema load timeout | The MCP server may be slow to respond; increase timeout in server config or preload schemas |
| Stale schemas after server update | Run `/mcp lazy clear` to flush the cache, or set a shorter `schema_ttl` |

## What's Next

- [Demo 18: MCP Plugin Directory](../18-mcp-directory/) -- Browse, install, and rate verified MCP plugins
- [Demo 16: MCP Server Integration](../16-mcp-servers/) -- MCP fundamentals and custom servers
- [Demo 4: Agent Loop & Tool Execution](../agent-loop/) -- How agents select and use tools
