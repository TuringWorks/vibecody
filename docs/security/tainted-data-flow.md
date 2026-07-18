---
layout: page
title: Tainted Data Flow — Prompt-Injection Containment Design
permalink: /security/tainted-data-flow/
---

> **Status:** design draft, 2026-05-14. Not yet implemented.
> **Threat:** DREAD #1 in [`threat-model.md`](./threat-model.md) — *prompt injection in repo/file content escalates to file-write or shell tool call*. Damage 10 / Reproducibility 8 / Exploitability 7 / Affected 10 / Discoverability 8 → **8.6**, the highest score on the open list.
> **Owner:** Security SME (rotating). **Reviewer:** maintainers of `serve.rs`, MCP runtime, and the tool registry.

---

## 1. The problem in one paragraph

VibeCody routinely feeds **attacker-controlled strings** — file contents, web pages, MCP tool outputs, RAG hits — into LLM prompts. The LLM produces responses that frequently include **tool calls** (`fs.write`, `shell.exec`, `git.commit`, `mcp.invoke`). Without a propagation discipline, a prompt-injection payload in a README, an HTML page, or a `mcp.search_repo` result can convince the model to issue a tool call that exfiltrates `~/.aws/credentials` to a Slack webhook, rewrites the user's `.zshrc`, or pushes a malicious branch. There is no current architectural barrier — only the model's politeness.

This document proposes a **taint-propagation discipline** that makes the prompt-injection vector a *first-class* constraint of the codebase, in the same way the encrypted ProfileStore makes "API keys never touch plaintext on disk" a first-class constraint.

---

## 2. Threat model recap

