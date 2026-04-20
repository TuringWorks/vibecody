---
layout: page
title: API Reference
permalink: /api-reference/
---


Complete HTTP API reference for the VibeCLI daemon (`vibecli serve`).


## Overview

Start the daemon:

```bash
vibecli --serve --port 7878 --provider ollama
```

On startup, a **Bearer token** is printed to stderr. All authenticated endpoints require this token.

| Property | Value |
|----------|-------|
| **Base URL** | `http://localhost:7878` |
| **Content-Type** | `application/json` |
| **Auth** | `Authorization: Bearer <token>` |
| **Max body** | 1 MB |
| **CORS origins** | `localhost`, `127.0.0.1`, `tauri://localhost` |


## Authentication

All endpoints except `/health`, `/webhook/github`, `/pair`, `/acp/v1/capabilities`, and `/ws/collab/:room_id` require a Bearer token.

```bash
# Token is printed on startup:
#   [serve] API token: abc123...

export VIBECLI_TOKEN="abc123..."
```

Unauthenticated requests receive:

```json
{ "error": "Missing or invalid Authorization: Bearer <token>" }
```

**Status:** `401 Unauthorized`

### API Key Rotation

Restart the daemon to generate a new token. A fresh token is printed to stderr on each startup.


## Error Handling

All errors return a consistent JSON structure:

```json
{ "error": "Human-readable error message" }
```

| Status Code | Meaning |
|-------------|---------|
| `400` | Bad request (malformed JSON, missing fields) |
| `401` | Missing or invalid Bearer token |
| `404` | Resource not found (session, job, task) |
| `429` | Rate limit exceeded |
| `500` | Internal server error (provider failure) |

User-supplied input in error messages is sanitized (alphanumeric + `-_.` only, truncated to 200 chars).


## Rate Limiting

Two rate limit tiers apply:

| Tier | Limit | Window | Applies to |
|------|-------|--------|------------|
| **Authenticated** | 60 requests | 60 seconds | All authed endpoints |
| **Public** | 10 requests | 60 seconds | `/health`, `/webhook/github`, etc. |

When the limit is exceeded:

```text
HTTP/1.1 429 Too Many Requests
Retry-After: 5

{ "error": "Rate limit exceeded. Try again shortly." }
```


## Endpoints

### GET /health

Liveness check. No authentication required.

**Response** `200 OK`:

```json
{
  "status": "ok",
  "version": "0.3.3"
}
```

```bash
curl http://localhost:7878/health
```


### POST /chat

Single-turn chat completion (non-streaming). Collects the full response before returning.

**Request body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `messages` | `ChatMessage[]` | Yes | Conversation history |
| `model` | `string` | No | Override the provider's default model |

**ChatMessage:**

| Field | Type | Values |
|-------|------|--------|
| `role` | `string` | `"user"`, `"assistant"`, `"system"` |
| `content` | `string` | Message text |

**Response** `200 OK`:

```json
{
  "content": "The AI response text..."
}
```

**Example:**

```bash
curl -X POST http://localhost:7878/chat \
  -H "Authorization: Bearer $VIBECLI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "messages": [
      {"role": "user", "content": "Explain Rust lifetimes in 3 sentences"}
    ]
  }'
```

**Errors:**

| Status | Cause |
|--------|-------|
| `500` | `LLM provider error: ...` or `Stream error: ...` |


### POST /chat/stream

Streaming chat completion via Server-Sent Events (SSE). Returns tokens as they are generated.

**Request body:** Same as `POST /chat`.

**SSE event types:**

| Event | Data | Description |
|-------|------|-------------|
| `message` (default) | Token text | Incremental content chunk |
| `error` | Error string | Provider or stream error |
| `done` | `""` (empty) | Stream finished |

**Keep-alive:** Every 15 seconds.

**Example:**

```bash
curl -N -X POST http://localhost:7878/chat/stream \
  -H "Authorization: Bearer $VIBECLI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "messages": [
      {"role": "system", "content": "You are a Rust expert."},
      {"role": "user", "content": "Write a binary search function"}
    ]
  }'
```

