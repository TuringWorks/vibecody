---
layout: page
title: VibeCody Feature Demos
permalink: /demos/
---

Interactive walkthroughs covering VibeCody's major capabilities across CLI, Desktop IDE, and API surfaces.

---

## Getting Started

| Demo | Surface | Description |
|------|---------|-------------|
| [01 — First Run & Setup](first-run/) | CLI | Install, configure providers, first AI chat |
| [02 — TUI Interface](tui-interface/) | CLI | Navigate the terminal UI, panels, keybindings |

## AI & Providers

| Demo | Surface | Description |
|------|---------|-------------|
| [03 — Multi-Provider AI Chat](multi-provider-chat/) | CLI + UI | Switch between 22 AI providers, streaming responses |
| [04 — Agent Loop & Tool Execution](agent-loop/) | CLI + UI | Autonomous coding with file edit, shell, search tools |
| [05 — Model Arena](model-arena/) | UI | Side-by-side model comparison and ranking |
| [06 — Cost Observatory](cost-observatory/) | CLI + UI | Track token usage and costs across providers |

## Code Intelligence

| Demo | Surface | Description |
|------|---------|-------------|
| [07 — Inline Chat & Completions](07-inline-chat/) | UI | Context-aware code suggestions in the editor |
| [08 — Code Search & Embeddings](08-code-search/) | CLI + UI | Semantic search across codebases |
| [09 — Autofix & Diagnostics](09-autofix/) | CLI + UI | Automated bug detection and repair |
| [10 — Code Transforms](10-code-transforms/) | CLI + UI | AST-based refactoring and code generation |

## DevOps & Infrastructure

| Demo | Surface | Description |
|------|---------|-------------|
| [11 — Docker & Container Management](11-docker/) | CLI + UI | Build, run, manage containers |
| [12 — Kubernetes Operations](12-kubernetes/) | CLI + UI | Deploy, scale, monitor K8s workloads |
| [13 — CI/CD Pipeline](13-cicd/) | CLI + UI | GitHub Actions, pipeline monitoring |
| [14 — Cloud Provider Integration](14-cloud-providers/) | CLI + UI | AWS/GCP/Azure scanning, IAM, IaC generation |
| [15 — Deploy & Database](15-deploy-database/) | CLI + UI | Deployment workflows and database management |

## MCP & Extensions

| Demo | Surface | Description |
|------|---------|-------------|
| [16 — MCP Server Integration](16-mcp-servers/) | CLI + UI | Connect external tool servers |
| [17 — MCP Lazy Loading](17-mcp-lazy-loading/) | CLI + UI | Scalable tool registry with on-demand loading |
| [18 — MCP Plugin Directory](18-mcp-directory/) | UI | Browse, install, rate verified plugins |

## Collaboration & Context

| Demo | Surface | Description |
|------|---------|-------------|
| [19 — Context Bundles](context-bundles/) | CLI + UI | Shareable context sets for teams |
| [20 — Agent Teams](agent-teams/) | CLI + UI | Multi-agent collaboration with roles |
| [21 — CRDT Collaboration](crdt-collab/) | UI | Real-time multi-user editing |
| [22 — Gateway Messaging](gateway/) | CLI | AI assistant on 18 platforms (Slack, Discord, etc.) |

## Testing & Quality

| Demo | Surface | Description |
|------|---------|-------------|
| [23 — Test Runner & Coverage](test-coverage/) | CLI + UI | Run tests, track coverage, generate tests |
| [24 — Red Team Security](red-team/) | CLI + UI | Security scanning and vulnerability detection |
| [25 — SWE-bench Benchmarking](swe-bench/) | CLI + UI | Benchmark AI coding performance |
| [26 — QA Validation Pipeline](qa-validation/) | CLI + UI | Multi-round quality validation |

## Developer Tools

| Demo | Surface | Description |
|------|---------|-------------|
| [27 — HTTP Playground](http-playground/) | CLI + UI | API testing with history and collections |
| [28 — GraphQL Explorer](graphql/) | UI | Schema introspection and query building |
| [29 — Regex & Encoding Tools](regex-encoding/) | UI | Regex tester, JWT decoder, base converter |
| [30 — Notebook & Scripts](notebook-scripts/) | CLI + UI | Interactive notebooks and script runner |

## Enterprise & Advanced

| Demo | Surface | Description |
|------|---------|-------------|
| [31 — Batch Builder](31-batch-builder/) | CLI + UI | Generate entire codebases (3M+ lines) |
| [32 — Legacy Migration](32-legacy-migration/) | CLI + UI | COBOL/Fortran to modern languages |
| [33 — App Builder](33-app-builder/) | CLI + UI | Full-stack app scaffolding from prompts |
| [34 — Usage Metering](34-usage-metering/) | CLI + UI | Credit budgets and team cost allocation |
| [35 — Compliance & Audit](35-compliance/) | CLI + UI | SOC 2 controls and audit trails |

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

1. Open VibeUI: `cd vibeui && npm run tauri:dev`
2. Navigate to the **Demo** tab in the AI panel
3. Browse demos by category
4. Click **Play** to step through interactively

### Self-Hosted / Air-Gapped

```bash
# Run with Ollama (no internet required)
docker-compose up -d
vibecli --provider ollama chat "Hello"
```
