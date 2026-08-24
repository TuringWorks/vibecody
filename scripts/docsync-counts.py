#!/usr/bin/env python3
"""Sync VibeCody docs to verified repo ground truth.

Ground truth (measured 2026-08-22 at v0.5.10):
  providers selectable : 25   (useModelRegistry.ts STATIC_MODELS / PROVIDER_DEFAULT_MODEL)
  provider backends    : 23 concrete + 1 failover meta + 2 compat helpers (26 files)
  skill files          : 1,144 across 155 categories
  panels               : 246 *Panel.tsx + 41 composite dashboards (303 top-level components)
  tauri commands       : 1,349 #[tauri::command]
  repl slash commands  : 142
  rust tests           : 16,102 #[test]/#[tokio::test]; 89 harness files in vibecli tests/
  ProfileStore cipher  : ChaCha20-Poly1305 AEAD, random 12-byte nonce prepended
  OpenMemory encrypt   : repeating-key XOR (NOT AES-256-GCM)
"""
import os
import re
import sys

# (path, old, new, expected_min_hits)  -- exact substring replacements
EXACT = [
    # ---------- providers: crate-layout contexts ----------
    ("README.md",
     "# 22 AI providers + shared openai_compat; agents, hooks, planner",
     "# 23 provider backends + failover + openai_compat; agents, hooks, planner"),
    ("README.md",
     "# See docs/configuration.md for all 22 providers",
     "# See docs/configuration.md for all 25 providers"),
    ("SOUL.md",
     "`vibe-ai` (22 AI providers + failover)",
     "`vibe-ai` (23 AI provider backends + failover)"),
    ("AGENTS.md",
     "`vibe-ai` (22 providers)",
     "`vibe-ai` (23 provider backends + failover)"),
    ("docs/architecture.md",
     "← Library: AI providers + agent (22 providers + openai_compat)",
     "← Library: AI providers + agent (23 backends + failover + openai_compat)"),
    ("docs/architecture.md",
     "All 24 providers follow the same pattern",
     "All 23 provider backends follow the same pattern"),
    ("docs/architecture.md",
     "| 22 providers + openai_compat, tools,",
     "| 23 provider backends + failover + openai_compat, tools,"),
    ("docs/development.md",
     "# AIProvider trait, agent loop, 22 providers",
     "# AIProvider trait, agent loop, 23 provider backends"),
    ("docs/development.md",
     "All 22 AI providers implement `AIProvider`",
     "All 23 AI provider backends implement `AIProvider`"),
    ("docs/vibecoder.md",
     "| 23 providers, tools, trace,",
     "| 23 provider backends + failover, tools, trace,"),
    ("docs/PLUGIN-DEVELOPMENT.md",
     "All 17 AI providers implement the `AIProvider` trait:",
     "All 23 AI provider backends implement the `AIProvider` trait:"),
    ("docs/contributing.md",
     "(23 providers exist today)",
     "(23 provider backends exist today; 25 are selectable in the UI picker)"),

    # ---------- skills ----------
    ("README.md", "# 711 skill files (25+ categories)", "# 1,144 skill files (155 categories)"),
    ("docs/architecture.md", "← 1,143 skill files (154 categories)", "← 1,144 skill files (155 categories)"),
    ("docs/contributing.md", "(1,143 skills exist today)", "(1,144 skills exist today)"),
    ("docs/development.md", "# 1,143 skill files", "# 1,144 skill files"),
    ("docs/index.md", "- 556 skill files across 25+ categories (106+ REPL commands)",
     "- 1,144 skill files across 155 categories (142 REPL slash commands)"),
    ("docs/setup.md", "7 review detectors, 550+ skills, LSP", "7 review detectors, 1,144 skills, LSP"),
    ("docs/skillforge.md", "VibeCody ships ~710 skill files in `vibecli/vibecli-cli/skills/*.md`",
     "VibeCody ships 1,144 skill files in `vibecli/vibecli-cli/skills/*.md`"),
    ("docs/skillforge.md", "the ~1,140 skills as a table", "the 1,144 skills as a table"),
    ("docs/skillforge.md", "the 710 shipped skills stay pristine", "the 1,144 shipped skills stay pristine"),
    ("docs/use-cases.md", "**556+ skill files**", "**1,144 skill files**"),
    ("docs/use-cases.md", "| **Skill Library** | 550+ skill files |", "| **Skill Library** | 1,144 skill files |"),
    ("docs/vibecli.md", "VibeCody ships with **568 skill files** across 25+ categories",
     "VibeCody ships with **1,144 skill files** across 155 categories"),
    ("docs/vibecli.md", "# 568 skill files (25+ categories)", "# 1,144 skill files (155 categories)"),
    ("docs/PLUGIN-DEVELOPMENT.md", "(built-in, 568 skills)", "(built-in, 1,144 skills)"),
    ("docs/llms-full.txt", "568 skill files (25+ categories)", "1,144 skill files (155 categories)"),
    ("docs/llms-full.txt", "539+ skill files", "1,144 skill files"),
    ("docs/FEATURE-MATRIX.md", "shipped 1,143 skills untouched", "shipped 1,144 skills untouched"),

    # ---------- panels ----------
    ("README.md", "# 293 panels + 42 composite dashboards",
     "# 246 *Panel.tsx + 41 composite dashboards (303 top-level components)"),
    ("AGENTS.md", "**1,045+ Tauri commands**, ~293 panels +",
     "**1,349 Tauri commands**, 246 panels +"),
    ("docs/architecture.md", "← React + TypeScript frontend (~293 panels + 42 composites)",
     "← React + TypeScript frontend (246 panels + 41 composites)"),
    ("docs/development.md", "# 235+ panel components (plus 39 composites)",
     "# 246 panel components (plus 41 composites)"),
    ("docs/faq.md", "with 187 integrated panels", "with 246 integrated panels"),
    ("docs/glossary.md", "VibeCoder includes 187 panels covering AI, security,",
     "VibeCoder includes 246 panels covering AI, security,"),
    ("docs/index.md", "(Tauri + Monaco, 293+ panels)", "(Tauri + Monaco, 246 panels)"),
    ("docs/setup.md", "| **Desktop App** | ✅ VibeCoder (196+ panels) | ❌ |",
     "| **Desktop App** | ✅ VibeCoder (246 panels) | ❌ |"),
    ("docs/use-cases.md", "VibeCoder with 196+ panels (Tauri + React)",
     "VibeCoder with 246 panels (Tauri + React)"),
    ("docs/vibecoder.md", "has **293 panel components + 42 composites** across categories:",
     "has **246 panel components + 41 composites** across categories:"),
    ("docs/RL-OS-ARCHITECTURE.md", "Leverages existing 196+ panel infrastructure",
     "Leverages existing 246-panel infrastructure"),
    ("docs/llms-full.txt", "187 panel components", "246 panel components"),

    # ---------- tauri commands ----------
    ("README.md", "# Tauri Rust backend (1,045+ commands)", "# Tauri Rust backend (1,349 commands)"),
    ("docs/architecture.md", "← Binary: Tauri desktop app (1,045+ Tauri commands)",
     "← Binary: Tauri desktop app (1,349 Tauri commands)"),
    ("docs/development.md", "# Tauri command registration (1,045+ commands)",
     "# Tauri command registration (1,349 commands)"),
    ("docs/development.md", "VibeCoder exposes 1,045+ Tauri commands.",
     "VibeCoder exposes 1,349 Tauri commands."),
    ("docs/vibecoder.md", "1,045+ Tauri commands (files, git, AI, agent …)",
     "1,349 Tauri commands (files, git, AI, agent …)"),

    # ---------- repl commands ----------
    ("docs/RL-OS-ARCHITECTURE.md", "and 106+ REPL commands", "and 142 REPL slash commands"),
    ("docs/use-cases.md", "**106+ REPL commands**", "**142 REPL slash commands**"),
    ("docs/use-cases.md", "| **REPL Commands** | 106+ commands with subcommands |",
     "| **REPL Commands** | 142 slash commands with subcommands |"),

    # ---------- tests ----------
    ("README.md", "**11,000+ unit tests + 62 BDD/integration harnesses** across the workspace.",
     "**16,102 test functions + 89 BDD/integration harnesses** across the workspace.\n\n> Counted at v0.5.10 by `#[test]` / `#[tokio::test]` attributes across `crates/`,\n> `vibecli/`, `vibecoder/crates/` and `vibecoder/src-tauri/`, plus harness files in\n> `vibecli/vibecli-cli/tests/`. A count is not a pass rate — run `make test` for that."),
    ("docs/AGENT-FRAMEWORK-BLUEPRINT.md", "**Excellent test coverage** (11,000+ unit tests + 62 BDD harnesses)",
     "**Excellent test coverage** (16,102 test functions + 89 BDD harnesses)"),
    ("docs/architecture.md", "**11,000+ unit tests + 62 BDD / integration harnesses** across the workspace (0 failures in CI).",
     "**16,102 test functions + 89 BDD / integration harnesses** across the workspace (0 failures in CI).\n\nCounted at v0.5.10 by `#[test]` / `#[tokio::test]` attribute. A count is not a pass rate."),
    ("docs/development.md", "# Full workspace (~10,535 tests)", "# Full workspace (16,102 test functions)"),
    ("docs/vibecoder.md", "**9,570 tests** pass across the workspace (as of 2026-03-29, 0 failures).",
     "**16,102 test functions** across the workspace (counted at v0.5.10 by `#[test]` / `#[tokio::test]` attribute)."),
    ("docs/llms-full.txt", "Total: 9,570 unit tests (0 failures)", "Total: 16,102 test functions"),

    # ---------- OpenMemory: XOR is not AES ----------
    ("docs/AGENT-FRAMEWORK-BLUEPRINT.md",
     "5 cognitive sectors, HNSW index, AES-256-GCM encryption, bi-temporal knowledge graph",
     "5 cognitive sectors, HNSW index, XOR obfuscation at rest (not cryptographic — see memory-guide), bi-temporal knowledge graph"),
    ("docs/configuration.md",
     "encryption = false          # AES-256-GCM at rest — run /openmemory encrypt to enable",
     "encryption = false          # XOR obfuscation at rest — NOT cryptographic; see memory-guide.md"),
    ("docs/memory-architecture.md",
     "| `~/.vibecli/openmemory/` + project-scoped | AES-256-GCM |",
     "| `~/.vibecli/openmemory/` + project-scoped | XOR obfuscation (not cryptographic) |"),
    ("docs/memory-architecture.md",
     "encryption = false               # AES-256-GCM at rest",
     "encryption = false               # XOR obfuscation at rest — NOT cryptographic"),
    ("docs/memory-guide.md",
     "encryption = false              # AES-256-GCM at rest (see /openmemory encrypt)",
     "encryption = false              # XOR obfuscation at rest — NOT cryptographic (see /openmemory encrypt)"),
    ("docs/memory-guide.md",
     "/openmemory encrypt                 Enable AES-256-GCM encryption",
     "/openmemory encrypt                 Enable XOR obfuscation at rest (not cryptographic)"),
    ("docs/vibecli.md",
     "| `/openmemory encrypt` | Enable AES-256-GCM encryption at rest |",
     "| `/openmemory encrypt` | Enable XOR obfuscation at rest (not cryptographic — see memory-guide.md) |"),
    ("docs/vibecoder.md",
     "Cognitive memory engine: 5 sectors, associative graph, HNSW index, AES-256-GCM encryption",
     "Cognitive memory engine: 5 sectors, associative graph, HNSW index, XOR obfuscation at rest (not cryptographic)"),

    # ---------- ghost text: stale removal claim ----------
    ("docs/vibecoder.md",
     "A deliberate alternative to keystroke-driven ghost text — there is no inline-completion / FIM / next-edit-prediction surface in VibeCody (those were removed in 2026-04-26).",
     "A deliberate alternative to keystroke-driven ghost text. Keystroke-driven inline completion was removed on 2026-04-26; inline completion returned later as an **explicit-trigger-only** surface bound to ⌥\\ (`vibe_ai::ghost`, 12-line cap) — it never fires on a keystroke. See [ghost-text.md](./ghost-text.md)."),
]

