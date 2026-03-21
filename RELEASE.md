# VibeCody v0.4.0 Release

**AI-powered developer toolchain — terminal assistant + desktop code editor.**

---

## What's New in v0.4.0

### Smart Project Understanding

VibeCody now auto-detects your project on startup and gives the AI deep context in every conversation — no more manually explaining your stack.

- **`/init` command** — Scans your workspace and detects languages (8), frameworks (25+), build/test/lint commands, entry points, architecture type, and required environment variables
- **Auto-context injection** — Project profile is automatically included in every agent conversation's system prompt
- **Task-based file gathering** — When you say "fix the tests", VibeCody auto-gathers test configs and test directories; "deploy" auto-gathers Dockerfiles and CI configs
- **Cached profiles** — Results cached to `.vibecli/project-profile.json` (1-hour TTL) for instant startup

### 5 New AI Providers (22 total)

| Provider | Type | Strength |
|----------|------|----------|
| **MiniMax** | Cloud | Chinese AI models (abab6.5) |
| **Perplexity** | Cloud | Search-augmented AI (Sonar Pro) |
| **Together AI** | Inference | Open model hosting (Llama, Qwen) |
| **Fireworks AI** | Inference | Fast open model inference |
| **SambaNova** | Inference | Hardware-accelerated inference |

All support streaming chat and code completion. Configure with env vars (`PERPLEXITY_API_KEY`, etc.) or `~/.vibecli/config.toml`.

### Always-On Channel Daemon

Run VibeCody as a 24/7 bot on Slack, Discord, or Telegram — like Claude Code Channels or Cursor Automations.

```bash
vibecli --channel-daemon slack
```

- **Automation routing** — Messages matched against rules in `.vibecli/automations/` spawn agent tasks
- **Session affinity** — Multi-turn conversations per channel+user
- **Concurrent execution** — Up to 4 parallel agent tasks (configurable)
- **Fallback chat** — Messages not matching any rule get conversational AI responses with full history

### Agent-Per-Branch Workflow

Each task gets its own git branch, runs in isolation, and auto-creates a PR:

```
> /branch-agent create add JWT auth middleware
```

The agent creates `agent/add-jwt-auth-middleware`, works autonomously, commits changes, pushes, and opens a PR with an AI-generated description.

### Polished Agent UX

The agent output is now structured and human-readable:

```
 Agent   fix the unwrap panic in src/auth.rs
  Policy: suggest (ask before every action)  |  Press Ctrl+C to stop

 ✓ Reading src/auth.rs
 ✓ Patching src/auth.rs (3 hunks)
 ✓ Running: cargo check

Agent complete: Replaced 3 unwrap() calls with ? operator.
   Files modified: src/auth.rs
   Commands run: 1
   Steps: 3/3 succeeded
   Trace saved: ~/.vibecli/traces/1711234567.jsonl
```

