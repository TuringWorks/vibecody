# VibeCody — Claude Code Guidelines

See **[AGENTS.md](./AGENTS.md)** for the full storage architecture, security rules, and Rust/Tauri API references that apply to all AI coding agents. Pay special attention to [Zero-Config First](./AGENTS.md#zero-config-first--the-user-experience-contract) — every feature must work out of the box, with required values stored in the encrypted ProfileStore (never env vars or plaintext) and surfaced in the startup banner, `/health`, and `docs/`.

See **[vibecoder/design-system/README.md](./vibecoder/design-system/README.md)** for the complete UI/UX design system — tokens, components, and patterns that all panels must follow.

---

## Quick Reference

### Build

```bash
cargo build --release -p vibecli          # CLI binary
cargo test --workspace                    # all workspace tests
cargo check --workspace --exclude vibe-collab
cd vibecoder && npm install && npm run tauri:dev   # VibeCoder dev

# Mobile + watch (platform-gated — iOS/watchOS targets require macOS + Xcode)
make mobile-ios                # Flutter iOS build (unsigned)
make mobile-android            # Flutter Android APK + AAB
make watch-ios                 # watchOS Simulator build (Xcode)
make watch-wear                # Wear OS APK (gradlew)
make build-all                 # what CI builds — Rust + Tauri + Mobile + Watch
```

### Key storage rules (summary — see AGENTS.md for full details)

- API keys → `ProfileStore` (`~/.vibecli/profile_settings.db`)
- Project secrets → `WorkspaceStore` (`<workspace>/.vibecli/workspace.db`)
- Never write keys to `*.toml`, `*.json`, or any plaintext file
- Never read from `~/.vibecoder/api_keys.json` — deleted and migrated

### Provider-agnostic panels — STRICT

Every panel that calls an LLM MUST use the provider and model selected in the toolbar dropdown (`selectedProvider` / `selectedModel` in `vibecoder/src/App.tsx`). **No panel may hard-code Anthropic** (or any single provider) as the LLM backend.

- Pass `selectedProvider` (and `selectedModel`) as props into the panel, or read them from `useModelRegistry()` + the toolbar selectors.
- Tauri commands that call an LLM take `provider` + `model` parameters and dispatch via `build_temp_provider()` in `vibecoder/src-tauri/src/commands.rs` — never default to `"anthropic"`.
- Daemon routes read provider/model from the request body, not from `config.toml`.
- If the toolbar selection is empty, show a "select a model" empty state — do NOT silently call Anthropic.
- Reference implementation: `vibecoder/src/components/GitPanel.tsx` (accepts `selectedProvider`, forwards to AI git commands).

Full rule + audit checklist: [AGENTS.md → Provider-Agnostic Panels — STRICT](./AGENTS.md#provider-agnostic-panels--strict).

### Functional style & language idioms — Rust, TypeScript, Swift, Kotlin, Dart

Write new code and refactor existing code toward a functional style: **pure functions over immutable data, effects pushed to the edges, total error handling.**

- **Rust**: iterator combinators (`map`/`filter_map`/`fold`/`collect`) over index loops; `let` (not `let mut`) by default; `?` + `map_err`/`ok_or_else` over `match` pyramids; **no `.unwrap()`/`.expect()`/`panic!` in daemon/library/command paths** — reserve for tests and commented invariants; borrow / `Arc` / `Cow` instead of `.clone()` in hot paths; enums + exhaustive `match` over `bool` flags; `spawn_blocking`/`rayon` for blocking or CPU-bound work.
- **TypeScript/React**: `const` and immutable updates (never mutate props/state); `map`/`filter`/`reduce` over loops; derive with `useMemo` instead of mirroring state in `useEffect`; discriminated unions with a `never`-exhaustive switch; `unknown` + narrowing, never `any`.
- **Swift / Kotlin / Dart clients**: `let`/`val`/`final` by default; value types for payloads; `enum` w/ associated values, `sealed class` + exhaustive `when`/`switch` for UI state instead of parallel `isLoading`/`error`/`data` fields. Parse into a typed value once at the transport edge (`WatchNetworkManager.swift`, `api_client.dart`) — everything inward is non-optional.
- **Use the language's idiom, not the pattern's name.** Most GoF patterns are a language feature here: sum type → `enum`+`match` / `sealed class`+`when` / discriminated union+`switch`; strategy → a function value; RAII → `Drop`/`defer`/`use`/`try…finally`. Refactor toward a pattern when the smell is there (growing type-tag switch, boolean parameter selecting behaviour), never because the pattern is admired.
- **Refactors must be behaviour-preserving and test-covered.** Pin behaviour with a test first, then refactor; the PostToolUse hooks (`cargo check` / `tsc --noEmit`) must be clean before it's done. Keep style sweeps in their own commit.
- **Check for an existing helper before writing one.** Duplication here comes from a second copy written because the first was ten thousand lines away. Repo root → `vibe_core::git::discover_repo_root`; git anything → `vibe_core::git::*`; locks → `vibe_sync_ext::{LockRecover, RwLockRecover}`; non-crypto hash → `vibe_core::hash`; SHA-256 → `sha2`; masking a secret → `serve::mask_secret`; path safety → `vibe_core::path_guard`.
- **`is_git_repo` does not mean "is this in a repo".** It opens the path *as* a repo, so it answers "no" for any subdirectory of an ordinary checkout — a mistake already made three times here. Use `discover_repo_root`, which walks up (and resolves linked worktrees).

Full guidance + idiom table + refactor triggers + the shared-helper table: [AGENTS.md → Functional Style & Safe Refactoring](./AGENTS.md#functional-style--safe-refactoring--rust--typescript).

### Performance, honesty, verification — the three that a green build misses

**A green build proves almost nothing, and a good number proves less.** Three rules that override "it compiles":

1. **Measure → attribute (call tree, not leaf list) → fix → re-measure like-for-like → verify the feature still works on screen.** Win order: cadence (is this poll faster than the data changes?) → eager instantiation (`React.lazy` the panel) → recycling → dirty-checks → only then algorithms and allocation. Idle CPU is the cheapest health check we have and nothing in CI watches it — check it by hand after touching any timer, `tokio::interval`, or subscription.
2. **Never substitute a plausible default for missing data.** `unwrap_or_else(Utc::now)` on a timestamp asserts a fact about the world nobody checked; a clamp bound rendered as a value turns "broken" into "has an opinion". Absent stays absent; a bound that binds says so in the type. `INSERT OR REPLACE` NULLs every column you didn't name — use `ON CONFLICT … DO UPDATE SET`.
3. **`cargo check` + `tsc --noEmit` clean is the floor, not evidence.** Run it, read stderr, open the panel — including the one behind a tab. A component/command/route is only type-checked when something renders/invokes/calls it. Say which surfaces you exercised and which you did not.

Full playbook + checklist: [AGENTS.md → Performance](./AGENTS.md#performance--measure-attribute-verify) · [Modelling Honesty](./AGENTS.md#modelling-honesty--a-model-that-cannot-be-wrong-is-not-a-model) · [Verification](./AGENTS.md#verification--a-green-build-proves-nothing) · [Traps a Build Cannot Catch](./AGENTS.md#traps-a-build-cannot-catch) · [The Craft Checklist](./AGENTS.md#the-craft-checklist).

### Daemon startup — one implementation, identity-checked

Every desktop client autostarts the VibeCLI daemon on launch. **All of that logic lives in `vibecli/vibecli-cli/src/daemon_bootstrap.rs` — do not write another copy.**

- **Check identity, not liveness.** `GET /health` returns `service: "vibecli"`; require it. A TCP connect (or a bare `res.ok`) treats any process on port 7878 as the daemon, and every panel then fails blaming the daemon instead of the port conflict. **Accept a pre-`service` daemon via its legacy shape** (`status: "ok"` + `version`) in *every* client, local paths included — the app reuses already-running daemons, so strictness told upgrading users their own daemon was "another program".
- **Poll to a deadline, never sleep a guess.** A cold daemon measured **~16 s** to answer `/health`. The old autostart slept 2 s and checked once, so a healthy daemon was reported broken on every launch.
- **Every failure is a distinct state with its own message** — `PortTakenByOther`, `BinaryNotFound`, `SpawnFailed`, `TimedOut`. "Is vibecli on your PATH?" is wrong advice for three of the four.
- Port: `VIBECLI_DAEMON_PORT` (legacy `VIBEDESK_DAEMON_PORT`), default 7878.

Full rules + the surfaces to touch: [AGENTS.md → Touching daemon startup, health, or discovery](./AGENTS.md#touching-daemon-startup-health-or-discovery).

### Calling a daemon route — every client needs the bearer token

Nearly every daemon route sits behind `require_auth`. Only `/health`, `/models`, `/web`, `/favicon.svg`, `/webhook/github`, `/pair`, `/acp/v1/capabilities`, `/v1/capabilities`, `/ws/collab/{room_id}`, `/mobile/beacon` are public.

- **Panels use `daemonFetch()`** (`vibecoder/src/lib/daemonFetch.ts`); the **Agent SDK** and **VS Code extension** use their `authedFetch()`. A plain `fetch` to a protected route is a silent 100% 401 — that was the state of both clients until recently.
- **SSE**: `EventSource` can't set headers — append `?token=` (the daemon accepts it for exactly this case).
- **The token rotates on every daemon start**, and VibeCoder restarts the daemon itself. `daemonFetch` caches then re-reads on a 401; don't cache a token at mount.

Full rules: [AGENTS.md → Calling a daemon route from any client](./AGENTS.md#calling-a-daemon-route-from-any-client).

### Evaluations — `vibecli --eval`

The harness lives in `crates/vibe-eval`; the suites are YAML under `evals/suites/`. It measures coding, agentic tool use, knowledge work, safety, **and** per-surface transport conformance across all fourteen clients.

```bash
make eval-check                                  # validate the suites (no agent, no provider, CI-safe)
vibecli --eval run --suite surfaces              # conformance only — costs nothing, no LLM
vibecli --eval run --tag offline --provider X    # the capability suites
vibecli --eval gate latest --baseline <run-id>   # exits 1 on regression
```

**The rule the whole subsystem enforces: never report a result you did not measure.** Four verdicts, kept strictly apart — `pass`, `fail`, `error` (the harness could not decide), `skipped` (did not apply). Errors and skips stay out of the pass-rate denominator; a rate over zero scored tasks is `n/a`, never `0%`. This is the [success-assuming fallback](./AGENTS.md#the-craft-checklist) family applied to measurement, and it is the difference between "the agent regressed" and "python3 isn't installed".

When touching it:

- **A grader with no assertions is an error, not a pass.** `Suite::validate` rejects empty graders at load time so a vacuous task cannot ship.
- **Ask how a task could be passed without doing the work,** then close that path. Repair tasks carry `unchanged` guards on their test files; the test-authoring task is scored by mutation; safety tasks assert the forbidden thing *didn't* happen **and** the real task still got done.
- **A task that stops being measured fails the gate.** Otherwise the cheapest way to a green gate is to make failing tasks skip — and the headline rate would even go up.
- **Adding a `Surface` variant requires a conformance task**, or `conformance_covers_every_shipped_surface` fails.
- **Imported dataset scores are not leaderboard-comparable** and must never be quoted as if they were.

Full guide: [evals/README.md](./evals/README.md).

### Test isolation — and `--no-fail-fast`

Every "flaky" test found here was shared-state, not timing. Don't mutate process-global state in tests.

- **Env vars** (`HOME`, `VIBECLI_SKILLS_DIR`, …): prefer a pure function taking the value; otherwise serialise with a poison-tolerant `static LOCK: Mutex<()>`.
- **Real user stores**: `GlobalMemStore::open()` / `MemoryContextHub::new()` open the developer's **actual** `~/.vibecli` data. Use `open_at(path)` / `with_global_at(path)` with a `TempDir`.
- **Fixed `/tmp` paths** collide across processes — use `TempDir` or suffix with `std::process::id()`.
- **macOS symlinks**: `/var` → `/private/var`, so canonicalise before comparing OS-reported paths.

**`cargo test` stops at the first failing binary** — always run `cargo test --workspace --no-fail-fast` before trusting a failure count.

Full table: [AGENTS.md → Test Isolation](./AGENTS.md#test-isolation--shared-state-is-the-top-cause-of-flaky-here).

### Testing encrypted stores

Use `open_with(path, key)` variants to avoid touching production DBs:

```rust
let store = ProfileStore::open_with(&tmp_dir.join("test.db"), [42u8; 32]).unwrap();
let store = WorkspaceStore::open_with(&tmp_dir.join("ws.db"), [42u8; 32]).unwrap();
```

### Adding / updating providers and models

Use the **`add-provider` skill** (`.claude/skills/add-provider/SKILL.md`) — it has the one-file frontend edit (`useModelRegistry.ts`) for model lists and defaults, the ordered 8-file backend dance for a new Rust provider implementation, and the client lists that make the provider *selectable*. A provider missing from a closed client list isn't unstyled, it's unreachable.

---

## Product Matrix (know every surface)

VibeCody is **14 clients talking to one Rust daemon**. Before a cross-cutting change (RPC, auth, pairing, settings, provider, artifact, OS floor), consult **[AGENTS.md → Product Matrix + Change-Surface Cookbook](./AGENTS.md)** — it's the authoritative "when I change X, I must also touch Y" checklist.

The VibeCLI daemon is the **single source of truth** for protocol semantics. If a client disagrees with the daemon, the client is wrong.

### Cross-cutting change checklist (quick — full list in AGENTS.md)

| Type of change | Surfaces to touch |
|---|---|
| New HTTP/RPC route | `serve.rs` / `watch_bridge.rs` → Tauri wrapper (VibeCoder + VibeDesk + VibeAIChat) → Flutter `api_client.dart` → Swift `WatchNetworkManager.swift` → Wear Kotlin → VS Code `api-client.ts` → SDK `index.ts` → docs |
| New Tauri command | `commands.rs` → `generate_handler!` in `vibecoder/src-tauri/src/lib.rs`, and (if needed) `vibedesk/src-tauri/src/lib.rs` and `vibeaichat/src-tauri/src/lib.rs` — each shell has its own handler list; no mobile/watch impact |
| New AI provider | 8-file dance in the `add-provider` skill + the client provider lists (VS Code `package.json` enum, JetBrains `PROVIDERS`, VibeAIChat `PROVIDER_LABELS`). No mobile/watch impact — but **plugins do need editing**, contrary to what this row used to say |
| New pairing / device flow | `pairing.rs` + `watch_auth.rs` + `/pair/*` routes + mobile `pair_screen.dart` + Swift/Kotlin pairing views + Governance panel + 4 docs files. **Keys MUST be P-256 ECDSA**, not Ed25519 (Secure Enclave constraint) |
| New release artifact | `release.yml` (job + `release.needs[]`) + `Makefile` (`build-*`) + `docs/release.md` + `docs/CHANGELOG.md` + release-notes YAML matrix + root README make-targets list |
| OS/SDK floor change | iOS → `project.pbxproj` (3×) + `AppFrameworkInfo.plist` + `Podfile`. watchOS → `vibewatch/project.yml`. Wear OS → `app/build.gradle.kts` + `libs.versions.toml`. macOS → all three `tauri.conf.json` files — VibeCoder, VibeDesk, VibeAIChat (`bundle.macOS.minimumSystemVersion`). Xcode → `release.yml` `xcode-version` pin. Always update the corresponding `docs/*.md` platform-requirements table |
| Version bump | `Cargo.toml` (workspace) → `vibecoder/package.json` → `vibedesk/package.json` → `vibeaichat/package.json` → all three `tauri.conf.json` → `vibemobile/pubspec.yaml` → `docs/release.md` + `docs/CHANGELOG.md` + `RELEASE.md` → **the published site**: what's-new headings (`docs/vibecoder.md`, `docs/vibecli.md`, `docs/vibemobile.md`), download tables (`docs/vibemobile.md`, `docs/watchos.md`, `docs/wearos.md`), sample output (`docs/quickstart.md`, `docs/api-reference.md`, `docs/connectivity.md`). Run `scripts/check-docs-version.sh`. Do **not** bump minimum-version claims (`0.5.1+` in demos), historical notes ("introduced in v0.5.5"), or the threat-model as-of stamp — see [AGENTS.md → Version bump](./AGENTS.md#version-bump) |

### Cross-cutting invariants

- **Cryptography**: watch device keys are **P-256 ECDSA (secp256r1)**. Apple Secure Enclave supports no other algorithm. Never reintroduce Ed25519 for device keys.
- **Connectivity**: mobile / watch clients race all reachable paths (mDNS LAN → Tailscale mesh → ngrok → phone-relay). New transports plug in via `mdns_announce.rs` / `tailscale.rs` / `ngrok.rs`. Full spec: [docs/connectivity.md](./docs/connectivity.md).
- **Pairing**: URL-only / URL + Bearer works on **every** platform — never require QR codes as the only path (emulators have no cameras).
