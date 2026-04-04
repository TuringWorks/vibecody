# VibeCody v0.5.4 Release

**AI-powered developer toolchain — terminal assistant + desktop code editor.**

---

## What's New in v0.5.4

### Claude Code System Prompt Integration

VibeCody now incorporates **254 system prompts** from the Claude Code reference collection, bringing best-in-class behavioral guidelines directly into the agent:

- **Core behavioral rules** baked into TOOL_SYSTEM_PROMPT — read before modify, minimize files, security-first, one file per response
- **Auto-mode guidance** — FullAuto approval policy triggers autonomous execution rules (minimize interruptions, prefer action, never destroy data)
- **5 dynamic skill files** — git-commit, pr-creation, security-review, debugging, simplify (auto-activate via trigger keywords)
- All 254 prompts stored as reference skills in `skills/claude-code-prompts/`

### Stability & UX Fixes

- **Apply crash resolved** — DiffReviewPanel now overlays the editor with deferred unmount and 150ms separation from Monaco updates; React.StrictMode removed (was causing double-mount crashes)
- **Terminal persistence** — Terminal and BrowserPanel stay mounted across tab switches (display toggle instead of unmount/remount)
- **GLM/Qwen tool call support** — `<|tag|>` delimiters from GLM-4 and Qwen models are normalized so XML tool calls execute correctly
- **Incremental streaming saves** — `<write_file>` blocks flush to disk as the closing tag arrives; partial work survives stream failures
- **Error Boundary** — React ErrorBoundary catches render crashes with error + stack trace instead of blank WebView

### Provider & LSP Improvements

- **Unique provider names** — 14 providers (Cerebras, Groq, Mistral, DeepSeek, etc.) now return `"Provider (model)"` instead of static strings
- **LSP camelCase params** — fixed snake_case to camelCase for hover, completion, and goto-definition
- **Expanded limits** — agent context 200K tokens, max_steps 50, Claude max_tokens 16K, Ollama num_predict 16K + 300s timeout

---

## What's New in v0.5.3

### Document & Media Viewers

New DocumentViewer, ImageViewer, HtmlPreview, and DrawioPreview components for VibeUI with full CSS styling and test coverage.

### RL-OS Core Modules

8 core modules (EnvOS, TrainOS, EvalOS, OptiOS, ModelHub, ServeOS, RLHF, MultiAgent) with 660 tests, 10 VibeUI panels, and 20 wired Tauri commands.

### Sketch Canvas

Working drawing support with Move tool, inline text editing, SVG/PNG export, shape recognition, and code generation.

### Key Fixes

- Empty AI responses in Vibe App (SSE parser field mismatch)
- Duplicate streaming text (React StrictMode guard)
- Ollama model list performance (instant name filter instead of per-model probe)
- Monaco Apply All crash (editor kept mounted)
- Agent identity renamed from "VibeCLI" to "Vibe Agent"

---

## What's New in v0.5.2

### RL-OS: Unified Reinforcement Learning Lifecycle Platform

VibeCody now includes the architecture and roadmap for **RL-OS** — the industry's first vertically-integrated reinforcement learning operating system.

- **Exhaustive Fit-Gap Analysis** — 40+ competitors analyzed across 8 categories
- **52 gaps identified** (22 P0, 20 P1, 9 P2, 1 P3)
- **12 unique capabilities** that no existing tool provides

### AI Code Review + Architecture Spec + Policy Engine (v0.5.1)

- **AI Code Review** (`ai_code_review.rs`, 97 tests) — 7 detectors, 8-linter aggregation, quality gates, learning loop
- **Architecture Spec Engine** (`architecture_spec.rs`, 108 tests) — TOGAF ADM, Zachman, C4 Model, ADRs
- **Policy Engine** (`policy_engine.rs`, 91 tests) — RBAC/ABAC, 14 operators, derived roles, YAML audit trail

### FIT-GAP v7 Complete (22 Gaps Closed)

All 22 gaps across Phases 23-31 now closed — A2A protocol, parallel worktree agents, proactive intelligence, web grounding, semantic indexing, MCTS code repair, visual verification, native connectors, execution-based learning, and more.

---

## By the Numbers

| Metric | v0.5.0 | v0.5.4 |
|--------|--------|--------|
| Tests | ~8,500 | **~10,535** |
| VibeUI Panels | 187+ | **196+** |
| REPL Commands | 105+ | **106+** |
| Rust Modules | 185+ | **196+** |
| Skill Files | ~543 | **~555** |
| Tauri Commands | 350+ | **360+** |
| AI Providers | 18 | **18 + OpenRouter (300+)** |
| Claude Code Prompts | 0 | **254** |

---

## Downloads

### VibeCLI — Terminal AI Assistant

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `vibecli-0.5.4-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `vibecli-0.5.4-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 (static musl) | `vibecli-0.5.4-x86_64-linux.tar.gz` |
| Linux ARM64 (static musl) | `vibecli-0.5.4-aarch64-linux.tar.gz` |
| Windows x64 | `vibecli-0.5.4-x86_64-windows.zip` |
| Docker | `vibecody/vibecli:0.5.4` |

### VibeUI — Desktop Code Editor

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `VibeUI_0.5.4_aarch64.dmg` |
| macOS (Intel) | `VibeUI_0.5.4_x64.dmg` |
| Linux x64 | `.deb` / `.AppImage` |
| Windows x64 | `.msi` / `.exe` |

### Quick Install

```bash
# One-liner (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh

# Docker (air-gapped / on-prem)
docker compose up -d

# Verify
vibecli --version   # Should print: vibecli 0.5.4
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
docker pull vibecody/vibecli:0.5.4
```

### New Features to Try First

1. **Claude Code prompts** — Agent now follows best practices from 254 reference prompts automatically
2. **`/aireview`** — Run AI code review on your current working directory
3. **`/archspec`** — Generate architecture documentation (C4, TOGAF, ADRs)
4. **`/policy`** — Define and test access control policies
5. **Sketch Canvas** — Draw UI wireframes and generate code from sketches

---

## Full Changelog

See [CHANGELOG.md](CHANGELOG.md) for the complete history.
See [compare view](../../compare/v0.5.3...v0.5.4) for the v0.5.4 diff.