# (path, regex, replacement) -- applied after EXACT
REGEX = [
    # user-facing provider counts -> 25
    ("docs/faq.md", r"VibeCody supports 23 AI providers:", "VibeCody supports 25 AI providers:"),
    ("docs/index.md", r"### Multi-Provider AI \(23 Providers\)", "### Multi-Provider AI (25 Providers)"),
    ("docs/quickstart.md", r"\ball 23 providers\b", "all 25 providers"),
    ("docs/quickstart.md", r"\bAll 23 providers\b", "All 25 providers"),
    ("docs/use-cases.md", r"\*\*23 AI providers\*\*", "**25 AI providers**"),
    ("docs/use-cases.md", r"across all 23 providers", "across all 25 providers"),
    ("docs/use-cases.md", r"\| 23 providers \(local \+ cloud\) \|", "| 25 providers (local + cloud) |"),
    ("docs/vibecoder.md", r"supports all 22 providers via the shared", "supports all 25 providers via the shared"),
    ("docs/llms.txt", r"\b23 AI providers,", "25 AI providers,"),
    ("docs/llms.txt", r"^23 providers, listed with env var", "25 providers, listed with env var"),
    ("docs/llms-full.txt", r"ALL 23 AI PROVIDERS", "ALL 25 AI PROVIDERS"),
    ("docs/FEATURE-MATRIX.md", r"\*\*24\+ providers\.\*\*", "**25 providers.**"),
    ("llms.txt", r"^17 providers:", "25 providers:"),
    # RL-OS / blueprint design docs still quoting 18
    ("docs/RL-OS-ARCHITECTURE.md", r"\b18 AI [Pp]roviders\b", "25 AI providers"),
    ("docs/RL-OS-ARCHITECTURE.md", r"\b18 AI Providers\b", "25 AI Providers"),
    ("docs/RL-OS-ARCHITECTURE.md", r"`vibe-ai` \(18 providers\)", "`vibe-ai` (23 provider backends)"),
    ("docs/AGENT-FRAMEWORK-BLUEPRINT.md", r"Provider Ecosystem \(18 providers\)", "Provider Ecosystem (25 providers)"),
    ("docs/AGENT-FRAMEWORK-BLUEPRINT.md", r"\*\*18 providers \+ failover\*\*", "**25 providers + failover**"),
    ("docs/AGENT-FRAMEWORK-BLUEPRINT.md", r"\(20\+ providers with failover mechanisms\)", "(25 providers with failover mechanisms)"),
]


