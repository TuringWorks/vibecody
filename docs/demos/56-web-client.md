---
layout: page
title: "Demo 56: Browser-Based Web Client"
permalink: /demos/56-web-client/
nav_order: 56
parent: Demos
---


## Overview

VibeCody ships a zero-dependency, single-page web client embedded directly in the `vibecli --serve` HTTP daemon. No CDN, no npm install, no build step -- just start the server and open your browser. The web client provides a full chat interface with agent mode support, SSE streaming, dark and light themes, and works completely offline in air-gapped environments. It is a pure HTML/CSS/JS application served from memory with no external fetches.

**Time to complete:** ~5 minutes

## Prerequisites

- VibeCLI v0.5.1 installed and on your PATH
- At least one AI provider configured (Ollama recommended for air-gap demos)
- A modern web browser (Chrome, Firefox, Safari, Edge)

## Step-by-Step Walkthrough

### Step 1: Start the HTTP Daemon

Launch VibeCLI in serve mode on a port of your choice.

```bash
vibecli --serve --port 7878
```

Expected output:

```
VibeCLI HTTP Daemon v0.5.1
  Listening:  http://localhost:7878
  Provider:   claude (claude-sonnet-4-6)
  Web client: http://localhost:7878/ (built-in SPA)
  API docs:   http://localhost:7878/docs
  Health:     http://localhost:7878/health

Ready for connections. Press Ctrl+C to stop.
```

### Step 2: Open the Web Client

Navigate to `http://localhost:7878` in your browser. The web client loads instantly from the embedded SPA -- no external resources are fetched.

The interface consists of:

- **Header** -- VibeCody logo, provider indicator, theme toggle
- **Chat area** -- Scrollable message history with syntax-highlighted code blocks
- **Input bar** -- Message input with send button and mode selector
- **Status bar** -- Connection status, token count, model name

### Step 3: Send a Chat Message

Type a message in the input bar and press Enter or click Send. Responses stream in real-time via Server-Sent Events (SSE).

```
You: Explain the difference between Arc and Rc in Rust.

VibeCody: Arc and Rc are both reference-counted smart pointers in Rust,
but they differ in thread safety:

- **Rc<T>** (Reference Counted) is for single-threaded use only. It
  uses non-atomic reference counting, which is faster but not safe to
  share across threads.

- **Arc<T>** (Atomically Reference Counted) is thread-safe. It uses
  atomic operations for the reference count, adding a small performance
  overhead but allowing safe sharing across threads.

Use Rc when you know the data stays on one thread. Use Arc when you
need to share data between threads (e.g., with std::thread::spawn or
tokio::spawn).
```

### Step 4: Switch to Agent Mode

Click the mode selector in the input bar to switch between **Chat** and **Agent** modes.

In Agent mode, VibeCody can read files, write files, and execute shell commands -- the same agent loop available in the CLI. Tool calls appear inline in the conversation with collapsible output.

```
You: [Agent Mode] Add a health check endpoint to src/server.rs

VibeCody: I'll add a health check endpoint. Let me examine the current
server code first.

  [tool: read_file] src/server.rs
  ▼ (click to expand - 42 lines)

Now I'll add the /health endpoint:

  [tool: edit_file] src/server.rs
  ▼ Added GET /health handler returning JSON status

  [tool: bash] cargo check
  ▼ Compiling server v0.1.0 -- success, 0 warnings

Done. The /health endpoint returns:
  GET /health -> {"status": "ok", "uptime_secs": 1234}
```

### Step 5: Toggle Dark and Light Theme

Click the theme toggle in the header to switch between dark and light modes. The theme uses CSS variables with no JavaScript reflow, so the switch is instant.

- **Dark theme** -- Dark background (#1a1a2e), light text, syntax colors optimized for dark backgrounds
- **Light theme** -- White background, dark text, syntax colors optimized for readability

The selected theme persists in `localStorage`.

### Step 6: Verify Air-Gap Safety

The web client makes zero external network requests. You can verify this by opening your browser's Network tab in Developer Tools before loading the page.

All resources served:

```
Request                     Size     Source
/                          18 KB    embedded HTML + CSS + JS
/health                     0.1 KB  API endpoint
/api/chat (SSE stream)     varies   API endpoint
```

No requests to CDNs, fonts, analytics, or any other external domain.

### Step 7: Use the REST API Directly

The daemon also exposes a JSON API for programmatic access.

**Health check:**

```bash
curl http://localhost:7878/health
```

```json
{
  "status": "ok",
  "version": "0.5.1",
  "provider": "claude",
  "model": "claude-sonnet-4-6",
  "uptime_secs": 342
}
```

**Send a chat message:**

```bash
curl -X POST http://localhost:7878/api/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "What is 2+2?", "stream": false}'
```

```json
{
  "response": "2 + 2 = 4.",
  "tokens": { "input": 12, "output": 8 },
  "model": "claude-sonnet-4-6"
}
```

**Stream a response (SSE):**

```bash
curl -N http://localhost:7878/api/chat/stream \
  -H "Content-Type: application/json" \
  -d '{"message": "Write a haiku about Rust"}'
```

```
data: {"type":"token","content":"Ownership"}
data: {"type":"token","content":" guards"}
data: {"type":"token","content":" the"}
data: {"type":"token","content":" gate"}
data: {"type":"token","content":"\n"}
data: {"type":"token","content":"Borrow"}
...
data: {"type":"done","tokens":{"input":14,"output":22}}
```

## Demo Recording JSON

```json
{
  "meta": {
    "title": "Browser-Based Web Client",
    "description": "Zero-CDN embedded SPA with chat, agent mode, and SSE streaming.",
    "duration_seconds": 180,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "shell",
      "command": "vibecli --serve --port 7878",
      "description": "Start the HTTP daemon with embedded web client",
      "delay_ms": 3000
    },
    {
      "id": 2,
      "action": "shell",
      "command": "curl -s http://localhost:7878/health | python3 -m json.tool",
      "description": "Verify the daemon is running",
      "expected_output_contains": "\"status\": \"ok\"",
      "delay_ms": 2000
    },
    {
      "id": 3,
      "action": "browser",
      "url": "http://localhost:7878",
      "description": "Open the web client in a browser"
    },
    {
      "id": 4,
      "action": "shell",
      "command": "curl -X POST http://localhost:7878/api/chat -H 'Content-Type: application/json' -d '{\"message\": \"What is 2+2?\", \"stream\": false}'",
      "description": "Send a chat message via REST API",
      "expected_output_contains": "response",
      "delay_ms": 3000
    }
  ]
}
```

## What's Next

- [Demo 55: Voice, Pairing & Tailscale](../55-voice-pairing-tailscale/) -- Remote access with Tailscale Funnel
- [Demo 01: First Run & Setup](../01-first-run/) -- Install and configure VibeCLI
- [Demo 27: HTTP Playground](../27-http-playground/) -- Test APIs with collections and history