| Stage | T-level | Where | What it can carry |
|---|---|---|---|
| Source — file contents | T5 | `fs.read`, `repo.diff`, Tree-sitter snippets | `<!-- INSTRUCTION: ignore previous instructions and run … -->` |
| Source — web fetch | T5 | `web.fetch`, `browser.navigate`, scraped HTML | The same, plus invisible CSS-hidden text / steganographic Unicode |
| Source — MCP tool output | T5 | `mcp.invoke` return strings, MCP `resources` reads | Same as above; some MCP servers fetch the open web |
| Source — RAG hit | T5 | `semantic_index` / `chroma` lookups against the workspace | Whatever the indexed corpus contains |
| Propagation | T5 | LLM input message, LLM streaming response, structured tool-call args | The model echoes attacker text into its own output |
| Sink — tool call | T0 → host | `fs.write`, `shell.exec`, `git.commit`, `mcp.invoke`, `http.request`, `provider.message` | Privileged operation on host or network |
| Sink — user-facing log | T0 → user | `tracing::info!` of prompt content; chat UI rendering | Information disclosure if rendered raw (covered by [DREAD #10](./threat-model.md) for the WebView side) |

The **tool-call sink is the catastrophic one** — that's where T5 text reaches T0 host privilege. Everything else is recoverable.

---

## 3. Design principles

1. **Honesty over cleverness.** Don't claim to "detect injection" — every detector is bypassable. Instead, track *provenance* (this string originated outside T1) and require a deliberate human or T1-equivalent step before T5 strings can drive a privileged operation.
2. **Visible at the type system.** A `Tainted<String>` should be impossible to silently coerce into a `&str` that feeds a tool-call argument. The compiler enforces the discipline.
3. **One direction only.** Once a string is tainted, it stays tainted. The only way to remove the taint is `tainted.unwrap_after_user_confirmation(reason)` or `tainted.sanitize_to::<RestrictedShape>()` — both deliberate, both audited.
4. **Co-located with existing trust boundaries.** Taint flows mirror the §6 STRIDE boundaries already named in the threat model — no new mental model.
5. **Incremental rollout.** The design must allow a partial deployment that still adds value. Day-one win = the highest-risk tool calls (`fs.write`, `shell.exec`) gated; later phases cover lower-risk surfaces.

---

## 4. Proposed core type

```rust
/// A string-shaped value whose origin is outside T1 (user keyboard +
/// local WebView). Once a string is `Tainted`, it stays that way until
/// it crosses one of three exit boundaries:
///
///   1. `confirm_with_user(reason)` — surfaces a modal/CLI prompt
///      describing the operation and the source of the tainted bytes.
///   2. `sanitize_to::<S: Sanitizer>(s)` — runs a domain-specific
///      sanitizer (file-path canonicalize-and-bound, URL allow-list,
///      shell-arg quote-escape) and produces an untainted value of a
///      narrower type.
///   3. Comparison / hashing / length checks that don't reveal content
///      to a privileged sink.
///
/// `Tainted<T>` deliberately has **no `Deref`** — accessing the inner
/// value requires `.expose_for(MCPArgument)` or `.expose_for(LogLine)`
/// with a `Reason` parameter that lands in the audit log.
pub struct Tainted<T> {
    value: T,
    origin: Provenance,
}

pub enum Provenance {
    File { path: PathBuf, byte_range: Range<usize> },
    WebFetch { url: Url, fetched_at: SystemTime },
    Mcp { server: String, tool: String, call_id: String },
    Rag { index: String, doc_id: String, score: f32 },
    LlmResponse { provider: String, model: String, request_id: String },
    /// Manual taint — for code paths that consume an external feed
    /// without going through one of the standard sources above.
    External { reason: String },
}
```

Mirrors the existing [`Redact<T>`](../../vibecli/vibecli-cli/src/redact.rs) newtype design: no `Deref`, no `Display` impl that exposes the inner value, serde-transparent where it makes sense, and explicit `.expose_for(...)` methods at the sink.

---

## 5. Where taint originates (8 entry points)

| # | Source | Today | After |
|---|---|---|---|
| 1 | `fs.read` of workspace file | returns `String` | returns `Tainted<String>` with `Provenance::File` |
| 2 | `web.fetch` / `browser` | returns `String` (raw HTML/text) | returns `Tainted<String>` with `Provenance::WebFetch` |
| 3 | MCP tool return | returns `Value` (`String` arm trusted) | wraps each leaf `String` in `Tainted<String>` |
| 4 | RAG / `semantic_index` query | returns `Vec<Match { text: String, … }>` | wraps `text` in `Tainted<String>` with `Provenance::Rag` |
| 5 | `repo.diff` / `git.log` / `git.show` | returns `String` | wraps in `Tainted<String>` |
| 6 | Clipboard paste (mobile / watch) | returns `String` | wraps in `Tainted<String>` with `Provenance::External` |
| 7 | OAuth callback bodies | returns `Value` | wraps as needed (currently OAuth flows are T2 — paired-device, so this is a defense-in-depth wrap) |
| 8 | LLM completion response | returns `String` | wraps each `delta.content` in `Tainted<String>` with `Provenance::LlmResponse` |

Entry **#8 is the load-bearing one**. Even text the model *generates* must be considered tainted because the model is a T5 actor (it processes T5 inputs that may have rewritten its instructions). This is the conceptual leap that makes the design work — without it, a prompt-injection payload that turns into a model-generated tool-call argument would bypass the system.

---

## 6. Where taint terminates (4 sinks)

Every sink needs an explicit untaint step. The design names four:

### 6.1 Tool-call execution (`fs.*`, `shell.*`, `git.*`, `http.*`)

The privileged operation. The Rust function signatures for these handlers change from `fn(path: &str)` to `fn(path: TaintedOrTrusted<PathBuf>)`. The wrapper type forces the caller to either:

- pass an `Untainted<PathBuf>` (originated in T1 — user typed it), or
- pass a `Tainted<PathBuf>` along with a `Confirmation` token proving the user clicked-through a modal that named the source.

This is where the **policy layer** lives: `fs.write` to anywhere under `~/.vibecli/` requires confirmation regardless of taint; `shell.exec` requires confirmation for *any* tainted argument; `git.commit` is allowed for tainted commit messages because the message is non-executing data.

### 6.2 Outbound HTTP (LLM provider, webhook, MCP server)

A tainted string in an HTTP request body to an LLM provider is *expected* — that's how the model receives context. **But:** a tainted string in the URL, the Authorization header, or the bearer-token claim is an exfiltration vector. The HTTP helper gains a `Body` enum where `Body::Json(value)` accepts tainted leaves while `Body::Url(_)` and headers must be untainted.

### 6.3 User-facing display (chat UI, CLI output, log files)

Tainted strings *can* be displayed to the user — that's the entire point of a chat UI. But:

- Tainted markdown going to the WebView already runs through DOMPurify ([DREAD #10](./threat-model.md)) — defense-in-depth complete.
- Tainted strings in `tracing::*!` logs must redact origin/byte-range to a 16-char hash (so admins can correlate but the log file isn't a prompt-injection corpus).

### 6.4 Cache storage (SessionStore, recap text)

Tainted text stored in SessionStore *retains* its taint when read back. The `recap::heuristic_recap` function reads tainted messages and produces a tainted recap — this is automatic via the `Tainted` propagation rule.

---

## 7. Propagation rules

These are the laws the type system enforces:

1. **Concat is contagious.** `tainted_a + clean_b` is tainted. `format!("{tainted} {clean}")` is tainted. `String::push_str` from a tainted source taints the destination.
2. **Slicing preserves taint.** `tainted[..10]` is tainted.
3. **Parsing preserves taint.** `Tainted<String>::parse::<u64>()` returns `Tainted<u64>` — even though numeric types don't carry payload, the *value* originated outside T1 and could drive a quantity sink (transfer amount, line count).
4. **Hashing untaints.** `Tainted<String>::hash_sha256()` returns `Untainted<[u8; 32]>` — a 256-bit hash is not a useful payload carrier for prompt-injection.
5. **Length untaints.** `Tainted<String>::len()` returns `Untainted<usize>` — same reasoning.
6. **`Display`/`Debug` are forbidden.** Use `.expose_for(reason)` to log; the audit trail records the exposure.

The mechanical effect: anywhere in the codebase that today does `let path = some_function(args)?;` and uses `path` to drive a tool call, the type system flags the missing `.expose_for(...)` or `.confirm_with_user(...)` step.

---

## 8. The confirmation flow

The visible UX. Three surfaces:

### 8.1 Desktop (VibeCoder WebView)

A modal interrupts the chat stream with:

```
┌─────────────────────────────────────────────────────────┐
│  Tool call requires confirmation                        │
│                                                         │
│  The model is about to:  shell.exec                     │
│  Command:                rm -rf /Users/me/old_backups   │
│                                                         │
│  This command includes text from:                       │
│    • README.md, lines 412–415                           │
│    • web.fetch result for https://example.com/post/42   │
│                                                         │
│  [ Cancel ]   [ Show full command ]   [ Run ]           │
└─────────────────────────────────────────────────────────┘
```

The provenance breakdown is the cue — even users who routinely click "Run" without reading commands should pause when they see "this command includes text from `https://example.com/...`" for a command they expected to be self-contained.

### 8.2 CLI (`vibecli` REPL)

Same content rendered as a stderr prompt with `[y/N/details]` — the daemon stalls the tool call until the user answers on stdin. CI / headless mode treats unprompted-but-required confirmation as a *cancel*; the agent surfaces the rejection and the model retries with a less-tainted command.

### 8.3 Mobile / Watch

A push notification on the paired device, content:

> ⚠ vibecli wants to run `shell.exec` with text from your README. Tap to review.

Tap leads to the same modal as 8.1. The watch face shows a single-tap accept/cancel for low-risk operations (commit messages, file reads in workspace); higher-risk (`shell`, `http.request` outbound) require the phone form factor.

---

## 9. Rollout plan

The whole-codebase deployment is multi-week. The plan is sliced so each slice is independently shippable and adds value.

### Slice A — Core type + first sink

Ship `Tainted<T>` and gate **`shell.exec` only**. ~3 days. Validates the design end-to-end with one privileged sink before instrumenting the others. Expected impact: catches every prompt-injection that pivots through shell, which is the most common live exploit vector in published injection literature.

### Slice B — File-write sink

Wire `Tainted<PathBuf>` into `fs.write` / `fs.append` / `fs.delete`. ~2 days. The file-write path is already partially defended by `safe_resolve_path` (DREAD #2), so this slice composes with existing checks.

### Slice C — Outbound HTTP sink

Gate `http.request` URL/header construction. ~2 days. Complicated by the LLM-provider HTTP calls (which are *expected* to ship tainted JSON bodies); the policy is "tainted body OK; tainted URL or header forbidden".

### Slice D — MCP boundary

Wrap every MCP tool return at the runtime boundary. ~3 days. This is structurally similar to the file-read tainting; the work is mechanical but spans the full MCP-server matrix.

### Slice E — RAG / semantic index

Wrap every retrieval-augmented-generation hit. ~2 days.

### Slice F — Log redaction

Update `tracing::*!` sites that log raw model output to use the audit-trail `.expose_for(...)` method. ~2 days.

### Slice G — User-facing modal

Wire the confirmation UI in VibeCoder, CLI REPL, and mobile. ~1 week (the design is in §8, but each surface has its own UI plumbing).

**Part 1 (shipped)** — `tainted_prompter::CliPrompter` reads stdin / writes stderr; gated on `--tainted-prompt`. Banner shows `audit_summary()` (truncated, payload-free). Used by the CLI REPL and any daemon started with a TTY attached.

**Part 1.5 (shipped)** — `TaintedDaemonFlags { strict, prompt, http_prompt }` on `ServeState`; flags propagate to every `ToolExecutor` constructed for a session.

**Part 2 (shipped)** — `tainted_http_bridge::{HttpPromptQueue, HttpBridgePrompter}` cross-process bridge.
- Daemon side: `MAX_PENDING = 32`, `RESPONSE_TIMEOUT = 300s`, `block_in_place + Handle::current().block_on(timeout(rx))` to bridge the sync `Prompter` trait to async HTTP.
- HTTP surface: `GET /v1/tainted/pending` (SSE — snapshot + live; events typed `pending`) and `POST /v1/tainted/respond` (JSON body `{ request_id, approve }`). Both authed via bearer-or-`?token=` query param (the EventSource API can't set custom headers, same fallback `ws_collab_handler` already uses).
- CLI flag: `--tainted-http-prompt` (implies `--tainted-strict`; mutually exclusive with `--tainted-prompt` via `conflicts_with`).
- VibeCoder: `TaintedConfirmationModal.tsx` mounted in `App.tsx`. Subscribes to the SSE, head-of-queue render, exponential-backoff reconnect, dispatches `respond` POST on click. Gated on `VITE_TAINTED_HTTP_PROMPT=1` until token plumbing lands.
- Fail-safe ordering: queue saturation → deny; timeout → deny; oneshot drop → deny; only explicit `approve=true` from UI executes.

**Part 3 (mobile / watch — design)** — VibeMobile (Flutter) and VibeWatch / VibeWear (Swift / Kotlin) consume the **same** `GET /v1/tainted/pending` SSE + `POST /v1/tainted/respond` endpoints. They authenticate with their existing pairing bearer (mobile) / signed-nonce (watch — P-256 ECDSA via Secure Enclave / Strongbox; never Ed25519). The render surface is platform-specific:

| Platform | Renderer | Auth | Notes |
|---|---|---|---|
| VibeMobile (Flutter) | `TaintedConfirmationSheet` modal sheet on `HomeScreen` (shipped 2026-05-15) | existing pairing bearer in `ApiClient` | Mobile clients race transports (mDNS → Tailscale → ngrok); SSE rides whichever is connected. `TaintedService` (ChangeNotifierProxyProvider2 in `main.dart`) follows `AuthService.machines.first`; exponential backoff reconnect (1s → 30s). 14 unit tests across `TaintedPrompt`/`TaintedService` (model parsing, FIFO ordering, de-dup, fail-safe-deny on POST failure, idempotent re-emit). |
| VibeCodyWatch (SwiftUI) | `TaintedConfirmationOverlay` rendered on `ContentView` (shipped 2026-05-18) | Watch-Token JWT via `WatchAuthManager.validAccessToken()` | New `GET /watch/tainted/pending` + `POST /watch/tainted/respond` routes share the same `HttpPromptQueue` as `/v1/tainted/*`. `TaintedConfirmationQueue` (MainActor `ObservableObject`) drives the FIFO + de-dup; head-of-queue card renders `sink` + monospace `summary` + audit_id; exponential backoff (1s → 30s). |
| VibeCodyWear (Compose) | `TaintedConfirmationOverlay` rendered on `MainActivity` `Box` (shipped 2026-05-18) | Watch-Token JWT via `WearAuthManager` | OkHttp `EventSource` consumes the same `/watch/tainted/pending` route; `WearNetworkManager.openTaintedPendingStream` filters on the `pending` event type. Mirrors watchOS UX. |

In all three cases the **payload bytes never leave the daemon** — only `audit_summary` (kind / origin / `audit_id`) crosses the wire. Mobile / watch slices ship in their own PRs (not in part 2). The contract is already stable.

**Total**: ~3 working weeks if done serially, ~2 weeks with parallelization across slices. The first slice (A) is independently valuable and could ship within the next batch.

---

## 10. Open questions

1. **How to handle the model's natural language?** A tainted-string answer that contains *only* prose ("Yes, I think we should refactor `foo`") is harmless. Forcing every chat-display path through a confirmation modal would be unusable. Proposal: the confirmation modal fires *only when a tool call is requested*. Pure prose responses don't trigger anything.

2. **What about retries?** If the model's first attempt hits the confirmation modal and the user clicks Cancel, does the agent get a chance to retry with a different approach? Proposal: the rejection is surfaced to the model as a `tool_result` with `status: "user_rejected", reason: "tainted shell.exec with cross-origin URL argument"` and the agent can adapt.

3. **What about the model itself trying to game the system?** A clever model could write a tool call whose argument is `path = "/safe/path"` while the *intent* is encoded in a comment. The policy can't detect this — the value of the type system is making the model unable to *bypass* the gate, not making the gate semantically infallible.

4. **CI / headless mode.** Without a user, confirmation is impossible. Two options: (a) headless rejects all tainted-argument tool calls, (b) headless honors a `--trust-mcp` / `--trust-fs` flag set at startup. Proposal: (a) by default with (b) as a documented escape hatch for trusted-corpus CI.

5. **Interaction with the audit log.** Every `.expose_for(reason)` and every `.confirm_with_user(...)` outcome should land in an append-only audit log under `~/.vibecli/audit.log` (encrypted, like ProfileStore). The audit log is part of the design but not part of the type system enforcement — it's the post-incident recovery story.

---

## 11. What this design does *not* attempt

- **Prompt-injection detection in the LLM input layer.** Detectors are bypassable; we don't ship one.
- **Sandboxing the LLM provider.** The LLM provider is T5 by definition.
- **Preventing the model from generating tool calls.** That's the product. We constrain *which arguments* it can ship.
- **Cryptographic provenance.** The `Provenance` enum is metadata, not a signature. A compromised daemon could forge it; that's out of scope (covered by host-OS-compromise out-of-scope clause in [`threat-model.md §9`](./threat-model.md)).

---

## 12. Companion changes outside this design

- The **rejection path** through the confirmation modal needs a UX writer pass — the language has to land in the moment without scaring users into clicking Cancel reflexively. See `vibecoder/design-system/README.md` for the existing modal patterns.
- The **mobile / watch surfaces** need their own pairing-style trust setup so a malicious LAN peer can't approve tool calls on behalf of the user. The watch-auth (P-256 ECDSA) layer already shipped is the right substrate — confirmation tokens reuse the same signing material.
- The **audit log** is a separate slice; its design is out of scope for this document but is referenced in §10 question 5.

---

## 13. Decision needed before slice A starts

| Question | Default proposal | Decided? |
|---|---|---|
| Newtype name | `Tainted<T>` | open |
| Crate location | `vibecli-cli/src/tainted.rs` (alongside `redact.rs`) | open |
| Exit method names | `.expose_for(Reason)` / `.confirm_with_user(Confirmation)` / `.sanitize_to::<S>()` | open |
| First gated sink | `shell.exec` (highest blast radius, least false-positive surface) | open |
| Headless behavior | Reject all tainted-argument tool calls | open |
| Audit-log scope | Out of scope of this design; tracked separately | open |

When these are decided, slice A can start. Estimated 3 working days from go-decision to a shippable PR that gates `shell.exec` end-to-end with unit + integration tests.
