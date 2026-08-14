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

Liveness **and identity** check. No authentication required.

**Response** `200 OK` (abridged — the live response also reports provider,
graph, skillforge and token-freshness status):

```json
{
  "status": "ok",
  "service": "vibecli",
  "version": "0.5.8"
}
```

```bash
curl http://localhost:7878/health
```

The full document also carries `graph`, `skillforge`, `embedding`,
`kv_cache_codec_probe` and `api_token` blocks — see
[Embeddings → `GET /health`](#get-health--embedding) for the embedding one.
`service` is the identity check every client's daemon-autostart requires: a
process answering on 7878 that reports a different `service` is not the daemon.

> **Clients must check `service`, not just the status code.** A 200 from this
> port only proves *something* is listening; any local service could answer.
> `service: "vibecli"` is the contract that distinguishes "the daemon is here"
> from "the port is taken by another program" — and those two need very
> different messages in the UI. The autostart path
> (`vibecli_cli::daemon_bootstrap::probe`) requires an exact match, and so
> should every client health check.

The daemon port defaults to `7878` and is overridable with
`VIBECLI_DAEMON_PORT` (the legacy `VIBEDESK_DAEMON_PORT` is still honoured).


### POST /chat

Single-turn chat completion (non-streaming). Collects the full response before returning.

**Request body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `messages` | `ChatMessage[]` | Yes | Conversation history |
| `provider` | `string` | No | Provider to answer with (e.g. `"anthropic"`, `"ollama"`) |
| `model` | `string` | No | Model to answer with |

`provider` and `model` are honoured **only together** — both non-empty, matching
`POST /agent`. Either one alone, an unknown provider, or a missing API key falls
back to the daemon's configured provider rather than failing the turn. Applies to
`/chat` and `/chat/stream` alike.

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
| `max_step_extensions` | `number` | No | How many times a run that is **still making progress** may extend its step budget past `max_steps`. Ceiling is `max_steps × (1 + this)`. Omit for the harness default (`3`); `0` restores a hard `max_steps` wall |

> **Approval over HTTP.** Only `"full-auto"` and `"auto-edit"` let a run finish
> unattended. There is no interactive tool-approval channel on this route, so a
> tool gated by `"suggest"` is rejected with a `system` event explaining why —
> the run continues and adapts rather than hanging.

> **Step budget.** `max_steps` (50) is a runaway guard. A run that exhausts it
> while still landing successful tool calls, with a healthy circuit breaker, is
> granted more runway instead of stopping mid-plan; a stalled or spinning run is
> not. Raise `max_step_extensions` for long tasks, lower it to cap cost.

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
| `type` | `string` | Always. One of: `chunk`, `step`, `system`, `retry`, `complete`, `partial`, `error` |
| `content` | `string` | `chunk`, `system`, `retry`, `complete`, `partial`, `error` |
| `step_num` | `number` | `step` |
| `tool_name` | `string` | `step` |
| `success` | `boolean` | `step` |
| `steps_completed` | `number` | `partial` |
| `steps_planned` | `number` | `partial` |
| `remaining_plan` | `string[]` | `partial` |
| `attempt` | `number` | `retry` (0-based) |
| `max_attempts` | `number` | `retry` |
| `backoff_ms` | `number` | `retry` |

**Event types:**

| Type | Terminal | Description |
|------|----------|-------------|
| `chunk` | no | Incremental text from the LLM |
| `step` | no | A tool was executed (e.g., `read_file`, `bash`) |
| `system` | no | Daemon advisory that isn't model output |
| `retry` | no | Transient provider error; backing off before another attempt |
| `complete` | **yes** | Agent finished the task. `content` has the summary |
| `partial` | **yes** | Agent stopped with planned work outstanding |
| `error` | **yes** | Agent failed. `content` has the error message |

The stream closes after a terminal event. **Break your read loop on `complete`,
`partial` *or* `error`** — treating only `complete` as success is wrong:
`partial` means the agent stopped before finishing everything it planned, and
`remaining_plan` lists exactly what it never executed. The corresponding job
record ends in status `partial` (terminal, but neither `complete` nor
`failed`), and the run is resumable from its checkpoint.

Unknown `type` values should be ignored rather than treated as errors — new
non-terminal kinds may be added.

**A partial run:**

```
data: {"type":"step","step_num":1,"tool_name":"read_file","success":true}

data: {"type":"partial","content":"Refactored the parser.\n\nRemaining (2 of 3 steps not done):\n  2. update the call sites\n  3. run the test suite","steps_completed":1,"steps_planned":3,"remaining_plan":["update the call sites","run the test suite"]}
```

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
| `status` | `string` | `"queued"`, `"running"`, `"complete"`, `"partial"`, `"failed"`, `"cancelled"`. `"partial"` is terminal but means the agent stopped with planned work outstanding — resumable, and not a success |
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


### POST /voice/transcribe

Speech to text. The single transcription surface for every client — panels,
mobile, watch, the editor plugins and the SDK all go through this rather than
each calling a speech provider directly.

The daemon picks the engine: a downloaded local whisper model when
`voice.prefer_local` is set or no cloud key is configured, Groq's
`whisper-large-v3` otherwise, with fallback between them. The response says
which one ran.

Two body forms are accepted:

| `Content-Type` | Body |
|---|---|
| `application/json` | `{"audio_base64": "…", "mime_type": "audio/webm", "language": "en", "prefer_local": false}` |
| an audio type | the raw audio bytes; hints go in `X-Voice-Language` / `X-Voice-Prefer-Local` |

Supported audio types: `audio/webm`, `audio/wav`, `audio/mp4` (m4a),
`audio/mpeg`, `audio/ogg`, `audio/flac`, `audio/aac`. Anything else is a `415`
rather than a guess — a wrong extension surfaces later as an opaque ffmpeg
error.

**Body limit:** 16 MB, overriding the daemon-wide 1 MB cap (~5 minutes of
16 kHz mono WAV).

**Response** `200 OK`:

```json
{ "text": "add a test for the parser", "engine": "local_whisper" }
```

`engine` is `local_whisper` (audio never left the machine) or `cloud_whisper`
(audio was uploaded to Groq).

```bash
# Raw bytes — the easiest form from a shell or a non-JS client.
curl -X POST http://localhost:7878/voice/transcribe \
  -H "Authorization: Bearer $VIBECLI_TOKEN" \
  -H "Content-Type: audio/wav" \
  --data-binary @clip.wav
```

**Errors:** `400` empty or malformed body · `415` unsupported audio type ·
`503` no engine available. The `503` body carries setup guidance ("run
`/voice download base`", "set `GROQ_API_KEY`") — surface it to the user
verbatim rather than reporting the status code.

### GET /voice/status

What the voice stack can do on this machine. Call it once to decide whether to
offer a mic button, and to explain a disabled one.

**Response** `200 OK`:

```json
{
  "cloud_stt_configured": false,
  "cloud_tts_configured": false,
  "local_model": "base",
  "local_model_size_mb": 142,
  "local_model_downloaded": true,
  "prefer_local": true,
  "language": "en",
  "whisper_cpp_installed": true,
  "whisper_python_installed": false,
  "sox_installed": true,
  "ffmpeg_installed": false,
  "can_transcribe": true,
  "upload_limit_bytes": 16777216
}
```

`can_transcribe` is the field to branch on: a downloaded model with no runtime
to execute it is not a usable engine, and this accounts for that. Results are
cached for 60 s because the probe shells out to `whisper-cli`, `whisper`, `sox`
and `ffmpeg`.

`ffmpeg_installed` matters only for *local* transcription of non-WAV audio.
Browser clients record WebM, so on a machine with a local model but no ffmpeg
their recordings can only be transcribed in the cloud — the `503` says so
explicitly rather than blaming the Whisper runtime.

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

### Tauri Commands (VibeCoder)

The following Tauri commands are available for the VibeCoder frontend via `invoke()`. All commands are registered in `vibecoder/src-tauri/src/lib.rs`.

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

#### Plugin commands

Back the Plugin Governance panel. Workspace plugins — signed bundles under
`<workspace>/.vibecli/plugins/`, distinct from the user-level plugins in
[Plugin Development](/plugin-development/).

| Command | Arguments | Returns |
|---------|-----------|---------|
| `plugin_list_installed` | `workspacePath: string` | `InstalledPlugin[]` |
| `plugin_catalog_list` | `workspacePath: string` | `CatalogPlugin[]` — every core plugin with `installed` and `policy` for this workspace |
| `plugin_install_from_catalog` | `workspacePath: string, name: string, force: boolean` | `InstalledPlugin` + `signing_key_persisted` |
| `plugin_install_from_file` | `workspacePath: string, bundlePath: string, force: boolean` | `InstalledPlugin` |
| `plugin_install_from_url` | `workspacePath: string, url: string, force: boolean` | `InstalledPlugin` |
| `plugin_set_policy` | `workspacePath, name, policy, isAdmin` | `{ ok }` |
| `plugin_uninstall` | `workspacePath, name, isAdmin` | `boolean` |

VibeCoder's Connectors panel has no Tauri wrapper — it calls
`/api/vibedesk/connectors*` through `daemonFetch` directly. The five
`connectors_*` commands it used to call were removed: they kept connectors in a
process-local `Vec`, recorded every one as `connected` without a credential, and
answered `connectors_test` from whether the row existed.

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

---

## Embeddings — `/embeddings/*`, `/index/*`

Semantic search / RAG. Full guide: [embeddings.md]({{ site.baseurl }}/embeddings/).

### `GET /embeddings/models`

Every embedding provider, its models, availability, the selected model, and the
embedding models pulled into the local Ollama.

`providers[].availability.state` is `ready` | `needs_api_key` |
`not_compiled_in`. `providers[].is_local` says whether embedding leaves the
machine — relevant before indexing a private repo with a cloud model.

`ollama_installed.status` is `ok` or `unreachable`; the latter is distinct from
an empty model list, which would read as "nothing installed".

### `POST /embeddings/embed`

```json
{ "texts": ["fn main() {}"], "kind": "document", "provider": "voyage", "model": "voyage-code-3" }
```

`kind` is `"document"` (default) or `"query"`. Asymmetric models place stored
passages and search queries in different regions of the space; embedding a
query as a document does not error, it just costs recall. `provider`/`model`
override the configured selection.

Response `dimension` is the length actually returned, not a catalog value.

### `GET /index/status`

```json
{
  "selected": { "provider": "ollama", "model": "nomic-embed-text" },
  "description": "ollama/nomic-embed-text (768d, local)",
  "built": true,
  "current": { "format_version": 2, "chunk_count": 4180, "file_count": 611, "dimension": 768 },
  "available": [ /* every index on disk, including other models */ ]
}
```

`built` refers to the **selected** model. `available` lists every per-model
index present — each one switchable to without re-embedding.

### `POST /index/build`

```json
{ "provider": "voyage", "model": "voyage-code-3" }
```

Both fields optional; omit to use the configured model. Responds when the index
is **written**, not when the job starts — embedding a workspace takes real time
and, on a paid provider, real money.

### `GET /health` → `embedding`

```json
"embedding": {
  "status": "indexed",
  "provider": "ollama",
  "model": "nomic-embed-text",
  "dimensions": 768,
  "local": true,
  "chunks": 4180,
  "files": 611,
  "other_indexes": ["voyage/voyage-code-3"]
}
```

`status` is `indexed` | `not_indexed` | `misconfigured`. Never includes the key.

---

## Goals — `/v1/goals/*`

Durable execution-intent primitive. See [design/goal/README.md](https://github.com/TuringWorks/vibecody/blob/main/docs/design/goal/README.md) for the full data model + cross-client surface table.

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/v1/goals` | Create. Body: `{ title, statement?, workspace?, success_criteria?, tags?, parent_goal_id? }`. Returns 201 + `Goal`. 409 on `(workspace, title)` conflict. |
| `GET` | `/v1/goals` | List. Query: `status`, `workspace`, `tag`, `limit` (default 50). Returns `{ goals, count }`. |
| `GET` | `/v1/goals/:id` | Detail. Returns `{ goal, links }`. |
| `PATCH` | `/v1/goals/:id` | Partial update. `workspace` and `parent_goal_id` use double-`Option` semantics (omit / `null` / value). Editing `statement` or `success_criteria` auto-clears `current_plan`. |
| `DELETE` | `/v1/goals/:id` | Hard delete; links cascade. |
| `POST` | `/v1/goals/:id/plan` | Generate `ExecutionPlan` via `PlannerAgent`. Body: `{ provider?, model? }`. Per-request override honored when both are present and the API key resolves (env or `profile_settings.db`); otherwise falls back to the daemon's configured provider. Response carries `plan_provider_override_applied`, `plan_provider_requested`, `plan_model_requested`. |
| `POST` | `/v1/goals/:id/link` | Attach a session / job / recap / note. Body: `{ kind, target_id, note? }`. |
| `POST` | `/v1/goals/:id/start` | Spawn a session bound to this goal. Body: `{ task?, provider?, model? }`. Returns `{ session_id, link_id, goal_id }`. |
| `POST` | `/v1/goals/:id/recap` | Cross-store aggregate recap. Body: `{ provider?, model? }`. When both fields are supplied and the named provider is reachable, the daemon synthesizes the headline + bullets via LLM and sets `recap_synthesizer: "llm"`. Otherwise the heuristic fold runs and `recap_synthesizer: "heuristic"` is returned. Per-target recaps are still collected via two-phase store split. |
| `GET` | `/v1/goals/:id/children` | One-level tree query. Returns `{ parent_goal_id, children, count }`. Walk iteratively for a full tree. |
| `GET` | `/v1/goals/:id/tree` | Recursive subtree walk. Query: `depth` (default 3, clamped to 1..10). Returns `{ root, depth, tree: { goal, children, [truncated, direct_child_count, cycle] } }`. Re-visited nodes set `cycle: true` so clients don't recurse. |
| `GET` | `/v1/goals/current` | Look up the pinned goal. Query: `workspace?` (empty / absent = global slot). Returns `{ workspace, goal_id, pinned_at, goal }` or `{ workspace, goal_id: null }`. |
| `PUT` | `/v1/goals/current` | Pin or replace the current goal. Body: `{ goal_id, workspace? }`. 404 if `goal_id` is unknown. |
| `DELETE` | `/v1/goals/current` | Clear the pin. Query: `workspace?`. Returns `{ workspace, removed }`. |

### Watch (curated proxies)

The Apple Watch / Wear OS never hits `/v1/*` directly. Use the curated read-only `/watch/goals` pair instead.

| Method | Path | Notes |
|---|---|---|
| `GET` | `/watch/goals` | Active goals only, ≤25, slim payload (`{ id, title, status, workspace_label, updated_at, pinned }`). `pinned` is `true` when the row is the workspace-specific OR global current pin (G11.2). Older daemons that lack the field decode cleanly on the watch side. |
| `GET` | `/watch/goals/:id` | Envelope `{ goal, links, pinned }` (G12.1 added `pinned: bool` at the envelope level so the watch detail / tile can render the ★ without a separate `/v1/goals/current` lookup; watch never hits `/v1/*`). |
| `POST` | `/watch/goals/:id/start` | Curated wrapper for `do_v1_exec_goal_start`. Body: `{ task? }`. Returns `{ session_id, link_id, goal_id }`. |

## Plugins & connectors — `/api/vibedesk/plugins/*`, `/api/vibedesk/connectors/*`

Back the Plugins panel: what is extending the agent in a workspace, what can be
installed, and which MCP servers this machine is connected to. All require the
bearer token. `path` (query for `GET`, body field for `POST`) scopes to a
project, like the other `/api/vibedesk/*` routes; omitted, it uses the daemon's
workspace root.

Plugins here are the **workspace** plugin system — signed bundles under
`<workspace>/.vibecli/plugins/`, policed per workspace. That is a different
system from the user-level `~/.vibecli/plugins/` bundles described in
[Plugin Development](/plugin-development/); the two do not share a manifest
format or an install directory.

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/api/vibedesk/plugins` | Components live in this workspace, grouped by kind. Read-only inventory. |
| `GET` | `/api/vibedesk/plugins/catalog` | Every core plugin compiled into the daemon, with `installed` and `policy` for this workspace. |
| `POST` | `/api/vibedesk/plugins/install` | Body: `{ name, path?, force? }`. Materialises the catalog entry, signs it, and installs it through the same verified path as a downloaded bundle. Returns `signing_key_persisted` — `false` means the publisher fingerprint will differ next install. |
| `POST` | `/api/vibedesk/plugins/policy` | Body: `{ name, policy, path? }`. `policy` is `"on"` or `"off"`. `"required"` is refused: it is an admin pin the same user could not then lower. |
| `POST` | `/api/vibedesk/plugins/uninstall` | Body: `{ name, path? }`. Removes the install directory and the policy row. `removed: false` means there was nothing on disk — the policy row may still have been cleared. |

### Core plugin catalog

Compiled into the binary, so there is no registry to reach and nothing to
download. Each ships skills and rules only — Markdown that `skill_catalog` and
`context_assembler` already load. Hooks and MCP-server components are not
offered here: a hook is an executable whose exec bit does not survive the bundle
round-trip, and plugin MCP servers are registered only by a module nothing
currently calls.

| Plugin | Ships |
|---|---|
| `core-review-standards` | Rule: what a review must check before approving. |
| `core-secure-defaults` | Rules: secret handling, and bounding untrusted input. |
| `core-commit-craft` | Skill: writing commit messages that say why. |
| `core-test-first` | Skill: pinning behaviour before changing it, and spotting a vacuous test. |

The signature on a catalog install is real and verified, but it attests
integrity, not provenance: the manifest is signed on this machine with a locally
generated P-256 key. The embedded publisher key in a third-party bundle carries
exactly the same weight — that is this format's trust model.

### Connectors

A connector is an MCP server definition plus the credentials it needs.
Definitions live in the workspace store; credentials live encrypted in
`workspace_secrets` and are never returned by any route.

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/api/vibedesk/connectors` | `{ connectors, catalog }`. Each configured connector reports `missing_credentials`; each catalog entry reports `runtime_available` (is `npx` / `uvx` on PATH). Neither carries a health field. |
| `POST` | `/api/vibedesk/connectors` | Body: `{ catalog_id, credentials?, path? }` for a catalog entry, or `{ id, title?, command, args?, credentials?, path? }` for a hand-entered server. A required credential left blank is refused. |
| `POST` | `/api/vibedesk/connectors/toggle` | Body: `{ id, enabled, path? }`. Disabled connectors are not launchable; their credentials are kept. |
| `POST` | `/api/vibedesk/connectors/remove` | Body: `{ id, path? }`. Returns `secrets_deleted` so the caller can say whether the credentials went too. |
| `POST` | `/api/vibedesk/connectors/probe` | Body: `{ id, path? }`. **Actually launches the server** and lists its tools. Returns `{ result: { state: "ok", tools } \| { state: "failed", error } \| { state: "timedout", after_secs } , checked_at }`. Bounded at 45s. |

`probe` is the only route that reports a connector as working, because it is the
only one that runs it. Nothing infers "connected" from the presence of a key,
and no probe result is persisted — a stored `ok` is a claim about the past
presented as a claim about now.

Enabled connectors are merged into `vibecli`'s `/mcp` command alongside
`[[mcp_servers]]` from `config.toml`, with config.toml winning a name collision.
Agent runs do not consume MCP tools yet, so a connector makes tools reachable
from the CLI, not from an agent turn.

| Connector | Runtime | Credential |
|---|---|---|
| `vibecli` | none — this binary, via `--mcp-server` | — |
| `filesystem` | `npx` | — |
| `git` | `uvx` | — |
| `fetch` | `uvx` | — |
| `memory` | `npx` | — |
| `github` | `npx` | `GITHUB_PERSONAL_ACCESS_TOKEN` |