**Response stream:**

```
data: fn binary_search

data: <T: Ord>(arr: &[T],

data:  target: &T) -> Option<usize>

event: done
data:
```


### POST /agent

Start a background agent task. Returns immediately with a session ID. Subscribe to events via `GET /stream/:session_id`.

**Request body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `task` | `string` | Yes | Natural language task description |
| `approval` | `string` | No | Override approval policy: `"suggest"`, `"auto-edit"`, or `"full-auto"` |

**Response** `200 OK`:

```json
{
  "session_id": "a1b2c3d4e5f6..."
}
```

The `session_id` is a cryptographically random 128-bit hex string.

**Example:**

```bash
curl -X POST http://localhost:7878/agent \
  -H "Authorization: Bearer $VIBECLI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "task": "Add input validation to src/api/handler.rs",
    "approval": "full-auto"
  }'
```


### GET /stream/:session_id

Subscribe to real-time agent events via SSE. Connect after calling `POST /agent`.

**SSE event data (JSON):**

Each event's `data` field is a JSON object with these fields:

| Field | Type | Present when |
|-------|------|-------------|
| `type` | `string` | Always. One of: `chunk`, `step`, `complete`, `error` |
| `content` | `string` | `chunk`, `complete`, `error` |
| `step_num` | `number` | `step` |
| `tool_name` | `string` | `step` |
| `success` | `boolean` | `step` |

**Event types:**

| Type | Description |
|------|-------------|
| `chunk` | Incremental text from the LLM |
| `step` | A tool was executed (e.g., `read_file`, `bash`) |
| `complete` | Agent finished. `content` has the summary |
| `error` | Agent failed. `content` has the error message |

**Example:**

```bash
curl -N http://localhost:7878/stream/a1b2c3d4e5f6... \
  -H "Authorization: Bearer $VIBECLI_TOKEN"
```

**Response stream:**

```
data: {"type":"chunk","content":"Reading the file..."}

data: {"type":"step","step_num":1,"tool_name":"read_file","success":true}

data: {"type":"chunk","content":"Adding validation..."}

data: {"type":"step","step_num":2,"tool_name":"write_file","success":true}

data: {"type":"complete","content":"Added input validation for all 3 handler functions."}
```

**Errors:**

| Status | Cause |
|--------|-------|
| `404` | `Session '<id>' not found` |


### GET /jobs

List all persisted job records, sorted by most recent first.

**Response** `200 OK`:

```json
[
  {
    "session_id": "a1b2c3d4...",
    "task": "Add input validation",
    "status": "complete",
    "provider": "ollama",
    "started_at": 1710700000000,
    "finished_at": 1710700060000,
    "summary": "Added input validation for all 3 handler functions."
  }
]
```

**JobRecord fields:**

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | `string` | Unique job identifier |
| `task` | `string` | Original task description |
| `status` | `string` | `"running"`, `"complete"`, `"failed"`, `"cancelled"` |
| `provider` | `string` | AI provider name |
| `started_at` | `number` | Unix timestamp (milliseconds) |
| `finished_at` | `number?` | Unix timestamp (milliseconds), null if running |
| `summary` | `string?` | Completion summary or error message |

```bash
curl http://localhost:7878/jobs \
  -H "Authorization: Bearer $VIBECLI_TOKEN"
```


### GET /jobs/:id

Get a single job record by session ID.

**Response** `200 OK`: A single `JobRecord` object (same schema as above).

```bash
curl http://localhost:7878/jobs/a1b2c3d4... \
  -H "Authorization: Bearer $VIBECLI_TOKEN"
```

**Errors:** `404` if not found.


### POST /jobs/:id/cancel

Cancel a running job. Removes the SSE stream and marks the job as cancelled.

**Response** `200 OK`: The updated `JobRecord` with `status: "cancelled"`.

```bash
curl -X POST http://localhost:7878/jobs/a1b2c3d4.../cancel \
  -H "Authorization: Bearer $VIBECLI_TOKEN"
```

