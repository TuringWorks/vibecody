# 08 — Cursor SDK Parity Audit

> Compares `@cursor/sdk` (TypeScript, public beta 2026-04-29) against `packages/agent-sdk` and the underlying VibeCLI daemon along seven dimensions: **subagents, hooks, plugins, skills, sandbox tiers, recap/resume, multi-client**. Closes v14 fit-gap entry **B4**.

**Audit date:** 2026-05-10
**Audit type:** Research deliverable (no code changes in this PR). Surfaces roadmap-entry candidates for v15 / Phase 55.
**Methodology:** Two passes — (1) feature-presence table across all seven dimensions, (2) code-shape comparison on the four dimensions where ergonomics gaps are likely to matter (SDK surface, hooks, subagents, recap/resume).

## Sources

- Cursor — Build programmatic agents with the Cursor SDK (blog) — https://cursor.com/blog/typescript-sdk
- Cursor changelog v2.4 — Subagents, Skills, Image Generation — https://cursor.com/changelog/2-4
- Cursor changelog v2.5 — Plugins, Sandbox Access Controls, Async Subagents — https://cursor.com/changelog/2-5
- Cursor SDK public-beta announcement — Apr 29, 2026
- DataCamp Cursor SDK tutorial — https://www.datacamp.com/tutorial/cursor-sdk
- Local: `packages/agent-sdk/src/index.ts` (299 lines, the entire VibeCody SDK)
- Local: `vibecli/vibecli-cli/src/lib.rs` (skill_catalog, async_subagent, plugin_*, sandbox_*, recap, resume modules)

## Cursor SDK surface (verbatim, extracted from sources)

```ts
import { Agent, type SDKMessage, CursorAgentError } from "@cursor/sdk";

// Local runtime
const agent = await Agent.create({
  apiKey: process.env.CURSOR_API_KEY!,
  model: { id: "composer-2" },
  local: { cwd: process.cwd() },
});

// Cloud runtime
const agent = await Agent.create({
  apiKey: process.env.CURSOR_API_KEY!,
  name: "my-agent",
  model: { id: "composer-2" },
  cloud: {
    repos: [{ url: "https://github.com/org/repo", startingRef: "main" }],
    autoCreatePR: true,
  },
  mcpServers: { /* … */ },
  agents:     { /* named subagent defs */ },
});

const run = await agent.send("Refactor auth.ts to use the new session shape");

for await (const evt of run.stream()) {
  // evt.type ∈ "system" | "user" | "assistant" | "tool_call" | "thinking" | "status" | "request" | "task"
}

const result = await run.wait();
// { status, durationMs, result, git?: { branches: [{ prUrl, branch }] } }

// Reconnect to an in-flight run from another process
const run2 = await Agent.getRun(runId, { runtime: "cloud", agentId, apiKey });
await run2.wait();

await agent[Symbol.asyncDispose]();
```

**Project-level config files** (Cursor expects in repo root):

- `.cursor/mcp.json` — MCP server definitions (stdio + HTTP)
- `.cursor/skills/*.md` — auto-loaded skill files (SKILL.md format)
- `.cursor/hooks.json` — agent-loop lifecycle hooks (cloud / self-hosted / local)
- `.cursor/agents/*.md` — named subagent definitions with prompts and per-agent models
- `sandbox.json` — sandbox network/filesystem access policy (Cursor 2.5)

**Runtime modes**: `local` (your machine), `cloud` (Cursor-hosted VM with repo clone + dev env), `self-hosted` (on-prem workers). Same agent script targets all three.

**Sandbox network tiers** (Cursor 2.5): `user-config-only` / `user-config-with-defaults` / `allow-all`. Admin-enforced allowlists/denylists at the org level.

## VibeCody surface (current, as of 2026-05-10)

### `@vibecody/agent-sdk` (299 lines, single file)

