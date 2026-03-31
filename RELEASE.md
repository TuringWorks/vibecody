# VibeCody v0.5.2 Release

**AI-powered developer toolchain — terminal assistant + desktop code editor.**

---

## What's New in v0.5.2

### RL-OS: Unified Reinforcement Learning Lifecycle Platform

VibeCody now includes the architecture and roadmap for **RL-OS** — the industry's first vertically-integrated reinforcement learning operating system.

- **Exhaustive Fit-Gap Analysis** — 40+ competitors analyzed across 8 categories (Ray RLlib, Stable Baselines3, Isaac Lab, TRL, d3rlpy, PettingZoo, SageMaker RL, Vertex AI, and more)
- **52 gaps identified** (22 P0, 20 P1, 9 P2, 1 P3)
- **12 unique capabilities** that no existing tool provides:
  1. RL-specific policy distillation framework
  2. RL-aware quantization (preserving reward-critical activations)
  3. RL-native serving runtime (stateful action-observation loop)
  4. Unified environment manager (versioned, declarative, hybrid sim+real)
  5. RL-native observability (reward drift, distributional shift, safety constraints)
  6. End-to-end lifecycle in one tool (12/12 stages)
  7. Time-travel replay for deterministic training reproduction
  8. Automatic policy rollback on reward regression
  9. A2A protocol for multi-agent RL
  10. RL from Execution Feedback (RLEF) for code generation
  11. Cross-framework policy portability with RL metadata
  12. Declarative distillation pipeline DSL

### RL-OS Architecture (7 Core Modules)

| Module | Purpose |
|--------|---------|
| **EnvOS** | Versioned environments, declarative YAML DSL, hybrid sim+real, domain randomization |
| **TrainOS** | Distributed training (Rust scheduler), 30+ algorithms, AutoRL, curriculum learning, MARL |
| **EvalOS** | Continuous eval pipelines, scenario testing, OPE, safety constraints |
| **OptiOS** | Policy distillation, RL-aware quantization (INT8/INT4), structured pruning |
| **ModelHub** | RL-native registry with policy + env + reward function lineage tracking |
| **ServeOS** | Stateful serving, A/B testing, edge deployment (WASM/ONNX), auto-rollback |
| **RLHF** | PPO/DPO/KTO/ORPO/GRPO alignment, RLEF via VibeCody sandbox, Constitutional AI |

### 12-Stage Lifecycle Scorecard

| Stage | Best Competitor | Score | VibeCody RL-OS |
|-------|----------------|-------|----------------|
| Env Definition | Gymnasium | 1/12 | 12/12 |
| Env Versioning | (none) | 0/12 | 12/12 |
| Data Collection | d3rlpy | 2/12 | 12/12 |
| Training | RLlib | 4/12 | 12/12 |
| Distributed Training | RLlib | 4/12 | 12/12 |
| Evaluation | (none) | 0/12 | 12/12 |
| Optimization/Distillation | (none) | 0/12 | 12/12 |
| Model Registry | SageMaker | 5/12 | 12/12 |
| Deployment/Serving | Ray Serve | 4/12 | 12/12 |
| Monitoring | (none) | 0/12 | 12/12 |
| Feedback/Retraining | (none) | 0/12 | 12/12 |
| Governance/Audit | (none) | 0/12 | 12/12 |

---

## What's New in v0.5.1

### AI Code Review + Architecture Spec + Policy Engine

Production-grade code intelligence modules achieving parity with Qodo, CodeRabbit, Bito, and Cerbos:

- **AI Code Review** (`ai_code_review.rs`, 97 tests) — 7 detectors (security/OWASP, complexity, style, docs, tests, duplication, architecture), 8-linter aggregation, quality gates, learning loop with precision/recall/F1, PR summary + Mermaid diagrams
- **Architecture Spec Engine** (`architecture_spec.rs`, 108 tests) — TOGAF ADM (9 phases), Zachman (6x6 matrix), C4 Model (4 levels + Mermaid), ADR lifecycle
- **Policy Engine** (`policy_engine.rs`, 91 tests) — RBAC/ABAC, 14 condition operators, derived roles, policy testing, YAML audit trail, conflict detection

### Phase 32: Advanced Agent Intelligence

- **Health Score** (92 tests) — multi-dimensional codebase health scoring
- **Intent Refactor** (89 tests) — natural-language-driven AST refactoring
- **Review Protocol** (50 tests) — structured code review workflow with approval gates
- **Skill Distillation** (82 tests) — extract reusable skills from agent traces
- **Phase 32 P0** — context_protocol, code_review_agent, diff_review, code_replay, speculative_exec, explainable_agent
- **TurboQuant KV-Cache** — PolarQuant + QJL (~3 bits/dim) for vector DB integration

### FIT-GAP v7 Complete (22 Gaps Closed)

All 22 gaps across Phases 23-31 now closed — A2A protocol, parallel worktree agents, proactive intelligence, web grounding, semantic indexing, MCTS code repair, visual verification, native connectors, execution-based learning, and more.

---

## By the Numbers

| Metric | v0.5.0 | v0.5.2 |
|--------|--------|--------|
| Tests | ~8,500 | **~10,535** |
| VibeUI Panels | 187+ | **196+** |
| REPL Commands | 105+ | **106+** |
| Rust Modules | 185+ | **196+** |
| Skill Files | ~543 | **~550** |
| Tauri Commands | 350+ | **360+** |
| RL Competitors Analyzed | 0 | **40+** |
| RL Lifecycle Coverage | 0/12 | **12/12 (target)** |

---

## Downloads

### VibeCLI — Terminal AI Assistant

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `vibecli-0.5.2-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `vibecli-0.5.2-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 (static musl) | `vibecli-0.5.2-x86_64-linux.tar.gz` |
| Linux ARM64 (static musl) | `vibecli-0.5.2-aarch64-linux.tar.gz` |
| Windows x64 | `vibecli-0.5.2-x86_64-windows.zip` |
| Docker | `vibecody/vibecli:0.5.2` |

### VibeUI — Desktop Code Editor

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `VibeUI_0.5.2_aarch64.dmg` |
| macOS (Intel) | `VibeUI_0.5.2_x64.dmg` |
| Linux x64 | `.deb` / `.AppImage` |
| Windows x64 | `.msi` / `.exe` |

### Quick Install

```bash
# One-liner (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh

# Docker (air-gapped / on-prem)
docker compose up -d

# Verify
vibecli --version   # Should print: vibecli 0.5.2
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
docker pull vibecody/vibecli:0.5.2
```

### New Features to Try First

1. **Read `docs/FIT-GAP-RL-OS.md`** — Understand the RL competitive landscape
2. **Read `docs/RL-OS-ARCHITECTURE.md`** — Explore the RL-OS module design and YAML DSL
3. **`/aireview`** — Run AI code review on your current working directory
4. **`/archspec`** — Generate architecture documentation (C4, TOGAF, ADRs)
5. **`/policy`** — Define and test access control policies

---

## Full Changelog

See [CHANGELOG.md](CHANGELOG.md) for the complete history.
See [compare view](../../compare/v0.5.1...v0.5.2) for the v0.5.2 diff.
