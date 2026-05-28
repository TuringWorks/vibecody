# VibeCody v0.5.6 Release

**AI-powered developer toolchain — terminal assistant + desktop code editor.**

---

## What's New in v0.5.6

### Plugin System (B2.6 – B2.12)

The plugin infrastructure goes end-to-end: signed MCPB bundles install from URLs, governance registers MCP servers from installed plugins, skills activate through the catalog, and — critically — **plugin hooks and rules now fire on both the CLI and daemon agent paths**. Every `AgentLoop::new` in `serve.rs` calls `plugin_runtime::merge_with_plugin_hooks`, so mobile, watch, VibeUI, VS Code, and JetBrains all run admin policy hooks. Plugin rules render under "### {plugin}/{rule}" in the context assembler, influencing chat and agent prompts.

- **B2.6** — Plugin Governance panel + Tauri bridge for signed bundle verification
- **B2.7** — Plugin skills appear in MCP `list_skills` / `get_skill`
- **B2.8** — `mcp_governance` registers MCP servers from installed plugins
- **B2.9** — Plugin hooks merge into agent dispatch (CLI + daemon)
- **B2.9.daemon** — Plugin hooks fire on the `/v1/agent` path (mobile/watch/IDE reach)
- **B2.10** — Plugin rules land in agent + chat system context
- **B2.12** — `vibecli plugin install <https://...>` downloads and verifies signed MCPB bundles

### /goal Feature (G1 – G13)

Full goal lifecycle: create, tree-view, pin/unpin, keyword search, inline-editable tags, goal-aware agent context (pinned-goal preamble injection), and a "Working toward" banner above the VibeUI chat. Apple Watch and Wear OS render ★ on pinned goals. VS Code shows pinned goals in the sidebar tree. TUI gets tree-mode toggle and pin hotkey.

- **G1–G6** — Schema, CRUD, pin/unpin, goal-aware agent context
- **G7** — Agent preamble injection from pinned goal title + statement + criteria
- **G8** — VS Code goals tree + mobile "+ New Goal" flow
- **G9** — Pinned-goal banner above VibeUI chat tabs
- **G10** — Keyword search + inline-editable tag chips
- **G11** — TUI tree-mode toggle + Watch ★ pin marker
- **G12** — Watch ★ on detail screens + Wear OS tile preference for pinned goal
- **G13** — VS Code + TUI ★ pin parity

### Sandbox Tiers (F0 – F8, H0 – H6)

The sandbox subsystem gains Firecracker and Hyperlight crate skeletons, rootfs building and CI, vsock broker config, virtio-fs shares, HTTP-over-UDS client, per-skill `SkillSandboxPolicy` schemas, and a standalone `sandbox-doctor` host probe. Tool execution now wires through `vibe-sandbox` Tier-0; Wasmtime gets fuel + epoch enforcement on `vibe-extensions`.

### Security Posture Scanner

