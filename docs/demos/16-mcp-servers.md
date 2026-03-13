---
layout: page
title: "Demo 16: MCP Server Integration"
permalink: /demos/16-mcp-servers/
nav_order: 16
parent: Demos
---

# Demo 16: MCP Server Integration

## Overview

The Model Context Protocol (MCP) is an open standard that lets AI assistants connect to external tool servers. VibeCody supports MCP natively, allowing you to extend the agent's capabilities with any MCP-compatible server -- databases, APIs, file systems, SaaS integrations, and custom tools. This demo covers configuring MCP servers, using their tools in agent loops, and building your own.

**Time to complete:** ~20 minutes

## Prerequisites

- VibeCLI installed and configured ([Demo 1](../first-run/))
- Node.js 18+ or Python 3.10+ (for running MCP servers)
- (Optional) VibeUI for the desktop panel experience

## What is MCP?

MCP (Model Context Protocol) defines a JSON-RPC interface between an AI assistant (the **client**) and external **servers** that expose tools, resources, and prompts. Instead of hardcoding integrations, the AI discovers available tools at runtime and calls them through a standardized protocol.

Key concepts:

- **Server** -- A process that exposes tools (functions the AI can call), resources (data the AI can read), and prompts (templates).
- **Client** -- VibeCody connects to servers over stdio or HTTP/SSE transports.
- **Tool** -- A function with a name, description, and JSON Schema for parameters. The AI decides when to call it.
- **Resource** -- Read-only data (files, database rows, API responses) the AI can access.

## Step-by-Step Walkthrough

### Step 1: Configure MCP servers in config.toml

Add MCP server definitions to your VibeCLI configuration.

```toml
# ~/.vibecli/config.toml

[mcp]
enabled = true

# Stdio transport (most common)
[mcp.servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/projects"]
description = "File system access for project directories"

# Another stdio server
[mcp.servers.github]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
env = { GITHUB_TOKEN = "ghp_..." }
description = "GitHub API access (repos, issues, PRs)"

# Python-based server
[mcp.servers.database]
command = "python3"
args = ["-m", "mcp_server_sqlite", "--db", "./data/app.db"]
description = "SQLite database queries"

# HTTP/SSE transport (remote server)
[mcp.servers.remote-api]
url = "http://localhost:8080/mcp"
transport = "sse"
description = "Custom REST API tools"
```

### Step 2: List configured servers

```bash
vibecli repl
> /mcp list
```

Expected output:

```
MCP Servers
============
Name          Transport  Status       Tools  Description
----------    ---------  ----------   -----  --------------------------
filesystem    stdio      connected    11     File system access for project directories
github        stdio      connected    8      GitHub API access (repos, issues, PRs)
database      stdio      connected    4      SQLite database queries
remote-api    sse        disconnected 0      Custom REST API tools

3 connected, 1 disconnected | 23 tools available
```

### Step 3: Connect and disconnect servers

```bash
# Connect a disconnected server
> /mcp connect remote-api

# Disconnect a running server
> /mcp disconnect database

# Reconnect all servers
> /mcp reconnect
```

### Step 4: Browse available tools

List all tools exposed by connected MCP servers.

```bash
> /mcp tools
```

```
MCP Tools (23 available)
=========================

filesystem:
  read_file          Read the contents of a file
  write_file         Write content to a file
  list_directory     List files in a directory
  create_directory   Create a new directory
  move_file          Move or rename a file
  search_files       Search for files by pattern
  get_file_info      Get file metadata
  read_multiple      Read multiple files at once
  edit_file          Apply edits to a file
  list_allowed       List allowed directories
  directory_tree     Get a tree view of a directory

github:
  search_repos       Search GitHub repositories
  get_repo           Get repository details
  list_issues        List issues for a repository
  create_issue       Create a new issue
  list_prs           List pull requests
  create_pr          Create a pull request
  get_file_contents  Get file contents from a repo
  push_files         Push file changes to a branch

database:
  query              Execute a SQL query
  list_tables        List all database tables
  describe_table     Get table schema
  create_table       Create a new table
```

Inspect a specific tool:

```bash
> /mcp tools filesystem:read_file
```

```
Tool: filesystem:read_file
===========================
Description: Read the complete contents of a file from the file system.
Server:      filesystem

Parameters (JSON Schema):
  path (string, required): Path of the file to read.
    Must be within allowed directories.

Example call:
  { "path": "/home/user/projects/src/main.rs" }
```

### Step 5: Use MCP tools in agent loops

When you start an agent loop, VibeCody automatically discovers MCP tools and makes them available alongside built-in tools. The AI decides which tools to use.

```bash
vibecli agent "Find all TODO comments in my project and create a GitHub issue for each one"
```

The agent uses MCP tools transparently:

```
Agent thinking...

Step 1: Search for TODO comments
  [Tool Call] filesystem:search_files
  { "pattern": "TODO", "path": "/home/user/projects" }
  Found 7 files with TODOs

Step 2: Read files to extract context
  [Tool Call] filesystem:read_file
  { "path": "/home/user/projects/src/auth.rs" }
  Found: "// TODO: implement token refresh logic" (line 87)

  [Tool Call] filesystem:read_file
  { "path": "/home/user/projects/src/api.rs" }
  Found: "// TODO: add rate limiting" (line 23)

Step 3: Create GitHub issues
  [Tool Call] github:create_issue
  {
    "repo": "myorg/myproject",
    "title": "Implement token refresh logic in auth.rs",
    "body": "Line 87 of src/auth.rs has a TODO...",
    "labels": ["enhancement"]
  }
  Created issue #42

  [Tool Call] github:create_issue
  {
    "repo": "myorg/myproject",
    "title": "Add rate limiting to API endpoints",
    "body": "Line 23 of src/api.rs has a TODO...",
    "labels": ["enhancement"]
  }
  Created issue #43

  ... (5 more issues created)

Done! Created 7 GitHub issues from TODO comments.
```

### Step 6: Semantic MCP (natural language tool discovery)

VibeCody's Semantic MCP feature lets you describe what you need in plain English, and it finds the right tool across all connected servers.

```bash
> /mcp search "read a file from disk"
```

```
Semantic Tool Search: "read a file from disk"
===============================================
  1. filesystem:read_file      (score: 0.95)  Read the contents of a file
  2. filesystem:read_multiple  (score: 0.82)  Read multiple files at once
  3. github:get_file_contents  (score: 0.71)  Get file contents from a repo
```

The agent uses semantic search automatically when it encounters a task that does not match any tool name exactly -- it searches by intent and picks the best match.

### Step 7: Use the MCP panel in VibeUI

Open VibeUI and navigate to the **MCP** panel.

```bash
cd vibeui && npm run tauri dev
```

The MCP panel provides:

1. **Servers** tab -- View all configured servers with connect/disconnect toggles, status indicators, and transport details. Add new servers through a form.

2. **Tools** tab -- Browse all available tools in a searchable list. Click a tool to see its schema, description, and try it with sample inputs.

3. **Logs** tab -- Real-time log of MCP messages (JSON-RPC calls and responses) for debugging.

4. **Config** tab -- Edit your MCP configuration directly. Changes are saved to config.toml.

### Step 8: Build a custom MCP server

Create a minimal MCP server in Python that exposes a weather tool.

```python
# weather_server.py
from mcp.server import Server
from mcp.types import Tool, TextContent
import json

server = Server("weather")

@server.list_tools()
async def list_tools():
    return [
        Tool(
            name="get_weather",
            description="Get current weather for a city",
            inputSchema={
                "type": "object",
                "properties": {
                    "city": {
                        "type": "string",
                        "description": "City name"
                    }
                },
                "required": ["city"]
            }
        )
    ]

@server.call_tool()
async def call_tool(name: str, arguments: dict):
    if name == "get_weather":
        city = arguments["city"]
        # In production, call a real weather API here
        return [TextContent(
            type="text",
            text=json.dumps({
                "city": city,
                "temperature": "22C",
                "condition": "Sunny",
                "humidity": "45%"
            })
        )]

if __name__ == "__main__":
    import asyncio
    from mcp.server.stdio import stdio_server

    async def main():
        async with stdio_server() as (read, write):
            await server.run(read, write)

    asyncio.run(main())
```

Install the MCP SDK and register your server:

```bash
pip install mcp
```

Add to config.toml:

```toml
[mcp.servers.weather]
command = "python3"
args = ["weather_server.py"]
description = "Custom weather data"
```

Test it:

```bash
vibecli repl
> /mcp connect weather
> /mcp tools weather:get_weather
> vibecli agent "What's the weather in Tokyo?"
```

```
Agent thinking...
  [Tool Call] weather:get_weather
  { "city": "Tokyo" }
  Result: {"city": "Tokyo", "temperature": "22C", "condition": "Sunny", "humidity": "45%"}

The current weather in Tokyo is 22C and sunny with 45% humidity.
```

## Demo Recording