def main():
    root = os.path.expanduser(sys.argv[1] if len(sys.argv) > 1 else ".")
    os.chdir(root)
    cache = {}
    applied, missed = 0, []

    def load(p):
        if p not in cache:
            if not os.path.exists(p):
                cache[p] = None
            else:
                cache[p] = open(p, encoding="utf-8", errors="ignore").read()
        return cache[p]

    for path, old, new in EXACT:
        s = load(path)
        if s is None:
            missed.append((path, "FILE MISSING", old[:60]))
            continue
        if old not in s:
            missed.append((path, "NO MATCH", old[:80]))
            continue
        cache[path] = s.replace(old, new)
        applied += 1

    for path, pat, rep in REGEX:
        s = load(path)
        if s is None:
            missed.append((path, "FILE MISSING", pat[:60]))
            continue
        new_s, n = re.subn(pat, rep, s, flags=re.M)
        if n == 0:
            missed.append((path, "NO REGEX MATCH", pat[:80]))
            continue
        cache[path] = new_s
        applied += n

    written = 0
    for p, s in cache.items():
        if s is None:
            continue
        orig = open(p, encoding="utf-8", errors="ignore").read()
        if orig != s:
            open(p, "w", encoding="utf-8").write(s)
            written += 1

    print(f"replacements applied : {applied}")
    print(f"files written        : {written}")
    print(f"misses               : {len(missed)}")
    for m in missed:
        print("  MISS", m[0], "|", m[1], "|", m[2])


if __name__ == "__main__":
    main()