New unifier panel, finding shape, 2 adapters (SonarQube + regex taint), persistence, and audit log. Secret-leak and license-clash scanners join the taint stub. 22 more Tauri commands get path-traversal gating. `path_guard` promoted to `vibe-core` (single source of truth, four consumers dedup'd).

### Watch & Mobile — Phone Relay Consolidation

Companion relay services moved from standalone apps into VibeMobile proper. iOS gets a `WatchConnectivityBridge` (WCSession relay + Keychain credentials). Android gets a `WearDataLayerService` (Wearable Data Layer + SharedPreferences credentials). Flutter `relay_bridge.dart` pushes paired-machine credentials to native keystores via MethodChannel.

### Hook Protocol Parity (VS Code + JetBrains)

VS Code and JetBrains plugins now implement the same seven-event hook contract as the CLI: `sh -c <command>` (or `cmd /c` on Windows), exit-code semantics (0/2/other), structured-JSON-decision stdout override, 30s per-hook timeout, ordered-chain short-circuit. `UserPromptSubmit` gates every prompt entry point with an event-source discriminator.

### MCP Apps Embedding Host (A1)

Generic React embedding host for MCP app panels in VibeUI — any MCP server that exposes an `app` resource can render inside a sandboxed iframe with postMessage RPC.

### OpenMemory — TurboQuant Index

`CompressedMemoryIndex` replaces the legacy f32 HNSW with a ~3 bits/dim PolarQuant + QJL backing store (≥ 8× smaller on disk). `/memory/stats` exposes index telemetry. `vibe-infer` crate ships pure-Rust embedding traits with an opt-in `candle` backend loading MiniLM-L6-v2.

### Dependency Updates

- **Rust**: hyper 1.9 → 1.10, minijinja 2.19 → 2.20, reqwest 0.13.3 → 0.13.4, tauri 2.11.1 → 2.11.2, tower-http 0.6.10 → 0.6.11, +45 transitive bumps
- **npm/VibeUI**: @tauri-apps/api 2.10 → 2.11, @tauri-apps/plugin-dialog 2.4 → 2.7, @tauri-apps/plugin-opener 2.5.3 → 2.5.4, @playwright/test 1.58 → 1.60, dompurify 3.2 → 3.4, fuse.js 7.1 → 7.3, jsdom 29.0 → 29.1, typescript-eslint 8.57 → 8.60, vitest 4.1.0 → 4.1.7, +106 changed packages
- **npm/VibeApp**: @tauri-apps/api 2.10 → 2.11, @tauri-apps/plugin-opener 2.5.3 → 2.5.4
- **Flutter/VibeMobile**: shared_preferences 2.5.3 → 2.5.5, path_provider_android 2.2.19 → 2.3.1, path_provider_foundation 2.4.2 → 2.6.0, vm_service 15.0 → 15.2

### Bug Fixes

- Fix `crate::path_guard` not visible in binary target (missing `mod path_guard;` in `main.rs`)
- Fix `truncate_snippet` byte-count off-by-N (U+2026 is 3 UTF-8 bytes, not 1)
- Fix `sign_manifest` test arity (2-arg call site → 3-arg)
- Restore inadvertently deleted `WatchConnectivityBridge.swift`
- Cargo watch errors + warnings cleanup
- Accept workspace paths on `/Volumes` external drives
- VibeUI design-system audit: 235/235 panels clean

---

## Downloads

### VibeCLI — Terminal AI Assistant

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `vibecli-0.5.6-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `vibecli-0.5.6-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 (static musl) | `vibecli-0.5.6-x86_64-linux.tar.gz` |
| Linux ARM64 (static musl) | `vibecli-0.5.6-aarch64-linux.tar.gz` |
| Windows x64 | `vibecli-0.5.6-x86_64-windows.zip` |
| Docker | `vibecody/vibecli:0.5.6` |

### VibeUI — Desktop Code Editor

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `VibeUI_0.5.6_aarch64.dmg` |
| macOS (Intel) | `VibeUI_0.5.6_x64.dmg` |
| Linux x64 | `.deb` / `.AppImage` |
| Windows x64 | `.msi` / `.exe` |

### Quick Install

```bash
# One-liner (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh

# Docker (air-gapped / on-prem)
docker pull vibecody/vibecli:0.5.6

# Verify
vibecli --version   # Should print: vibecli 0.5.6
```

---

## Upgrade Guide

### From v0.5.x

No breaking changes. Update the binary and restart:

```bash
# CLI
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh

# VibeUI — Download the new .dmg/.deb/.msi from the release page

# Docker
docker pull vibecody/vibecli:0.5.6
```

### New Features to Try First

1. **Plugin hooks** — Admin policy hooks now fire on daemon paths (mobile/watch/IDE)
2. **/goal** — Create goals, pin them, and your agent context gets a goal preamble
3. **Security posture** — Unifier panel with SonarQube + regex taint adapters
4. **Phone relay** — Watch apps can reach the daemon through VibeMobile when off-LAN
5. **TurboQuant** — Memory index is now 8× smaller on disk

---

## Full Changelog

See [CHANGELOG.md](CHANGELOG.md) for the complete history.
See [compare view](../../compare/v0.5.5...v0.5.6) for the v0.5.6 diff.
