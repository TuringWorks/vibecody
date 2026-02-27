---
layout: page
title: Shannon vs VibeCody — Red Teaming Feature Comparison
permalink: /shannon-comparison/
---

# Shannon vs VibeCody — Red Teaming Feature Comparison

**Date:** February 2026
**Shannon:** [github.com/KeygraphHQ/shannon](https://github.com/KeygraphHQ/shannon) — Autonomous AI-powered penetration testing framework
**VibeCody:** AI coding assistant with integrated security testing (Phase 41+)

---

## 1. Executive Summary

Shannon is a standalone autonomous pentesting tool ("The Red Team to your vibe-coding Blue team"). VibeCody is a full-stack AI coding assistant that now integrates red teaming capabilities directly into its development workflow. The key difference: Shannon is a separate tool you run *after* building; VibeCody's red team module lets you security-test *while* building, closing the build→test gap.

| Dimension | Shannon | VibeCody |
|-----------|---------|----------|
| **Primary focus** | Penetration testing | AI-assisted development + integrated security |
| **Architecture** | TypeScript + Temporal + Docker | Rust + Tokio + OS sandbox |
| **AI backend** | Claude (primary), experimental GPT/Gemini | 10+ providers (Ollama, Claude, OpenAI, Gemini, Grok, Bedrock, Copilot, Groq, OpenRouter, Azure) |
| **License** | AGPL-3.0 (Lite) / Commercial (Pro) | MIT |
| **Cost per scan** | ~$50 (Claude Sonnet, 1-1.5 hrs) | Per-token (user's configured provider) |
| **Target users** | Security teams, pentesters | Developers who want security built into their workflow |

---

## 2. Feature Matrix

### 2.1 Security Testing Pipeline

| Capability | Shannon | VibeCody (Phase 41) | Notes |
|------------|---------|---------------------|-------|
| Autonomous pentest pipeline | ✅ 5-phase (pre-recon → recon → vuln → exploit → report) | ✅ 5-stage (recon → analysis → exploitation → validation → report) | Both use multi-stage orchestration |
| Single-command launch | ✅ `./shannon start URL=<target> REPO=<name>` | ✅ `vibecli --redteam <url>` or `/redteam scan <url>` | Comparable UX |
| White-box (source-code-aware) | ✅ Analyzes repo in `./repos/` | ✅ Analyzes workspace via codebase index + embeddings | VibeCody reuses existing semantic index |
| Workspace resumability | ✅ Git commit checkpoints | ✅ Session resume + /rewind + SQLite persistence | VibeCody has richer session management |
| Parallel vuln agents | ✅ 5 concurrent | ✅ Configurable (default 3) via multi-agent orchestrator | Both leverage parallel execution |
| Report generation | ✅ Markdown pentest report with PoC | ✅ Markdown report with CVSS scores + PoC + remediation | Both produce actionable reports |

### 2.2 Vulnerability Coverage

| Vulnerability Type | Shannon | VibeCody (Phase 41) | Detection Method |
|--------------------|---------|---------------------|-----------------|
| SQL Injection (CWE-89) | ✅ Exploit + PoC | ✅ Static regex + LLM analysis + HTTP validation | Shannon validates via browser; VibeCody via HTTP requests |
| XSS — Reflected (CWE-79) | ✅ Browser-validated | ✅ Static regex + LLM + HTTP validation | Shannon uses Playwright; VibeCody uses reqwest + browser action |
| XSS — Stored (CWE-79) | ✅ Browser-validated | ✅ LLM analysis + HTTP validation | Both require multi-step payloads |
| SSRF (CWE-918) | ✅ Active exploitation | ✅ Static regex + LLM analysis | New CWE pattern added |
| Command Injection (CWE-78) | ✅ Denylist bypass | ✅ Static regex + LLM analysis | Existing pattern extended |
| Path Traversal (CWE-22) | ✅ | ✅ Static regex + LLM | Existing pattern |
| IDOR (CWE-639) | ✅ Active exploitation | ✅ Static regex + LLM analysis | New CWE pattern |
| Auth Bypass | ✅ JWT manipulation, registration bypass | ✅ LLM-driven auth flow testing | Shannon has deeper auth testing |
| Mass Assignment | ✅ Active exploitation | ✅ LLM analysis | Both LLM-powered |
| XXE (CWE-611) | ❌ Not mentioned | ✅ Static regex pattern | VibeCody advantage |
| Insecure Deserialization (CWE-502) | ❌ Not mentioned | ✅ Static regex pattern | VibeCody advantage |
| NoSQL Injection (CWE-943) | ❌ Not mentioned | ✅ Static regex pattern | VibeCody advantage |
| Template Injection (CWE-1336) | ❌ Not mentioned | ✅ Static regex pattern | VibeCody advantage |
| CSRF (CWE-352) | ❌ Not mentioned | ✅ Static regex pattern | VibeCody advantage |
| Cleartext Transmission (CWE-319) | ❌ Not mentioned | ✅ Static regex pattern | VibeCody advantage |
| Hardcoded Credentials (CWE-798) | ❌ Not mentioned | ✅ Static regex pattern | Existing pattern |
| Insecure RNG (CWE-338) | ❌ Not mentioned | ✅ Static regex pattern | Existing pattern |
| Open Redirect (CWE-601) | ❌ Not mentioned | ✅ Static regex pattern | Existing pattern |

### 2.3 Infrastructure & Tooling

| Capability | Shannon | VibeCody | Notes |
|------------|---------|----------|-------|
| Recon tools (nmap/subfinder/whatweb) | ✅ Docker containers | ⚠️ HTTP crawling + source analysis | Shannon has deeper network recon |
| Browser automation | ✅ Playwright (full) | ⚠️ reqwest + screencapture (basic) | Shannon has richer browser control |
| Docker isolation | ✅ Required | ❌ Uses OS sandbox (seatbelt/bwrap) | Different isolation models |
| Temporal durable workflows | ✅ Crash recovery, queryable | ❌ Agent loop + background jobs | Shannon more resilient to crashes |
| Auth flow YAML config | ✅ 2FA/TOTP/Google SSO | ✅ AuthFlow struct (URL, creds, selectors) | Shannon has broader auth support |
| MCP server | ✅ mcp-server/ | ✅ --mcp-server | Both expose MCP interface |
| CI/CD integration | ✅ Shannon Pro (commercial) | ✅ GitHub Actions + --exec mode | VibeCody CI is OSS |
| Progress monitoring | ✅ `./shannon logs` + Temporal UI | ✅ REPL `/redteam show` + RedTeamPanel UI | Both have live monitoring |

### 2.4 Developer Experience

| Capability | Shannon | VibeCody | Notes |
|------------|---------|----------|-------|
| Desktop GUI | ❌ CLI-only (+ Temporal Web UI) | ✅ RedTeamPanel.tsx in VibeUI | VibeCody has native desktop UI |
| IDE integration | ❌ | ✅ VS Code, JetBrains, Neovim | VibeCody has full IDE ecosystem |
| REPL commands | ❌ Shell script only | ✅ `/redteam` with tab-completion | VibeCody has interactive REPL |
| Scheduling | ❌ Manual launch | ✅ `/schedule` cron integration | VibeCody can schedule recurring scans |
| Notification gateways | ❌ | ✅ Slack/Telegram/Discord/Linear | VibeCody can alert on findings |
| BugBot PR review | ❌ | ✅ Inline GitHub PR comments | VibeCody integrates with PR workflow |
| Multi-provider LLM | ⚠️ Claude primary only | ✅ 10+ providers | VibeCody is provider-agnostic |

---

## 3. Architectural Comparison

### Shannon Architecture

```
CLI (./shannon) → Docker Compose → Temporal Worker
  └─ 5-Phase Pipeline:
     1. Pre-Recon (nmap, subfinder, whatweb, source analysis)
     2. Recon (attack surface mapping)
     3. Vulnerability Analysis (5 parallel agents)
     4. Exploitation (browser-based, conditional)
     5. Reporting (markdown with PoC)
  └─ AI: Claude Agent SDK (maxTurns: 10,000)
  └─ Storage: audit-logs/ directory, git checkpoints
```

### VibeCody Red Team Architecture

```
CLI (vibecli --redteam) or REPL (/redteam scan) or VibeUI (RedTeamPanel)
  └─ 5-Stage Pipeline (redteam.rs):
     1. Recon (HTTP crawl + source analysis via codebase index)
     2. Analysis (CWE regex + LLM white-box review)
     3. Exploitation (HTTP validation + browser actions)
     4. Validation (confirm exploitability, generate PoC)
     5. Report (markdown with CVSS + remediation)
  └─ AI: Any of 10+ providers (user-configured)
  └─ Storage: ~/.vibecli/redteam/ JSON + SQLite sessions
  └─ Reuses: bugbot.rs patterns, AgentLoop, multi-agent orchestrator,
             background_agents, session_store, workflow stage patterns
```

---

## 4. When to Use Which

| Scenario | Recommended Tool | Why |
|----------|------------------|-----|
| Annual professional pentest | **Shannon** | Deeper exploitation, browser-validated PoCs, Temporal durability |
| Security check during development | **VibeCody** | Integrated into editor/REPL, runs alongside coding workflow |
| CI/CD security gate | **Either** | Shannon Pro for full pentest; VibeCody `--redteam` for fast static+LLM scan |
| PR security review | **VibeCody** | BugBot inline comments, CWE pattern matching in diffs |
| OWASP Juice Shop / CTF | **Shannon** | Purpose-built for exploitation with 96% benchmark success |
| Quick vulnerability scan of WIP code | **VibeCody** | `/redteam scan localhost:3000` from REPL, instant results |
| Team notification on findings | **VibeCody** | Slack/Telegram/Discord gateways built-in |
| Scheduled recurring scans | **VibeCody** | `/schedule` cron integration |

---

## 5. Integration Opportunity

Shannon and VibeCody are complementary, not competing:

1. **VibeCody builds → Shannon validates**: Use VibeCody for development, trigger Shannon for deep pentest before release
2. **Shannon MCP → VibeCody MCP**: Both expose MCP servers; a meta-orchestrator could chain them
3. **VibeCody's `/redteam` for fast feedback** during development; **Shannon for thorough** pre-release security audit
4. **Shared findings format**: Both produce markdown reports; findings could feed into VibeCody's BugBot for tracking

---

*Updated 2026-02-26 — VibeCody Phase 41 (Red Team Module). Shannon analysis based on [KeygraphHQ/shannon](https://github.com/KeygraphHQ/shannon) repository.*