```json
{
  "meta": {
    "title": "MCP Server Integration",
    "description": "Configure MCP servers, discover tools, use them in agent loops, and build a custom MCP server.",
    "duration_seconds": 420,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "write_file",
      "path": "~/.vibecli/config.toml",
      "content": "[mcp]\nenabled = true\n\n[mcp.servers.filesystem]\ncommand = \"npx\"\nargs = [\"-y\", \"@modelcontextprotocol/server-filesystem\", \"/tmp/demo\"]\ndescription = \"File system access\"\n\n[mcp.servers.github]\ncommand = \"npx\"\nargs = [\"-y\", \"@modelcontextprotocol/server-github\"]\nenv = { GITHUB_TOKEN = \"ghp_demo\" }\ndescription = \"GitHub API access\"\n",
      "description": "Configure two MCP servers in config.toml",
      "delay_ms": 1000
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/mcp list", "delay_ms": 3000 }
      ],
      "description": "List all configured MCP servers and their connection status"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/mcp tools", "delay_ms": 3000 }
      ],
      "description": "Browse all available tools across connected servers"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/mcp tools filesystem:read_file", "delay_ms": 2000 }
      ],
      "description": "Inspect the schema and description of a specific tool"
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/mcp search \"read a file from disk\"", "delay_ms": 2000 }
      ],
      "description": "Use semantic search to find tools by natural language description"
    },
    {
      "id": 6,
      "action": "Narrate",
      "value": "Now let's see MCP tools used in a real agent loop. The agent will automatically discover and use the right tools."
    },
    {
      "id": 7,
      "action": "shell",
      "command": "vibecli agent \"List all files in /tmp/demo and summarize their contents\"",
      "description": "Run an agent loop that uses MCP filesystem tools",
      "delay_ms": 10000
    },
    {
      "id": 8,
      "action": "repl",
      "commands": [
        { "input": "/mcp disconnect github", "delay_ms": 1500 },
        { "input": "/mcp connect github", "delay_ms": 3000 }
      ],
      "description": "Disconnect and reconnect a server"
    },
    {
      "id": 9,
      "action": "Narrate",
      "value": "Let's build a custom MCP server and register it with VibeCody."
    },
    {
      "id": 10,
      "action": "write_file",
      "path": "/tmp/demo/weather_server.py",
      "content": "from mcp.server import Server\nfrom mcp.types import Tool, TextContent\nimport json\n\nserver = Server(\"weather\")\n\n@server.list_tools()\nasync def list_tools():\n    return [Tool(name=\"get_weather\", description=\"Get weather for a city\", inputSchema={\"type\":\"object\",\"properties\":{\"city\":{\"type\":\"string\"}},\"required\":[\"city\"]})]\n\n@server.call_tool()\nasync def call_tool(name, arguments):\n    return [TextContent(type=\"text\", text=json.dumps({\"city\":arguments[\"city\"],\"temp\":\"22C\",\"condition\":\"Sunny\"}))]\n",
      "description": "Create a custom weather MCP server",
      "delay_ms": 1000
    },
    {
      "id": 11,
      "action": "repl",
      "commands": [
        { "input": "/mcp connect weather", "delay_ms": 3000 },
        { "input": "/mcp tools weather:get_weather", "delay_ms": 2000 }
      ],
      "description": "Connect the custom server and inspect its tools"
    },
    {
      "id": 12,
      "action": "shell",
      "command": "vibecli agent \"What's the weather in Paris?\"",
      "description": "Use the custom MCP tool in an agent loop",
      "delay_ms": 5000
    },
    {
      "id": 13,
      "action": "shell",
      "command": "cd vibeui && npm run tauri dev",
      "description": "Launch VibeUI to explore the MCP panel",
      "delay_ms": 8000
    },
    {
      "id": 14,
      "action": "Navigate",
      "target": "panel://mcp",
      "description": "Open the MCP panel in VibeUI"
    },
    {
      "id": 15,
      "action": "Click",
      "target": ".tab-servers",
      "description": "View the Servers tab with connection status"
    },
    {
      "id": 16,
      "action": "Screenshot",
      "label": "mcp-servers-connected",
      "description": "Capture the MCP panel showing connected servers"
    },
    {
      "id": 17,
      "action": "Click",
      "target": ".tab-tools",
      "description": "Switch to the Tools tab"
    },
    {
      "id": 18,
      "action": "Type",
      "target": ".tool-search-input",
      "value": "file",
      "description": "Search for file-related tools"
    },
    {
      "id": 19,
      "action": "Screenshot",
      "label": "mcp-tools-search",
      "description": "Capture filtered tool list"
    },
    {
      "id": 20,
      "action": "Click",
      "target": ".tab-logs",
      "description": "View the real-time MCP message log"
    },
    {
      "id": 21,
      "action": "Screenshot",
      "label": "mcp-logs",
      "description": "Capture the JSON-RPC message log for debugging"
    }
  ]
}
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `Server failed to start` | Check that the command and args are correct; run the command manually to see errors |
| `npx` not found | Install Node.js 18+ and ensure `npx` is in your PATH |
| `Connection timeout` | For SSE transport, verify the server URL is reachable; for stdio, check the process starts cleanly |
| Tools not appearing | Run `/mcp tools` to refresh; some servers take a few seconds to initialize |
| `Permission denied` in filesystem server | The filesystem server only allows access to explicitly listed directories |
| Agent ignores MCP tools | Ensure `[mcp] enabled = true` in config.toml; the agent uses MCP tools when they match the task |

## What's Next

- [Demo 17: MCP Lazy Loading](../17-mcp-lazy-loading/) -- Scale to 100+ MCP servers with on-demand tool loading
- [Demo 18: MCP Plugin Directory](../18-mcp-directory/) -- Browse and install verified MCP plugins
- [Demo 4: Agent Loop & Tool Execution](../04-agent-loop/) -- See how agents use tools autonomously