- **`think` tool** — Free chain-of-thought reasoning step (no side effects, doesn't count toward step limit)
- **Human-readable step descriptions** — "Reading src/auth.rs" instead of "read_file(src/auth.rs)"
- **Change summary** at completion — files modified, commands run, success rate
- **Trace & resume** — Every session can be resumed with `vibecli --resume <id>`

### Design Canvas (VibeUI)

Visual drag-and-drop component builder — inspired by Bolt.new and v0:

- **14 component types** — Container, heading, text, button, input, image, card, list, nav, form, table, hero, sidebar, footer
- **Snap-to-grid canvas** with property editor
- **Live React + Tailwind CSS code generation**
- **AI Generate tab** — Describe your UI in natural language → components
- **Export** — Copy to clipboard or save to workspace

### Authorization Scaffolding (42 Providers)

The Auth panel is now "Authorization" with 42 auth providers across 5 categories:

- **OAuth / Social (14)** — GitHub, Google, Apple, Microsoft, Facebook, Twitter/X, Discord, Slack, LinkedIn, GitLab, Bitbucket, Spotify, Twitch, Dropbox
- **Enterprise SSO (7)** — SAML, OpenID Connect, LDAP/AD, Kerberos, RADIUS, Okta, Auth0
- **Token / Key (6)** — JWT Bearer, API Key, OAuth2 Client Credentials, Basic Auth, Hawk, mTLS
- **Credential / Passwordless (5)** — Email+Password, Phone OTP, Magic Link, Passkey (WebAuthn), TOTP 2FA
- **Platform / BaaS (10)** — Supabase, Firebase, Clerk, AWS Cognito, Keycloak, FusionAuth, Auth0 Universal, Stytch, WorkOS, Descope

Generates scaffolding code for 85+ frameworks across 17 languages.

### MCP Panel Overhaul

- **Installed tab** — Rich plugin management with version badges, status indicators, config paths, update-all button
- **Tools tab** — Shows all 11 built-in agent tools + MCP server tools grouped by server
- **Live tool discovery** — Probes registered MCP servers to find their tools (fixes plugins like Terraform not showing tools)
- **View Tools navigation** — Click "View Tools" on an installed plugin → scrolls to and highlights its server section with smooth animation
- **Plugin tool mappings** — 22 known MCP plugins with pre-mapped tool lists as fallback

### Vulnerability Scanner

Industry-grade security scanning built into VibeCody:

- **CVE database** — 326+ known vulnerabilities
- **SAST rules** — 67 static analysis patterns
- **Lockfile parsers** — npm, yarn, pip, cargo, go, ruby, composer
- **SARIF output** — Standard format for CI integration
- **REPL**: `/vulnscan scan|deps|file|report`
- **REST API**: `POST /vulnscan/scan`, `POST /vulnscan/file`

### Spec-Driven Development

EARS (Easy Approach to Requirements Syntax) pipeline — inspired by AWS Kiro:

- Structured requirements → design → tasks workflow
- 5 EARS patterns: ubiquitous, event-driven, unwanted behavior, state-driven, optional
- Validation, cross-linking, and living document sync
- `/spec init|req|design|task|validate`

### VM Agent Orchestration

Run parallel agents in isolated Docker containers:

- Up to 8 concurrent VMs with resource limits (CPU, memory, disk, timeout)
- Auto-creates feature branches per VM
- Auto-opens PRs on completion with AI-generated descriptions
- Cost estimation and cleanup management
- `/vm launch|list|status|stop|cleanup`

### Security Hardening (14 fixes)

| Category | Fixes |
|----------|-------|
| **Path traversal** | Workspace-boundary canonicalization in TauriToolExecutor |
| **SSRF** | Block loopback, RFC 1918, link-local, cloud metadata (169.254.169.254) in 3 locations |
| **Command injection** | 19-pattern blocklist in all executors; AI response command filtering |
| **Timeouts** | 120s bash timeout (VibeUI), 300s scripts |
| **SQLite injection** | Block `.shell`, `.system`, `.import`, `ATTACH DATABASE` |
| **Cryptography** | Replace hand-rolled HMAC-SHA256 with audited `hmac`+`sha2` crates |
| **Secrets** | API keys stored with 0600 permissions; trace context redaction |
| **Session auth** | Collab sessions upgraded to 128-bit IDs + token auth |

### Documentation

- **Development Guide** — Build, test, debug, security checklist, architecture patterns
- **Security Guide** — Updated with all hardening details
- **2 new tutorials** — Project Init & Auto-Context, Always-On Channel Daemon
- **9 new demo entries** for recent features
- All docs updated to match codebase: 22 providers, 155+ panels, 93 commands, 543 skills, 7,400+ tests

### 11 New REPL Commands

| Command | Feature |
|---------|---------|
| `/init` | Project scanning and auto-context |
| `/daemon` | Channel daemon management |
| `/vm` | VM agent orchestration |
| `/branch-agent` | Agent-per-branch workflow |
| `/design` | Design-to-code (Figma/SVG/screenshot) |
| `/audio` | Text-to-speech output |
| `/org` | Cross-repo org-wide context |
| `/share-session` | Agent session sharing |
| `/data` | Data analysis (CSV/JSON) |
| `/ci-gates` | CI quality gates |
| `/agentic` | Auto-fix builds, gen tests |

### By the Numbers

| Metric | v0.3.0 | v0.4.0 |
|--------|--------|--------|
| AI Providers | 17 | **22** |
| VibeUI Panels | 139+ | **155+** |
| Skills | 530+ | **543** |
| Tests | 5,900+ | **7,400+** |
| REPL Commands | 72+ | **93** |
| Auth Providers | 6 | **42** |
| Workspace Members | 6 | **9** |
| Security Fixes | — | **14** |

---

## Downloads

### VibeCLI — Terminal AI Assistant

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `vibecli-0.4.0-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `vibecli-0.4.0-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 (static musl) | `vibecli-0.4.0-x86_64-linux.tar.gz` |
| Linux ARM64 (static musl) | `vibecli-0.4.0-aarch64-linux.tar.gz` |
| Windows x64 | `vibecli-0.4.0-x86_64-windows.zip` |
| Docker | `vibecody/vibecli:0.4.0` |

### VibeUI — Desktop Code Editor

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `VibeUI_0.4.0_aarch64.dmg` |
| macOS (Intel) | `VibeUI_0.4.0_x64.dmg` |
| Linux x64 | `.deb` / `.AppImage` |
| Windows x64 | `.msi` / `.exe` |

### Quick Install

```bash
# One-liner (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh

# Docker (air-gapped / on-prem)
docker compose up -d

# Verify
vibecli --version   # Should print: vibecli 0.4.0
```

---

## Upgrade Guide

### From v0.3.x

No breaking changes. Update the binary and restart:

```bash
# CLI
curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh

# VibeUI
# Download the new .dmg/.deb/.msi from the release page

# Docker
docker pull vibecody/vibecli:0.4.0
```

### New Features to Try First

1. **`/init`** — Run this in any project to see the auto-detection in action
2. **`/agent "your task"`** — Notice the polished output with change summaries
3. **Authorization panel** — Browse 42 auth providers and generate scaffolding
4. **MCP Directory → Installed tab** — See your installed plugins with tool counts

---

## Full Changelog

See [compare view](../../compare/v0.3.3...v0.4.0) for the complete diff.