**Errors:** `404` if not found. If the job is already finished, it returns the record unchanged.


### GET /sessions

HTML page listing all agent sessions. Useful for browsing in a web browser.

```bash
curl http://localhost:7878/sessions \
  -H "Authorization: Bearer $VIBECLI_TOKEN"
```


### GET /sessions.json

JSON list of all sessions (machine-readable alternative to `/sessions`).

```bash
curl http://localhost:7878/sessions.json \
  -H "Authorization: Bearer $VIBECLI_TOKEN"
```


### GET /view/:id

HTML page for a specific session with full conversation history.

```bash
curl http://localhost:7878/view/a1b2c3d4... \
  -H "Authorization: Bearer $VIBECLI_TOKEN"
```


### GET /share/:id

Read-only shareable session view. Displays a "Shared" banner at the top.

```bash
curl http://localhost:7878/share/a1b2c3d4... \
  -H "Authorization: Bearer $VIBECLI_TOKEN"
```


### WS /ws/collab/:room_id

WebSocket endpoint for real-time CRDT collaboration. No Bearer token required (public).

**Connect:**

```bash
websocat ws://localhost:7878/ws/collab/my-room
```

**Message format:** Binary CRDT sync messages from the `vibe-collab` crate. Messages are broadcast to all peers in the room.

**Related REST endpoints (authenticated):**

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/collab/rooms` | Create a new collaboration room |
| `GET` | `/collab/rooms` | List all active rooms |
| `GET` | `/collab/rooms/:room_id/peers` | List peers in a room |


### POST /acp/v1/tasks

Create a task via the Agent Client Protocol. Runs the agent in `full-auto` mode.

**Request body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `task` | `string` | Yes | Task description |
| `context` | `object` | No | Optional context |
| `context.workspace_root` | `string` | No | Override workspace directory |

**Response** `201 Created`:

```json
{
  "id": "acp-a1b2c3d4e5f6...",
  "status": "pending",
  "summary": "Task queued: Add tests for auth module",
  "files_modified": [],
  "steps_completed": 0
}
```

```bash
curl -X POST http://localhost:7878/acp/v1/tasks \
  -H "Authorization: Bearer $VIBECLI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task": "Add tests for auth module"}'
```


### GET /acp/v1/tasks/:id

Get ACP task status.

**Response** `200 OK`:

```json
{
  "id": "acp-a1b2c3d4e5f6...",
  "status": "complete",
  "summary": "ACP task completed",
  "files_modified": [],
  "steps_completed": 0
}
```

```bash
curl http://localhost:7878/acp/v1/tasks/acp-a1b2c3d4e5f6... \
  -H "Authorization: Bearer $VIBECLI_TOKEN"
```


### GET /acp/v1/capabilities

ACP capability advertisement. No authentication required.

```bash
curl http://localhost:7878/acp/v1/capabilities
```


### POST /webhook/github

GitHub App webhook endpoint. No Bearer token required. Uses HMAC-SHA256 signature verification via the `X-Hub-Signature-256` header.

**Headers:**

| Header | Description |
|--------|-------------|
| `X-GitHub-Event` | Event type (e.g., `pull_request`) |
| `X-Hub-Signature-256` | HMAC-SHA256 signature |

**Response** `200 OK`:

```json
{
  "status": "reviewed",
  "findings": 3,
  "summary": "Found 3 issues in the PR"
}
```

Unhandled event types return `{"status": "ignored"}`.


### POST /webhook/skill/:skill_name

Trigger a skill by its `webhook_trigger` name. Requires authentication.

```bash
curl -X POST http://localhost:7878/webhook/skill/deploy-prod \
  -H "Authorization: Bearer $VIBECLI_TOKEN" \
  -d '{"ref": "main"}'