```ts
import { VibeCLIAgent, createAgent, AgentError } from '@vibecody/agent-sdk';

const agent = new VibeCLIAgent({
  provider: 'claude',          // 'ollama' | 'claude' | 'openai' | 'gemini' | 'grok'
  approval: 'full-auto',       // 'suggest' | 'auto-edit' | 'full-auto'
  port: 7878,
  host: 'localhost',
});

for await (const evt of agent.run("Add TS strict mode to all files")) {
  // evt.type ∈ "chunk" | "step" | "complete" | "error"
}

await agent.chat([{ role: 'user', content: 'hi' }]);
for await (const tok of agent.chatStream(msgs)) { /* … */ }
await agent.listJobs();
await agent.getJob(sessionId);
await agent.cancelJob(sessionId);
await agent.stop();
await agent.isConnected();
```

**That's the entire public TypeScript surface.** No subagent API, no hooks API, no plugin API, no skills API, no sandbox-tier selector, no recap/resume API, no cloud runtime, no reconnect-to-running, no `[Symbol.asyncDispose]`, no typed error subclasses.

### Daemon modules that exist but are not surfaced through the SDK

These are real modules in `vibecli/vibecli-cli/src/` that the SDK does **not** expose:

| Module | Status | Notes |
|---|---|---|
| `skill_catalog.rs` | shipped (PR #11 / B1) | YAML-frontmatter skill loader, 964 markdown files; MCP `list_skills` / `get_skill` tools |
| `skill_watcher.rs` | shipped (PR #15 / A10) | live hot-reload via notify v7 |
| `async_subagent.rs` | shipped (PR #21 / A5) | state-machine registry; **not yet wired to any HTTP route** |
| `plugin_marketplace.rs`, `plugin_bundle.rs`, `plugin_registry.rs`, `plugin_lifecycle.rs`, `plugin_sdk.rs` | partial | Plugin infra exists; B2 gated behind patent-distance audit (PR #24 §18) |
| `hook_abort.rs`, `jetbrains_hooks.rs`, `webhook.rs` | shipped | Hook abort + JetBrains-specific hooks, plus generic webhook outbound |
| `sandbox_bwrap.rs`, `sandbox_entry.rs`, `sandbox_windows.rs`, `cloud_sandbox.rs`, `opensandbox_client.rs` | shipped + design | 4-tier sandbox design at `docs/design/sandbox-tiers/`; bwrap profile is dead-code-ready |
| `recap.rs`, `resume.rs` | shipped (Phases F2.3 / J1.3 / M1.1) | `/v1/recap` + `/v1/resume` routes live; Flutter mobile recap header lands May 2026 |
| `session_resume_protocol.rs` | shipped (PR #23 / A9) | P-256 signed handoff envelope for cloud-agent resume |
| `acp_stdio.rs` | shipped (PR #16 / A4) | ACP v0.11 server (Zed / JetBrains / Neovim) |
| `mcpb_bundle.rs` | shipped (PR #18 / A2) | MCPB bundle pack/unpack + signature digest |

The daemon is **rich**; the TypeScript surface is **thin**. Every dimension where Cursor's SDK has a richer ergonomic surface, our daemon already has the underlying primitive — the gap is the SDK wrapper, not the protocol.

---

## Pass 1 — Feature-presence table (seven dimensions)

Legend: ✅ shipped in SDK · 🟡 daemon-only (SDK doesn't surface it) · ⚠️ partial · ❌ not present · 🚫 deliberately deferred (patent-distance)

| Dimension | Cursor SDK | VibeCody SDK | VibeCody daemon | Verdict |
|---|---|---|---|---|
| **Subagents** | ✅ `agents: { … }` in config; named subagent defs in `.cursor/agents/*.md`; async subagent spawn from inside agent loop; subagent hierarchies (subagents spawn subagents) | ❌ | 🟡 `async_subagent.rs` registry exists; no HTTP/SDK route, no `.vibecli/agents/*.md` discovery | **GAP D1** — SDK + filesystem-discovery + route wiring |
| **Hooks** | ✅ `.cursor/hooks.json` declarative file; lifecycle events (PreToolUse/PostToolUse/SessionStart/Stop/TaskCompleted-equivalents); cloud + local + self-hosted | ❌ | 🟡 `hook_abort.rs`, `jetbrains_hooks.rs`, `webhook.rs` exist; plus `.claude/settings.json`-style hooks in `CLAUDE.md`; no single `.vibecli/hooks.json` schema, no SDK surface | **GAP D2** — unified declarative hooks config + SDK surface |
| **Plugins** | ✅ Marketplace at `cursor.com/marketplace`; `/add-plugin` editor command; plugins bundle skills+subagents+MCP+hooks+rules; admin install policies (Off / On / Required) | ❌ | ⚠️ `plugin_marketplace.rs` + `plugin_bundle.rs` + `plugin_registry.rs` exist; 🚫 **B2 deferred behind patent-distance audit** per FIT-GAP §18 / PR #24 | **DEFERRED** — B2 already has a shape (federated index, per-publisher P-256 keys, query+category UI). SDK surface follows once shape ships. |
| **Skills** | ✅ `.cursor/skills/*.md` auto-discovery; SKILL.md format; slash-menu invocation; SDK config can pass skills inline | 🟡 711 skill files in `vibecli/vibecli-cli/skills/`; **MCP `list_skills` / `get_skill` shipped in PR #11 (B1)**; no SDK method to list/invoke them | 🟡 `skill_catalog.rs` (PR #11) + `skill_watcher.rs` (PR #15) — feature-complete daemon-side, hot-reload included | **GAP D3** — SDK method to enumerate + invoke skills against an agent run |
| **Sandbox tiers** | ✅ Three network tiers (`user-config-only` / `user-config-with-defaults` / `allow-all`); `sandbox.json` config; admin org-enforced allow/denylists; FS controls | ❌ | ⚠️ `docs/design/sandbox-tiers/` four-backend design (native/broker/Firecracker/Hyperlight); bwrap profile dead-code; egress broker designed but not wired | **GAP D4** — wire sandbox-tier selection through `serve.rs` and surface it on `Agent.create({ sandbox: … })`. Independent from B2/B3 patent posture. |
| **Recap / resume** | ⚠️ `Agent.getRun(runId, { runtime, agentId, apiKey })` reconnect API; durable session state for network drops; no first-class "show me what happened in the last 5 minutes" summary | 🟡 `/v1/recap` + `/v1/resume` daemon routes (Phases F2.3 / J1.3 / M1.1); session resume handoff with P-256 signature (PR #23); no SDK method | ✅ `recap.rs`, `resume.rs`, `session_resume_protocol.rs` — **richer than Cursor's**; we ship narrative recaps, Cursor ships event-stream reconnect | **GAP D5** — wrap `/v1/recap` + `/v1/resume` + `Agent.getRun(...)`-style reconnect in the TypeScript SDK. We have more capability; we just don't expose it. |
| **Multi-client** | ✅ Same agent reachable from desktop / CLI / web (same harness); `local` / `cloud` / `self-hosted` runtime modes selected in SDK config | ❌ SDK only talks to a local daemon (`localhost:7878`) | ✅ daemon serves VibeUI + VibeApp + VibeMobile + VibeCodyWatch + VibeCodyWear + VS Code / JetBrains / Neovim plugins (13 clients per CLAUDE.md product matrix); pairing via P-256 keys | **GAP D6** — SDK runtime selector (`local` / `cloud` / `pairing-url`) mirroring Cursor's three modes; the underlying daemon already speaks to all clients |

**Summary**: SDK-surface gaps **D1–D6** (six entries) become roadmap candidates. None require new daemon protocol work in the next sprint; all of them are TypeScript wrapper work plus, where missing, an HTTP route on the daemon. **D4 (sandbox-tier selector) is the deepest** because it depends on the sandbox-tiers design landing first.

---

## Pass 2 — Code-shape comparison

Four dimensions where ergonomics differences matter most. Verbatim API shapes side-by-side.

### Code-shape A — Agent construction + run

**Cursor:**
```ts
const agent = await Agent.create({
  apiKey, model: { id: "composer-2" }, local: { cwd: process.cwd() },
});
const run = await agent.send("Refactor auth.ts");
for await (const evt of run.stream()) { /* … */ }
const result = await run.wait();   // { status, durationMs, git: { branches: [{ prUrl }] } }
await agent[Symbol.asyncDispose]();
```

**VibeCody:**
```ts
const agent = new VibeCLIAgent({ provider: 'claude', approval: 'full-auto' });
for await (const evt of agent.run("Refactor auth.ts")) { /* events */ }
// No separate Run handle, no run.wait(), no run.cancel(), no result shape, no asyncDispose.
await agent.stop();  // cancel-last; no per-run handle
```

**Shape gaps:**
1. **No `Run` handle.** Our `agent.run()` returns the async iterator directly; Cursor returns a `Run` object that can be queried (`.id`), waited (`.wait()`), and cancelled (`.cancel()`) independently of iteration.
2. **No structured terminal result.** Cursor's `run.wait()` returns `{ status, durationMs, result, git: { branches: [{ prUrl, branch }] } }`. Ours emits a `'complete'` event with a `content` string — no PR URL surfaced, no duration, no branch.
3. **No `[Symbol.asyncDispose]`.** Means `using agent = …` doesn't work in TS 5.2+ explicit-resource-management syntax. Modern ergonomic miss.
4. **No typed error subclasses.** Cursor ships `AuthenticationError`, `ConfigurationError`, `RateLimitError`, `IntegrationNotConnectedError`, `NetworkError`. We have one `AgentError`. Means callers can't `catch (e) { if (e instanceof RateLimitError) backoff(); }`.
5. **No async-factory pattern.** Cursor's `Agent.create()` does an async warm-up (auth + capability probe); ours is synchronous `new VibeCLIAgent()` which means errors only surface on the first `.run()` call.

### Code-shape B — Subagents

**Cursor:**
```ts
// .cursor/agents/code-reviewer.md
// ---
// model: composer-2
// tools: [Read, Grep]
// ---
// You are a careful code reviewer …

const agent = await Agent.create({
  agents: { 'code-reviewer': { /* override config */ } },
  …
});
// Parent agent spawns subagent via "Agent" tool inside its loop;
// in v2.5 the spawn can be async (parent continues while subagent runs).
```

**VibeCody:**
```ts
// Nothing in @vibecody/agent-sdk. The daemon's async_subagent.rs has:
//   AsyncSubagentRegistry::register / mark_running / mark_completed / cancel / poll
// but no .vibecli/agents/*.md discovery, no HTTP route, no SDK method.
```

**Shape gaps:**
1. **No filesystem discovery.** Cursor scans `.cursor/agents/*.md` on agent construction; we have no convention.
2. **No subagent-spawn-from-tool.** Parent → child invocation through the tool layer is the entire UX of subagents. Without it, the registry in `async_subagent.rs` is just data structures.
3. **No async-vs-sync spawn flag.** Cursor 2.5 distinguishes; ours doesn't model the difference because nothing's wired.

### Code-shape C — Hooks

**Cursor `.cursor/hooks.json`** (inferred shape from blog + docs):
```json
{
  "hooks": [
    { "event": "PreToolUse", "tools": ["Bash"], "command": "./scripts/audit-bash.sh" },
    { "event": "PostToolUse", "command": "./scripts/log.sh" }
  ]
}
```

**VibeCody:**
```ts
// @vibecody/agent-sdk has a HookConfig type:
export interface HookConfig {
  event: 'PreToolUse' | 'PostToolUse' | 'SessionStart' | 'TaskCompleted' | 'Stop';
  tools?: string[];
  command: string;
}
// …but no method on VibeCLIAgent uses it. Type is exported, never consumed.
```

**Shape gaps:**
1. **`HookConfig` is dead-exported.** The type exists in `index.ts:56–62` but no `agent.registerHook(...)`, no `Agent.create({ hooks: [...] })`, no daemon route accepts it.
2. **No `.vibecli/hooks.json` convention.** Cursor's filesystem-discovery shape works for both editor and SDK; we'd benefit from the same uniformity.
3. **Daemon-side: `hook_abort.rs` exists** but is bound to the Rust loop, not to a SDK-callable surface.

### Code-shape D — Recap / resume + reconnect

**Cursor:**
```ts
// runId persisted somewhere (DB, environment, sticky note)
const run = await Agent.getRun(runId, {
  runtime: "cloud",
  agentId,
  apiKey,
});
const result = await run.wait();   // resumes streaming + final result
```

**VibeCody daemon (already shipped):**
```
POST /v1/recap        { kind: "session" | "job", session_id, force? }
                      → narrative summary + structured timeline
GET  /v1/recap/:id
PATCH /v1/recap/:id   { title, summary, tags }
POST /v1/resume       { kind: "session" | "job", session_id, handoff_envelope }
                      → re-attach to the in-flight session (verifies P-256
                        signature via session_resume_protocol.rs)
```

**VibeCody SDK:** none of the above is wrapped.

**Shape gaps:**
1. **SDK can't see `/v1/recap`.** Web/mobile clients can; the TS SDK can't.
2. **SDK can't see `/v1/resume`.** The cryptographic handoff lands but only mobile/watch can present it.
3. **Cursor's `Agent.getRun(...)` shape is good** — single static method, takes the runId plus runtime context, returns a `Run` handle that streams + waits. Worth mirroring as `VibeCLIAgent.attach(sessionId, { handoff? })`.

---

## New gaps surfaced → roadmap-entry candidates

Add these to FIT-GAP §16 as **D1–D6** (the next free letter after the C1–C3 trivial closes in v14). All six are SDK-surface work, not protocol work; the daemon side is mostly already there.

| # | Gap | Surface needed | Daemon side | Effort |
|---|-----|---------------|-------------|--------|
| D1 | TypeScript SDK does not expose subagents — no filesystem discovery, no spawn API, no `.vibecli/agents/*.md` convention | `Agent.create({ agents: { … } })`-style config + `agent.spawnSubagent(name, prompt, { async? })` + `.vibecli/agents/*.md` loader | `async_subagent.rs` registry shipped (PR #21); needs HTTP route `POST /agents/spawn` and `.vibecli/agents/` filesystem watcher | M |
| D2 | Hooks are typed but not used; no `.vibecli/hooks.json` convention | `Agent.create({ hooks: [...] })` consumes the existing `HookConfig` type; SDK method registers hooks with daemon | Add `/hooks/register` route + reuse `hook_abort.rs` semantics; converge with `.claude/settings.json` PostToolUse for compatibility | M |
| D3 | Skills not exposed in SDK — daemon has `list_skills` / `get_skill` MCP tools but TS callers can't trigger them | `agent.listSkills({ category?, query? })`, `agent.skill(name).invoke(prompt)` | Already covered by PR #11 (`skill_catalog.rs` + MCP tools); needs thin HTTP wrappers if MCP isn't already exposed over HTTP | S |
| D4 | Sandbox-tier selector missing from SDK; sandbox tiers themselves are designed but not wired into `serve.rs` | `Agent.create({ sandbox: 'native' \| 'broker' \| 'firecracker' \| 'hyperlight' })` | Wire the sandbox-tier design from `docs/design/sandbox-tiers/` into `serve.rs`; bwrap profile is already dead-code-ready | L (depends on sandbox-tiers landing) |
| D5 | `/v1/recap` and `/v1/resume` are not wrapped in the SDK | `agent.recap()`, `agent.resume({ handoff })`, `VibeCLIAgent.attach(sessionId)` mirroring Cursor's `Agent.getRun(...)` | Routes shipped; just wrap | S |
| D6 | SDK only targets `localhost:7878` — no `cloud` / `paired-url` runtime mode | `new VibeCLIAgent({ host, port, bearer, pairingUrl })` already partial; add full pairing-URL parsing + Tailscale/ngrok awareness mirroring `connectivity.md` | Daemon already serves 13 clients across LAN / Tailscale / ngrok / relay; the SDK just picks one transport | M |

**Effort key:** S = ½–1 day · M = 2–3 days · L = depends on prereq landing.

**Recommended bundling:** D3 + D5 + D2 are quick and complementary (skills, recap/resume, hooks). Ship as a single PR titled "feat(sdk): expose daemon capabilities to TypeScript callers". D1 + D6 are next; D4 waits on sandbox-tiers.

---

## Patent-distance check on the proposed shape

Per the cross-cutting principles in `notes/PATENT_AUDIT_INLINE.md` and FIT-GAP §18, four of these candidates touch surfaces where Cursor has filed-or-likely-to-file claims. Pre-implementation distance check:

| Gap | Cursor surface | Distance posture for VibeCody |
|---|---|---|
| D1 (subagents) | `.cursor/agents/*.md` + Agent-tool spawn from parent loop | **Use existing prior art directly** — subagent pattern is well-established (LangChain agents 2023, CrewAI 2024, multi-agent orchestration in academic literature pre-2020). Filesystem convention is one-line different (`.vibecli/agents/` vs `.cursor/agents/`); no claim risk. **Cleared.** |
| D2 (hooks) | `.cursor/hooks.json` declarative file | **Pre-existing prior art** — Git hooks (1996), Husky (2014), npm/yarn lifecycle scripts (2010), `.claude/settings.json` (existing). Declarative hooks files are a mature shape. **Cleared.** |
| D4 (sandbox tiers) | `sandbox.json` policy + org-enforced allowlists | **Distance applies** — policy enforcement must be client-side per FIT-GAP §18 principle 2 ("no server can flip a workspace plugin or agent class from Off to Required"). Sandbox-tier configuration goes in `WorkspaceStore`, not a Cursor-style centralized admin dashboard. The four-backend design in `docs/design/sandbox-tiers/` already honors this. **Cleared with constraint.** |
| D5 (recap/resume) | `Agent.getRun(...)` reconnect | **No claim overlap** — recap is *narrative summary*, distinct from Cursor's *event-stream reconnect*. Resume uses our existing P-256 signed handoff (PR #23 / A9), which has its own prior-art lineage to JWS-detached + the watch_auth.rs key infrastructure. **Cleared.** |

D3 (skills) and D6 (multi-client) don't trigger claim concerns.

---

## What VibeCody has that Cursor doesn't (positioning)

For completeness — the audit cuts both ways. Five Cursor-doesn't-ship items worth surfacing in marketing or the next industry-delta cycle:

1. **Narrative recap is richer than event-stream reconnect.** Cursor's `Agent.getRun(...)` resumes the stream; our `/v1/recap` produces a structured summary of what happened so far. Different shape, more useful for "I closed the laptop for 2 hours, what did the agent do?".
2. **Watch + mobile companions.** VibeMobile (Flutter) + VibeCodyWatch (SwiftUI watchOS) + VibeCodyWear (Wear OS Kotlin) — no comparable Cursor surface. Resume from watch is a real workflow.
3. **ACP server mode (PR #16 / A4).** VibeCLI is reachable as an ACP v0.11 server from Zed / JetBrains / Neovim; Cursor's SDK doesn't speak ACP at all (its agents only run inside Cursor's own runtime modes).
4. **MCPB bundle format (PR #18 / A2).** Open-spec bundle distribution; Cursor's plugin marketplace bundles are proprietary.
5. **Provider-agnostic by construction.** Cursor's SDK is hard-bound to Cursor's harness + composer-2 + GPT-5.5 routing; our SDK takes a `provider` string and dispatches via `build_temp_provider()` (per CLAUDE.md "Provider-Agnostic Panels — STRICT" rule).

---

## Recommended next actions

1. **Add D1–D6 to FIT-GAP §16.4** (or §16.6 if the 2026-05-17 scheduled agent prefers a new sub-section). Land as a docs PR.
2. **Ship the SDK-wrapper bundle**: D3 + D5 + D2 as one PR. Daemon side requires zero new routes for D3 (MCP already exposes the tools); minor wrapper for D2 hooks; pure wrapper for D5.
3. **Park D4 behind sandbox-tiers**: the design lives in `docs/design/sandbox-tiers/`; until that lands as code in `serve.rs`, the SDK has nothing to expose.
4. **Refresh during the v15 industry delta**: re-check Cursor SDK changelog at 2026-05-17 for any new surface additions between v14 and v15.

## What this audit does NOT establish

- Whether `@vibecody/agent-sdk` *should* mirror `@cursor/sdk` shape-for-shape — that's a product decision, not a research finding. The audit lists gaps; whether each one is worth closing is a roadmap call.
- Whether Cursor's claim-readable surfaces (D1, D2, D4) infringe any specific patent — same disclaimer as `notes/PATENT_AUDIT_INLINE.md`. This is a risk-triage and ergonomics-comparison, not legal opinion.
- Whether token-based consumption pricing on the Cursor side will affect competitive positioning — out of scope for this audit; surfaces as a v15 positioning signal if relevant.
