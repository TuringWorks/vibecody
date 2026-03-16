# VibeCody v0.3.0 Release

**AI-powered developer toolchain — terminal assistant + desktop code editor.**

---

## What's New in v0.3.0

### Dynamic Editor Theming
The Monaco code editor now dynamically adapts to whatever VibeUI theme is active. Switch themes and watch syntax highlighting, gutters, selections, diff colors, and 60+ editor UI elements update instantly — no restart needed. The new `useEditorTheme` hook bridges VibeUI's CSS variable system to Monaco's `IStandaloneThemeData`, so every theme (including custom ones) automatically generates a matching editor experience.

### 40+ Themes — Including Rivian & Apple Collections
- **10 Rivian theme pairs** inspired by R1/R1S/R2 exterior and interior colors: Rivian Blue, Forest Green, El Cap Granite, Midnight, Red Canyon, Launch Green, Catalina Cove, Storm Blue, Half Moon Grey, and Borealis — each with a dark and light variant.
- **10 Apple theme pairs** based on MacBook and iPhone product finishes: Space Black, Mac Midnight, Space Gray, Black Titanium, Blue Titanium, Desert Titanium, Ultramarine, iPhone Teal, iPhone Pink, and iPhone Green.
- All themes pass WCAG contrast checks with automatic post-processing.

### Security Operations Center
- **Blue Team Panel** — Incident management (P1-P4 severity), IOC tracking (9 indicator types), SIEM integration (Splunk, Sentinel, Elastic, QRadar, CrowdStrike, Wazuh, Datadog, SumoLogic), forensic case management, detection rule authoring with platform-specific query generation, and threat hunting workflows.
- **Purple Team Panel** — MITRE ATT&CK exercise runner with 14 tactics and 20 pre-loaded techniques, attack simulation, detection validation, coverage gap analysis, heatmap generation, and cross-exercise comparison.
- **Red Team Panel** — Offensive security testing with MITRE mapping.

### Internal Developer Platform (IDP)
Full IDP integration supporting 12 platforms (Backstage, Cycloid, Humanitec, Port, Qovery, Mia Platform, OpsLevel, Roadie, Cortex, Morpheus Data, CloudBolt, Harness). Service catalog, golden paths, DORA scorecards, self-service infrastructure provisioning, team onboarding, and auto-generated Backstage catalog-info.yaml / Cycloid blueprints / Humanitec Score files.

### Futureproofing (Phases 10-14)
12 new modules with 419 tests:
- **MCP Lazy Loading** — On-demand tool loading with LRU eviction and context savings metrics
- **Context Bundles** — Priority-ordered context spaces with TOML serialization and import/export
- **Cloud Provider Integration** — AWS/GCP/Azure service detection, IAM policy generation, Terraform/CloudFormation/Pulumi templates, cost estimation
- **ACP Protocol** — Agent Client Protocol with capability negotiation and tool registration
- **MCP Directory** — Verified plugin directory with search, install, review pipeline
- **Usage Metering** — Credit system with per-user/project/team budgets, alerts, chargeback, and reporting
- **SWE-bench** — Benchmarking harness with run/compare/export
- **Session Memory** — Long-session memory profiling with leak detection and auto-compact

### Competitor Parity
13 modules closing remaining gaps vs Cursor, Windsurf, Amp, Devin, Bolt.new, and Blitzy:
- Debug mode, three agent modes (Smart/Rush/Deep), conversational search, clarifying questions with megaplan, fast context / SWE-grep, image generation agent, discussion mode, full-stack generation, enhanced agent teams, team governance, cloud autofix, GitHub Actions agent, and render optimization.

### UI Polish
- **Keep-alive panel rendering** — Panels retain state across tab switches instead of remounting
- **CSS variable normalization** — 52+ panel files migrated from hardcoded hex colors to CSS variables for consistent dark/light theme support
- **Google OAuth login** with profile auto-fill
- **Encrypted panel settings store** with profile manager
- **Premium UI redesign** with refined typography, spacing, and component hierarchy

### By the Numbers
| Metric | v0.2.0 | v0.3.0 |
|--------|--------|--------|
| AI Providers | 17 | 17 |
| VibeUI Panels | 98 | 139+ |
| Skills | 507 | 530+ |
| Tests | 2,810 | 5,900+ |
| Themes | ~10 | 40+ |
| REPL Commands | 55 | 72+ |
| Tauri Commands | ~80 | 100+ |

---

## Downloads

### VibeCLI — Terminal AI Assistant

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `vibecli-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `vibecli-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 (static musl) | `vibecli-x86_64-linux.tar.gz` |
| Linux ARM64 (static musl) | `vibecli-aarch64-linux.tar.gz` |
| Windows x64 | `vibecli-x86_64-windows.zip` |
| Docker | `vibecli-docker-v0.3.0.tar.gz` |

### VibeUI — Desktop Code Editor

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `.dmg` |
| macOS (Intel) | `.dmg` |
| Linux x64 | `.deb` / `.AppImage` |
| Windows x64 | `.msi` / `.exe` |

### Quick Install

```bash
# One-liner (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/anthropics/vibecody/main/install.sh | sh

# Docker (air-gapped / on-prem)
docker compose up -d
```

---

## Full Changelog

39 commits since v0.2.0. See [compare view](../../compare/v0.2.0...v0.3.0) for the complete diff.