```

**Response** `200 OK`:

```json
{
  "triggered": true,
  "skill": "deploy-production",
  "body_length": 16
}
```

**Errors:** `404` if no skill has a matching `webhook_trigger`.


### Memory Endpoints

The OpenMemory cognitive memory engine provides persistent, queryable memory across two storage layers: the cognitive store (5-sector vector graph) and the verbatim drawer store (lossless 800-char chunks).

All memory endpoints require authentication (`Authorization: Bearer $VIBECLI_TOKEN`).

#### Cognitive store

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/memory/add` | Add a memory entry (sector auto-classified) |
| `POST` | `/memory/query` | Semantic search with composite scoring |
| `GET` | `/memory/list` | List all memories (supports `?sector=` and `?limit=` params) |
| `GET` | `/memory/stats` | Counts by sector, storage size, encryption status, drawer count |
| `POST` | `/memory/fact` | Add a temporal fact (auto-closes previous same-key fact) |
| `GET` | `/memory/facts` | List active and closed facts |
| `POST` | `/memory/decay` | Run exponential salience decay |
| `POST` | `/memory/consolidate` | Sleep-cycle consolidation — merge weak memories, generate reflections |
| `GET` | `/memory/export` | Export all memories as JSON |
| `POST` | `/memory/import` | Import memories from mem0 / Zep / native JSON |
| `POST` | `/memory/pin` | Pin a memory by ID (exempt from decay and purge) |
| `POST` | `/memory/unpin` | Remove the pin flag from a memory |
| `POST` | `/memory/delete` | Delete a memory permanently by ID |

#### Verbatim drawer layer (MemPalace)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/memory/chunk` | Ingest text as verbatim 800-char chunks |
| `GET`  | `/memory/drawers/stats` | Drawer count, Wing/Room distribution, dedup hit rate |
| `POST` | `/memory/tunnel` | Create a cross-project waypoint between two memories |
| `POST` | `/memory/auto-tunnel` | Auto-detect and create tunnel waypoints across stores |
| `GET`  | `/memory/benchmark` | Run LongMemEval recall@K (supports `?k=` param, default 5) |

#### 4-layer context

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/memory/context` | Get the full 4-layer context block the agent would receive |

```bash
# Add a cognitive memory
curl -X POST http://localhost:7878/memory/add \
  -H "Authorization: Bearer $VIBECLI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"content": "The auth module uses JWT with RS256 signing"}'

# Semantic query
curl -X POST http://localhost:7878/memory/query \
  -H "Authorization: Bearer $VIBECLI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query": "How does authentication work?", "limit": 5}'

# Ingest raw text as verbatim chunks
curl -X POST http://localhost:7878/memory/chunk \
  -H "Authorization: Bearer $VIBECLI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"content": "Runbook step 3: restart payment-worker pods after migration 0047..."}'

# Get 4-layer agent context
curl -X POST http://localhost:7878/memory/context \
  -H "Authorization: Bearer $VIBECLI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query": "deployment process", "l1_tokens": 700, "l2_limit": 8}'

# Run recall benchmark at k=5
curl "http://localhost:7878/memory/benchmark?k=5" \
  -H "Authorization: Bearer $VIBECLI_TOKEN"

# Pin a memory (survives decay and consolidation purge)
curl -X POST http://localhost:7878/memory/pin \
  -H "Authorization: Bearer $VIBECLI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"id": "mem_c2a9"}'

# Remove a pin
curl -X POST http://localhost:7878/memory/unpin \
  -H "Authorization: Bearer $VIBECLI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"id": "mem_c2a9"}'

# Delete a memory permanently
curl -X POST http://localhost:7878/memory/delete \
  -H "Authorization: Bearer $VIBECLI_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"id": "mem_d1f6"}'
