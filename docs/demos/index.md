---
layout: page
title: VibeCody Feature Demos
permalink: /demos/
---

# VibeCody Feature Demos

Interactive walkthroughs covering VibeCody's major capabilities across CLI, Desktop IDE, and API surfaces.

---

## Getting Started

| Demo | Surface | Description |
|------|---------|-------------|
| [01 — First Run & Setup](01-first-run.md) | CLI | Install, configure providers, first AI chat |
| [02 — TUI Interface](02-tui-interface.md) | CLI | Navigate the terminal UI, panels, keybindings |

## AI & Providers

| Demo | Surface | Description |
|------|---------|-------------|
| [03 — Multi-Provider AI Chat](03-multi-provider-chat.md) | CLI + UI | Switch between 17 AI providers, streaming responses |
| [04 — Agent Loop & Tool Execution](04-agent-loop.md) | CLI + UI | Autonomous coding with file edit, shell, search tools |
| [05 — Model Arena](05-model-arena.md) | UI | Side-by-side model comparison and ranking |
| [06 — Cost Observatory](06-cost-observatory.md) | CLI + UI | Track token usage and costs across providers |

## Code Intelligence

| Demo | Surface | Description |
|------|---------|-------------|
| [07 — Inline Chat & Completions](07-inline-chat.md) | UI | Context-aware code suggestions in the editor |
| [08 — Code Search & Embeddings](08-code-search.md) | CLI + UI | Semantic search across codebases |
| [09 — Autofix & Diagnostics](09-autofix.md) | CLI + UI | Automated bug detection and repair |
| [10 — Code Transforms](10-code-transforms.md) | CLI + UI | AST-based refactoring and code generation |

## DevOps & Infrastructure

| Demo | Surface | Description |
|------|---------|-------------|
| [11 — Docker & Container Management](11-docker.md) | CLI + UI | Build, run, manage containers |
| [12 — Kubernetes Operations](12-kubernetes.md) | CLI + UI | Deploy, scale, monitor K8s workloads |
| [13 — CI/CD Pipeline](13-cicd.md) | CLI + UI | GitHub Actions, pipeline monitoring |
| [14 — Cloud Provider Integration](14-cloud-providers.md) | CLI + UI | AWS/GCP/Azure scanning, IAM, IaC generation |
| [15 — Deploy & Database](15-deploy-database.md) | CLI + UI | Deployment workflows and database management |

## MCP & Extensions

| Demo | Surface | Description |
|------|---------|-------------|
| [16 — MCP Server Integration](16-mcp-servers.md) | CLI + UI | Connect external tool servers |
| [17 — MCP Lazy Loading](17-mcp-lazy-loading.md) | CLI + UI | Scalable tool registry with on-demand loading |
| [18 — MCP Plugin Directory](18-mcp-directory.md) | UI | Browse, install, rate verified plugins |

## Collaboration & Context

| Demo | Surface | Description |
|------|---------|-------------|
| [19 — Context Bundles](19-context-bundles.md) | CLI + UI | Shareable context sets for teams |
| [20 — Agent Teams](20-agent-teams.md) | CLI + UI | Multi-agent collaboration with roles |
| [21 — CRDT Collaboration](21-crdt-collab.md) | UI | Real-time multi-user editing |
| [22 — Gateway Messaging](22-gateway.md) | CLI | AI assistant on 18 platforms (Slack, Discord, etc.) |

## Testing & Quality

| Demo | Surface | Description |
|------|---------|-------------|
| [23 — Test Runner & Coverage](23-test-coverage.md) | CLI + UI | Run tests, track coverage, generate tests |
| [24 — Red Team Security](24-red-team.md) | CLI + UI | Security scanning and vulnerability detection |
| [25 — SWE-bench Benchmarking](25-swe-bench.md) | CLI + UI | Benchmark AI coding performance |
| [26 — QA Validation Pipeline](26-qa-validation.md) | CLI + UI | Multi-round quality validation |

## Developer Tools

| Demo | Surface | Description |
|------|---------|-------------|
| [27 — HTTP Playground](27-http-playground.md) | CLI + UI | API testing with history and collections |
| [28 — GraphQL Explorer](28-graphql.md) | UI | Schema introspection and query building |
| [29 — Regex & Encoding Tools](29-regex-encoding.md) | UI | Regex tester, JWT decoder, base converter |
| [30 — Notebook & Scripts](30-notebook-scripts.md) | CLI + UI | Interactive notebooks and script runner |

## Enterprise & Advanced

| Demo | Surface | Description |
|------|---------|-------------|
| [31 — Batch Builder](31-batch-builder.md) | CLI + UI | Generate entire codebases (3M+ lines) |
| [32 — Legacy Migration](32-legacy-migration.md) | CLI + UI | COBOL/Fortran to modern languages |
| [33 — App Builder](33-app-builder.md) | CLI + UI | Full-stack app scaffolding from prompts |
| [34 — Usage Metering](34-usage-metering.md) | CLI + UI | Credit budgets and team cost allocation |
| [35 — Compliance & Audit](35-compliance.md) | CLI + UI | SOC 2 controls and audit trails |

---

## Running Demos

### CLI Demos

```bash
# Run a specific demo recording
vibecli demo run <demo-id>

# List available demos
vibecli demo list

# Generate a demo with AI
vibecli demo generate --feature "agent loop"

# Export demo as HTML slideshow
vibecli demo export <demo-id> --format html
```

### VibeUI Demos

1. Open VibeUI: `cd vibeui && npm run tauri dev`
2. Navigate to the **Demo** tab in the AI panel
3. Browse demos by category
4. Click **Play** to step through interactively

### Self-Hosted / Air-Gapped

```bash
# Run with Ollama (no internet required)
docker-compose up -d
vibecli --provider ollama chat "Hello"
```

---

## Demo JSON Format

Demos use VibeCody's `DemoRecording` format:

```json
{
  "id": "demo-agent-loop",
  "title": "Agent Loop & Tool Execution",
  "description": "Watch the AI agent autonomously edit files",
  "steps": [
    { "action": "Navigate", "target": "http://localhost:7878" },
    { "action": "Type", "target": "#prompt", "value": "Fix the bug in auth.rs" },
    { "action": "Narrate", "value": "The agent analyzes the codebase..." },
    { "action": "Screenshot", "label": "agent-thinking" },
    { "action": "Wait", "duration_ms": 2000 },
    { "action": "Assert", "target": ".tool-call", "value": "contains:EditFile" }
  ],
  "tags": ["agent", "tools", "coding"]
}
```

Step types: `Navigate`, `Click`, `Type`, `Wait`, `Screenshot`, `Assert`, `Narrate`, `EvalJs`, `Scroll`, `WaitForSelector`