```

**`/memory/pin`, `/memory/unpin`, `/memory/delete` responses:**

```json
{ "ok": true }
```

All three endpoints return `{"ok": false, "error": "memory not found"}` when the `id` does not match any stored memory.

**`/memory/stats` response:**

```json
{
  "total_memories": 47,
  "total_waypoints": 12,
  "total_facts": 9,
  "total_drawers": 132,
  "encryption": false,
  "sectors": [
    { "sector": "Semantic",   "count": 18, "avg_salience": 0.82, "pinned_count": 3 },
    { "sector": "Episodic",   "count": 14, "avg_salience": 0.61, "pinned_count": 1 },
    { "sector": "Procedural", "count": 11, "avg_salience": 0.75, "pinned_count": 2 },
    { "sector": "Reflective", "count":  3, "avg_salience": 0.90, "pinned_count": 3 },
    { "sector": "Emotional",  "count":  1, "avg_salience": 0.45, "pinned_count": 0 }
  ],
  "embedding_dim": 512,
  "embedding_compression_ratio": 10.7,
  "embedding_backend": "turboquant"
}
```

The `embedding_*` fields describe the in-process vector index. `embedding_backend` is currently always `"turboquant"` (~3 bits/dim compressed); clients should treat the field as opaque so future backends (e.g. `"hnsw_f32"`, `"candle_bert"`) can be added without breaking parsers.

**`/memory/benchmark` response:**

```json
{
  "k": 5,
  "total_memories": 47,
  "total_drawers": 132,
  "probes": 20,
  "hits_cognitive": 15,
  "hits_verbatim": 18,
  "recall_cognitive": 0.75,
  "recall_verbatim": 0.90,
  "recall_combined": 0.975,
  "cases": [
    { "sector": "episodic",   "query": "What was the last project I worked on?", "found_cognitive": true,  "found_verbatim": true  },
    { "sector": "preference", "query": "What coding style does the user prefer?", "found_cognitive": false, "found_verbatim": true  }
  ]
}
```

### Tauri Commands (VibeUI)

The following Tauri commands are available for the VibeUI frontend via `invoke()`. All commands are registered in `vibeui/src-tauri/src/lib.rs`.

#### Memory commands

| Command | Arguments | Returns |
|---------|-----------|---------|
| `openmemory_stats` | — | `{ total_memories, total_waypoints, total_facts, total_drawers, sectors[] }` |
| `openmemory_add` | `content: string, tags?: string[]` | `{ id, sector, tags, weight, created_at }` |
| `openmemory_query` | `query: string, limit?: number, sector?: string` | `QueryResult[]` |
| `openmemory_list` | `offset?: number, limit?: number, sector?: string` | `Memory[]` |
| `openmemory_facts` | — | `TemporalFact[]` |
| `openmemory_add_fact` | `subject, predicate, object: string` | `TemporalFact` |
| `openmemory_decay` | — | `{ decayed: number, remaining: number }` |
| `openmemory_consolidate` | — | `{ merged: number, reflections_created: number }` |
| `openmemory_export` | — | `string` (markdown) |
| `openmemory_enable_encryption` | `key?: string` | `{ enabled: boolean }` |
| `openmemory_pin` | `id: string` | `{ ok: boolean }` |
| `openmemory_unpin` | `id: string` | `{ ok: boolean }` |
| `openmemory_delete` | `id: string` | `{ ok: boolean }` |

#### Verbatim drawer commands

| Command | Arguments | Returns |
|---------|-----------|---------|
| `openmemory_drawer_stats` | — | `{ total_drawers, wings[], rooms[] }` |
| `openmemory_layered_context` | `query: string, l1_tokens?: number, l2_limit?: number` | `{ l1_essential_story, l2_scoped[], l3_drawers[], total_drawers }` |
| `openmemory_benchmark` | `k?: number` | `{ k, recall_cognitive, recall_verbatim, recall_combined, cases[], … }` |

```typescript
// Example: run benchmark and display results
const result = await invoke<BenchmarkResult>('openmemory_benchmark', { k: 5 });
console.log(`Combined Recall@5: ${(result.recall_combined * 100).toFixed(1)}%`);

// Example: get layered context for a query
const ctx = await invoke('openmemory_layered_context', {
  query: 'deployment process',
  l1Tokens: 700,
  l2Limit: 8,
});
```


### GET /pair

Generate a one-time device pairing URL. No authentication required.

```bash
curl http://localhost:7878/pair
```

**Response** `200 OK`:

```json
{
  "url": "http://localhost:7878/pair?token=...",
  "token": "abc123...",
  "instructions": "Open this URL in your device's browser to pair with this VibeCLI instance."
}
```
