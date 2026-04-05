---
layout: page
title: Use Cases
permalink: /use-cases/
nav_order: 3
---


# VibeCody Use Cases

VibeCody is an AI-powered coding assistant that runs **anywhere you need it** -- on your laptop, in the cloud, or on a Raspberry Pi. Deploy it as a desktop app (VibeUI), a terminal companion (VibeCLI), or an always-on server (`vibecli --serve --port 7878`) that monitors your infrastructure, responds to webhooks, and runs scheduled tasks around the clock.

With **23 AI providers** (from local Ollama to Claude, OpenAI, Gemini, and 20 more), **106+ REPL commands**, **556+ skill files**, and a full autonomous agent loop, VibeCody covers the entire software development lifecycle -- from writing the first line of code to deploying, monitoring, and securing production systems. It also connects to **Gmail/Outlook, Google/Outlook Calendar, Todoist, Notion, Jira, and Home Assistant** for productivity workflows beyond coding.

See the [Configuration Guide](/configuration/) for setup and the [Provider Guide](/providers/) for connecting your preferred AI backend.

---

## Table of Contents

1. [Developer Workflows](#1-developer-workflows)
2. [Always-On Automation](#2-always-on-automation)
3. [DevOps and Infrastructure](#3-devops-and-infrastructure)
4. [Productivity and Knowledge](#4-productivity-and-knowledge)
5. [Team Collaboration](#5-team-collaboration)
6. [Security and Compliance](#6-security-and-compliance)
7. [Data and Analytics](#7-data-and-analytics)
8. [IoT and Edge](#8-iot-and-edge)
9. [Enterprise](#9-enterprise)
10. [Creative and Personal](#10-creative-and-personal)

---

## 1. Developer Workflows

### **AI-Powered Code Review**

VibeCody's code review engine runs 7 detectors (security/OWASP, complexity, style, documentation, tests, duplication, architecture) with 8-linter aggregation and quality gates. Get a prioritized list of issues with one command.

```bash
/aireview --path src/ --detectors all --quality-gate strict
```

*Works on: all (cloud, desktop, Pi)*

---

### **Automated Test Generation**

Point the agent at a module and it generates unit tests, integration tests, and property-based tests with full coverage reporting. It reads your existing test patterns and matches the framework you already use.

```bash
/agent "Generate comprehensive tests for src/auth/login.rs targeting 90% coverage"
```

*Works on: all*

---

### **Intelligent Refactoring**

The intent-aware refactoring engine analyzes your code, proposes structural improvements, and applies them in a single pass. It understands design patterns, SOLID principles, and language idioms.

```bash
/intent-refactor src/handlers/ --strategy extract-method --dry-run
```

*Works on: all*

---

### **Pull Request Automation**

Generate PR descriptions with Mermaid diagrams, run pre-submit reviews, and auto-fix linter warnings before pushing. The agent reads the full diff, summarizes changes, and flags risks.

```bash
/agent "Create a PR for branch feature/auth-refactor with a summary, test plan, and risk assessment"
```

*Works on: all*

---

### **Interactive Debugging**

Describe a bug in natural language. The agent reads stack traces, searches for related code, inserts diagnostic logging, runs the failing test, and proposes a fix -- all in an autonomous loop with human approval gates.

```bash
/agent "The /api/users endpoint returns 500 when the email contains a plus sign. Find and fix the bug."
```

*Works on: all*

---

### **CI/CD Pipeline Management**

Monitor GitHub Actions runs, view build logs, trigger workflows, and debug failing pipelines from VibeCLI or VibeUI. The GH Actions agent can diagnose flaky tests and suggest fixes.

```bash
/cicd status --repo myorg/myapp
/cicd logs --run 12345 --job build
/cicd trigger --workflow release.yml --ref main
```

*Works on: all*

---

### **Deployment Workflows**

Deploy to cloud providers, Kubernetes clusters, or bare-metal servers with AI-assisted rollback detection. VibeCody generates deployment manifests, runs pre-flight checks, and monitors health after deploy.

```bash
/deploy --target k8s --namespace production --image myapp:v2.1.0 --strategy rolling
```

*Works on: cloud, desktop*

---

### **Git Workflow Automation**

AI-assisted commit messages, interactive rebase planning, branch management, and conflict resolution. VibeCody reads the diff and writes conventional-commit-style messages.

```bash
/gitflow commit --conventional
/gitflow rebase-plan main..feature/auth
```

*Works on: all*

---

### **Autofix for Linter and Compiler Errors**

Feed compiler errors or linter output to VibeCody and it applies fixes automatically, re-runs the check, and iterates until the build is clean.

```bash
/autofix --command "cargo clippy --workspace" --max-iterations 5
```

*Works on: all*

---

### **Code Replay and Explanation**

Step through a git history range and have VibeCody explain each change, why it was made, and how it affects the rest of the system. Ideal for onboarding or post-incident review.

```bash
/code-replay --range HEAD~10..HEAD --annotate
```

*Works on: all*

---

### **Scaffold New Projects**

Generate project scaffolding from templates -- REST APIs, CLIs, microservices, React apps, and more -- with CI config, Dockerfiles, and test harnesses included.

```bash
/scaffold --template rust-axum-api --name my-service --features auth,db,docker
```

*Works on: all*

---

## 2. Always-On Automation

### **Server Mode**

Run VibeCody as a persistent HTTP daemon that accepts tasks via REST API. Integrate it into webhooks, chatbots, or custom tooling.

```bash
vibecli --serve --port 7878 --provider ollama
curl -X POST http://localhost:7878/task -d '{"prompt": "Review the latest commit"}'
```

*Works on: cloud, desktop, Pi*

---

### **Scheduled Agent Tasks**

Define cron-style schedules for recurring tasks: nightly code quality scans, weekly dependency audits, or hourly health checks.

```bash
/schedule create --name "nightly-review" --cron "0 2 * * *" --command "/aireview --path src/"
/schedule list
```

*Works on: cloud, Pi*

---

### **Auto-Research**

VibeCody autonomously researches a topic, crawls documentation, and produces a structured report. Leave it running overnight and read the results in the morning.

```bash
/auto-research "Compare WebSocket libraries for Rust: tokio-tungstenite vs axum vs actix-web"
```

*Works on: all*

---

### **Health Monitoring**

Continuously monitor application health endpoints, log response times, and alert when services degrade. The AI can diagnose issues and suggest remediations.

```bash
/health-monitor --endpoints config/endpoints.json --interval 60s --alert slack
```

*Works on: cloud, Pi*

---

### **Webhook-Driven Pipelines**

Configure VibeCody's server mode to respond to GitHub webhooks: auto-review PRs on open, run security scans on push, or generate release notes on tag.

```bash
# In config.toml
[hooks.on_pr_opened]
command = "/aireview --pr ${PR_NUMBER}"

[hooks.on_push]
command = "/redteam --changed-files-only"
```

*Works on: cloud, Pi*

---

### **Dependency Update Monitoring**

Schedule periodic dependency audits that check for CVEs, outdated packages, and license compliance. VibeCody creates PRs for safe updates.

```bash
/schedule create --name "dep-audit" --cron "0 8 * * 1" --command "/agent 'Audit dependencies, update safe patches, create a PR'"
```

*Works on: cloud, Pi*

---

### **Log Analysis**

Point VibeCody at log files or streams and have it identify anomalies, correlate errors across services, and surface root causes.

```bash
/agent "Analyze /var/log/app/*.log from the last 24 hours. Find error patterns and suggest fixes."
```

*Works on: cloud, desktop, Pi*

---

### **Automated Documentation Sync**

Schedule doc generation from code comments, API schemas, and architecture decision records. Keep your docs site in sync with your codebase automatically.

```bash
/schedule create --name "doc-sync" --cron "0 6 * * *" --command "/agent 'Regenerate API docs from OpenAPI spec and update docs/ folder'"
```

*Works on: cloud, Pi*

---

### **Environment Drift Detection**

Compare staging and production configs on a schedule and alert when drift is detected. The agent generates a remediation plan.

```bash
/schedule create --name "drift-check" --cron "0 */4 * * *" --command "/env-diff staging production --alert"
```

*Works on: cloud, Pi*

---

## 3. DevOps and Infrastructure

### **Docker Container Management**

Build, run, inspect, and debug Docker containers directly from VibeCLI. VibeCody generates Dockerfiles, docker-compose configs, and multi-stage builds optimized for your stack.

```bash
/docker build --context . --target production --optimize
/docker compose up --detach
/docker logs my-service --tail 100 --analyze
```

*Works on: cloud, desktop*

---

### **Kubernetes Operations**

Deploy, scale, and troubleshoot Kubernetes workloads. VibeCody reads cluster state, generates manifests, and applies rolling updates.

```bash
/k8s status --namespace production
/k8s deploy --manifest k8s/deployment.yaml --dry-run
/k8s debug pod/my-app-7d4f8b --logs --events
```

*Works on: cloud, desktop*

---

### **Cloud Provider Scanning**

Scan your AWS, GCP, or Azure accounts for misconfigured resources, cost optimization opportunities, and security issues.

```bash
/cloud scan --provider aws --checks security,cost,compliance
/cloud report --format markdown
```

*Works on: cloud, desktop*

---

### **Infrastructure as Code Generation**

Describe your desired infrastructure in natural language and VibeCody generates Terraform, Pulumi, CloudFormation, or CDK code.

```bash
/agent "Generate Terraform for a 3-tier AWS architecture: ALB, ECS Fargate, Aurora PostgreSQL, with VPC and security groups"
```

*Works on: all*

---

### **Cost Estimation**

Estimate cloud costs before deploying. VibeCody analyzes your IaC files and produces a monthly cost breakdown per resource.

```bash
/cloud cost-estimate --terraform infra/ --region us-east-1
```

*Works on: all*

---

### **Network Diagnostics**

Run DNS lookups, traceroutes, port scans, and TLS certificate checks from VibeCody. The AI interprets results and suggests fixes.

```bash
/network diagnose --target api.example.com --checks dns,tls,latency,ports
```

*Works on: cloud, desktop, Pi*

---

### **SSH Session Management**

Connect to remote servers, run commands, and transfer files through VibeCody's SSH integration. The agent can troubleshoot remote issues interactively.

```bash
/ssh connect production-web-01 --command "systemctl status nginx"
/ssh exec-all --group web-servers --command "df -h"
```

*Works on: cloud, desktop*

---

### **CI/CD Pipeline Generation**

Generate GitHub Actions, GitLab CI, or Jenkins pipelines from your project structure. VibeCody detects the language, test framework, and deployment target automatically.

```bash
/agent "Generate a GitHub Actions workflow for this Rust project: lint, test, build Docker image, deploy to ECS"
```

*Works on: all*

---

### **Container Sandbox Execution**

Run agent tasks in isolated Docker or Podman containers for safety. The sandbox supports 16 async operations with resource limits and network policies.

```bash
vibecli --sandbox docker --agent "Refactor the payment module"
```

*Works on: cloud, desktop*

---

### **Load Testing**

Generate and run load test scenarios against your APIs. VibeCody creates test scripts, executes them, and analyzes the results.

```bash
/loadtest --target http://localhost:8080/api --rps 500 --duration 60s --analyze
```

*Works on: cloud, desktop*

---

## 4. Productivity and Knowledge

### **Context Bundles**

Save and restore conversation contexts as reusable bundles. Share a bundle with teammates so they can pick up where you left off.

```bash
/context save auth-refactor "Context for the authentication rewrite project"
/context load auth-refactor
/context list
```

*Works on: all*

---

### **Open Memory**

VibeCody maintains an auto-updated memory of your project -- architecture decisions, coding patterns, team conventions -- that persists across sessions and improves suggestions over time.

```bash
/memory show
/memory add "We use snake_case for database columns and camelCase for API responses"
/memory search "authentication flow"
```

*Works on: all*

---

### **Session Management**

Create, resume, and branch sessions backed by SQLite. Each session preserves full conversation history, tool outputs, and agent state.

```bash
/session new --name "bug-1234"
/session resume bug-1234
/session list --last 10
/session branch bug-1234 --name "bug-1234-alt-approach"
```

*Works on: all*

---

### **AI-Assisted Documentation**

Generate README files, API docs, architecture decision records, and inline documentation from your codebase.

```bash
/agent "Generate API documentation for all endpoints in src/routes/ using OpenAPI 3.1 format"
```

*Works on: all*

---

### **Codebase Q&A**

Ask questions about your codebase in natural language. VibeCody searches files, reads relevant code, and answers with citations.

```bash
vibecli -c "How does the authentication middleware work? Which files are involved?"
```

*Works on: all*

---

### **Bookmark and Snippet Management**

Save code snippets, file locations, and search results as bookmarks for quick reference later.

```bash
/bookmark add src/auth/middleware.rs:42 "JWT validation logic"
/bookmark list --tag auth
/snippet save "retry-with-backoff" --file src/utils/retry.rs --lines 10-35
```

*Works on: all*

---

### **Architecture Decision Records**

Create and manage ADRs with lifecycle tracking, markdown generation, and governance rules powered by the TOGAF/Zachman engine.

```bash
/archspec adr create "Use PostgreSQL for user data" --status accepted --context "Evaluated Mongo, DynamoDB, Postgres"
/archspec adr list --status accepted
```

*Works on: all*

---

### **One-Shot Queries**

Get quick answers without entering the REPL. Pipe output from other tools for instant analysis.

```bash
vibecli -c "Explain what this regex does: ^(?:[a-z0-9!#$%&'*+/=?^_\`{|}~-]+\.)*"
cat error.log | vibecli -c "What caused this crash?"
```

*Works on: all*

---

### **Profile Switching**

Maintain separate profiles for different projects, teams, or contexts. Each profile has its own provider, model, system prompt, and rules.

```bash
vibecli --profile work
vibecli --profile personal
/profile list
/profile create oss --provider ollama --model codellama:34b
```

*Works on: all*

---

### **Email Triage and Management**

Connect Gmail or Outlook and manage email from the terminal. The AI triage mode classifies messages as urgent, action-needed, or FYI and suggests responses.

```bash
# Morning briefing
/email unread           # 14 unread messages
/email triage           # AI classifies + archives FYI messages

# Send a reply
/email read <id>
/email send alice@co.com "Re: deploy plan" "Looks good — approved"
```

Configure in `~/.vibecli/config.toml` under `[email]` or set `GMAIL_ACCESS_TOKEN` / `OUTLOOK_ACCESS_TOKEN`.

*Works on: all*

---

### **Calendar and Scheduling**

View and manage Google Calendar or Outlook Calendar events. Find free slots before proposing meetings.

```bash
/cal today              # Today's events with times
/cal free tomorrow      # Open slots tomorrow
/cal create "Sprint review" "friday 2pm" "friday 3pm"
/cal next               # Next upcoming event
```

*Works on: all*

---

### **Task Management with Todoist**

Add, complete, and triage tasks without leaving the terminal. Integrates with Jira for cross-system tracking.

```bash
/todo today             # Tasks due today + overdue
/todo add "Fix login bug" due:today p1
/todo close 1234567     # Mark done
/todo project "Work"    # View project tasks
```

*Works on: all*

---

### **Notion Knowledge Base**

Search, read, and append to Notion pages without opening a browser. Useful for logging meeting notes and cross-referencing docs during development.

```bash
/notion search "API design decisions"
/notion get abc123def456    # Read page content
/notion append abc123def456 "2026-04-04: switched to REST from GraphQL"
```

*Works on: all*

---

### **Jira Issue Tracking**

Create tickets, update statuses, and add comments from the terminal. Works with Jira Cloud and Server.

```bash
/jira mine              # My open issues
/jira create PROJ "Null pointer in UserService" "Stack: ..."
/jira transition PROJ-234 "In Review"
/jira comment PROJ-234 "Root cause found — missing null check on line 47"
```

*Works on: all*

---

### **Smart Home Control via Home Assistant**

Control lights, thermostats, scenes, and automations from the terminal. Run focus or wind-down routines as part of your work workflow.

```bash
/ha scene focus         # Dim lights, set temperature for deep work
/ha status              # Full home state summary
/ha off all lights      # Lights out
/ha climate thermostat.main 72
```

Requires Home Assistant with a long-lived access token. Works locally or via Tailscale/Nabu Casa for remote access.

*Works on: all*

---

### **Morning Briefing Sequence**

Run a complete morning briefing in four commands — no browser tabs needed.

```bash
/email unread     # Overnight emails
/cal today        # Today's meetings
/todo today       # Tasks due today
/jira mine        # Open Jira issues
```

*Works on: all*

---

## 5. Team Collaboration

### **Agent Teams**

Spin up multi-agent teams with specialized roles (Architect, Coder, Reviewer, Tester, SecurityAuditor) that collaborate on tasks through an inter-agent messaging bus.

```bash
/team create feature-auth --roles architect,coder,reviewer,tester
/team run feature-auth "Design and implement OAuth2 PKCE flow for the mobile app"
```

*Works on: cloud, desktop*

---

### **CRDT Collaborative Editing**

Multiple users can edit the same file simultaneously with conflict-free resolution using CRDTs. Changes are synchronized in real time over Tailscale or local networks.

```bash
/collab start --file src/main.rs --peers alice@tailscale,bob@tailscale
/collab status
```

*Works on: cloud, desktop*

---

### **Gateway Messaging (18 Platforms)**

Connect VibeCody to Slack, Discord, Teams, Telegram, WhatsApp, and 13 other platforms. Team members can interact with VibeCody from their preferred messaging tool.

```bash
vibecli --serve --port 7878 --gateway slack,discord,teams
/gateway status
/gateway send slack "#engineering" "Deploy v2.1 completed successfully"
```

*Works on: cloud, Pi*

---

### **Multi-Agent Orchestration**

Coordinate multiple agents working on different parts of a codebase. The orchestrator handles task decomposition, dependency resolution, and result synthesis.

```bash
/orchestrate "Refactor the payment system" --agents 4 --strategy divide-and-conquer
```

*Works on: cloud, desktop*

---

### **Shared Agent Memory**

Agent teams share a memory store so discoveries by one agent (e.g., a security flaw found by the auditor) are immediately available to all other agents on the team.

```bash
/team memory show feature-auth
/team memory add feature-auth "The existing session table uses UUID v4 primary keys"
```

*Works on: cloud, desktop*

---

### **Voice Pairing via Tailscale**

Pair-program with VibeCody using voice commands over a Tailscale connection. Speak your intent, and VibeCody writes the code.

```bash
/voice start --tailscale --device my-laptop
/voice language en-US
# "Add error handling to the upload function"
```

*Works on: desktop, Pi*

---

### **Code Review Protocol**

Define team review standards as machine-readable protocols. VibeCody enforces them automatically on every PR, ensuring consistent quality.

```bash
/review-protocol load team-standards.yaml
/review-protocol run --pr 456 --strict
```

*Works on: all*

---

### **Linear and Jira Integration**

Create, update, and track issues directly from VibeCLI. The agent can read issue descriptions and automatically start working on them.

```bash
/linear list --status "In Progress" --assignee me
/agent "Pick up LINEAR-1234 and implement the feature described in the ticket"
```

*Works on: all*

---

## 6. Security and Compliance

### **Red Team Security Testing**

Run automated adversarial security scans against your codebase. VibeCody probes for OWASP Top 10 vulnerabilities, path traversal, SQL injection, XSS, and insecure deserialization.

```bash
/redteam scan --path src/ --checks owasp-top-10 --severity high,critical
/redteam report --format sarif
```

*Works on: all*

---

### **OWASP Vulnerability Scanning**

Targeted scanning for OWASP categories with remediation suggestions that the agent can apply automatically.

```bash
/redteam owasp --category injection,xss,auth-failure --autofix
```

*Works on: all*

---

### **Policy Engine**

Define RBAC/ABAC policies in YAML (Cerbos-style) with 14 condition operators, derived roles, conflict detection, and coverage analysis. Enforce who can do what across your system.

```bash
/policy load policies/
/policy test --suite policy-tests.yaml
/policy audit --last 24h
```

*Works on: all*

---

### **Architecture Governance**

Enforce architectural boundaries with a TOGAF/Zachman governance engine. Detect violations like circular dependencies, layer breaches, and unauthorized direct database access from controllers.

```bash
/archspec governance check --rules arch-rules.yaml
/archspec c4 generate --level component --output docs/architecture.md
```

*Works on: all*

---

### **Dependency Vulnerability Audit**

Scan dependencies across Rust (cargo-audit), Node (npm audit), Python (pip-audit), and more. VibeCody correlates CVEs with your actual usage to eliminate false positives.

```bash
/agent "Audit all dependencies for known vulnerabilities. For each CVE, check if our code actually calls the affected function."
```

*Works on: all*

---

### **Secret Detection**

Scan your codebase and git history for leaked secrets: API keys, passwords, tokens, and private keys. VibeCody identifies the commit, author, and suggests rotation steps.

```bash
/redteam secrets --scan-history --depth 100
```

*Works on: all*

---

### **Compliance Reporting**

Generate compliance reports for SOC2, HIPAA, PCI-DSS, and GDPR. VibeCody maps your codebase controls to compliance frameworks and identifies gaps.

```bash
/policy compliance --framework soc2 --output compliance-report.md
```

*Works on: all*

---

### **Blue/Purple Team Exercises**

Run coordinated attack-and-defend exercises where one agent team attacks and another defends. Results are scored and reported.

```bash
/purpleteam run --attack-surface api --duration 30m --report
```

*Works on: cloud, desktop*

---

### **Audit Trail**

Every agent action is logged to JSONL trace files with context sidecars. Full audit trail for compliance and post-incident review.

```bash
/trace list --last 7d
/trace show 2026-04-01T14-30-00 --include-context
/trace export --format csv --output audit.csv
```

*Works on: all*

---

## 7. Data and Analytics

### **Database Management**

Connect to PostgreSQL, MySQL, SQLite, MongoDB, Redis, and DuckDB. Run queries, inspect schemas, generate migrations, and optimize slow queries with AI assistance.

```bash
/db connect postgres://user:pass@localhost:5432/mydb
/db schema --table users
/db query "SELECT * FROM orders WHERE status = 'pending' LIMIT 10"
/db optimize --slow-query-log
```

*Works on: all*

---

### **Cost Observatory**

Track AI token usage, API costs, and cloud spend across all 23 providers. Set budgets and alerts to prevent surprise bills.

```bash
/cost show --period this-month --by-provider
/cost budget set --monthly 50.00 --alert-at 80%
/cost export --format csv
```

*Works on: all*

---

### **Embeddings and Semantic Search**

Generate embeddings for your codebase and use semantic search to find code by meaning rather than keywords. Powered by optimized O(n) updates and fused cosine similarity.

```bash
/embeddings index --path src/
/embeddings search "function that handles user authentication with JWT"
```

*Works on: all*

---

### **Metrics Collection and Visualization**

Collect application metrics, analyze trends, and generate dashboards. VibeCody integrates with Prometheus, Grafana, and custom metric sources.

```bash
/metrics collect --source prometheus --query 'http_requests_total{status="500"}'
/metrics analyze --period 7d --anomaly-detection
```

*Works on: cloud, desktop*

---

### **Database Migration Generation**

Describe schema changes in natural language and VibeCody generates migration files compatible with your ORM (Diesel, SQLx, Prisma, Alembic, etc.).

```bash
/agent "Add a 'teams' table with id, name, created_at. Add team_id foreign key to users. Generate a Diesel migration."
```

*Works on: all*

---

### **GraphQL Schema Management**

Generate, validate, and evolve GraphQL schemas. VibeCody creates resolvers, types, and mutations from your database schema.

```bash
/graphql generate --from-db postgres://localhost/mydb --output schema.graphql
/graphql validate schema.graphql
```

*Works on: all*

---

### **API Performance Analysis**

Analyze API response times, error rates, and throughput from logs or monitoring data. VibeCody identifies bottlenecks and suggests optimizations.

```bash
/agent "Analyze the nginx access logs for the last 24 hours. Which endpoints are slowest? What is the p99 latency?"
```

*Works on: all*

---

### **Data Pipeline Debugging**

Trace data through ETL pipelines, identify where transformations fail, and suggest fixes. VibeCody reads pipeline configs and log outputs.

```bash
/agent "The daily user_events ETL pipeline failed at the transform step. Diagnose from the Airflow logs and fix the SQL."
```

*Works on: cloud, desktop*

---

## 8. IoT and Edge

### **Raspberry Pi Deployment**

VibeCody compiles to a single ARM binary that runs on Raspberry Pi. Use it as a local AI assistant, home automation controller, or edge compute node.

```bash
# Install on Raspberry Pi
curl -fsSL https://vibecody.dev/install.sh | sh
vibecli --provider ollama --model tinyllama:1.1b
```

*Works on: Pi*

---

### **Smart Home Automation via MCP**

Connect to smart home devices through MCP (Model Context Protocol) servers. Control lights, thermostats, sensors, and appliances with natural language.

```bash
# Connect to Home Assistant MCP server
/mcp connect homeassistant --url http://homeassistant.local:8123/mcp

# Natural language control
/agent "Turn off all lights except the living room. Set thermostat to 68F."
```

*Works on: Pi, desktop*

---

### **Edge Sensor Monitoring**

Read sensor data from GPIO, I2C, or MQTT sources. VibeCody analyzes patterns and triggers alerts when thresholds are exceeded.

```bash
/agent "Monitor the MQTT topic sensors/temperature. Alert me if it exceeds 85F for more than 5 minutes."
```

*Works on: Pi*

---

### **Local-Only AI Processing**

Run entirely offline with Ollama and a local model. No data leaves your device. Ideal for air-gapped environments and privacy-sensitive workloads.

```bash
vibecli --provider ollama --model codellama:13b --offline
```

*Works on: Pi, desktop*

---

### **Edge Deployment Orchestration**

Deploy and update applications across a fleet of edge devices. VibeCody generates OTA update manifests and manages rollback strategies.

```bash
/agent "Generate a Balena fleet deployment config for our sensor-reader app targeting RPi4 devices"
```

*Works on: cloud, Pi*

---

### **Voice-Controlled Development**

Use voice commands to write code, run tests, and manage deployments hands-free. Useful for hardware prototyping when your hands are occupied.

```bash
/voice start --provider whisper --device default
# "Run the test suite and tell me if anything failed"
```

*Works on: Pi, desktop*

---

### **MCP Server Integration**

Connect to any MCP-compatible tool server to extend VibeCody's capabilities. Browse the MCP directory and lazy-load servers on demand.

```bash
/mcp directory search "database"
/mcp connect sqlite --lazy
/mcp list
```

*Works on: all*

---

### **Kiosk and Display Mode**

Run VibeUI in kiosk mode on a dedicated display for dashboards, monitoring, or team status boards.

```bash
vibeui --kiosk --panel cost-observatory --refresh 60s
```

*Works on: Pi, desktop*

---

## 9. Enterprise

### **Multi-Provider Failover**

Configure automatic failover between providers. If Claude is down, VibeCody seamlessly switches to OpenAI, then Gemini, with no interruption.

```bash
# In config.toml
[failover]
chain = ["claude", "openai", "gemini", "ollama"]
timeout_ms = 5000
retry_count = 2
```

*Works on: all*

---

### **Bring Your Own Key (BYOK)**

Every team member uses their own API keys. No shared keys, no central billing proxy. Keys can be stored in vaults, env vars, or helper scripts.

```bash
# Key from a vault helper
[claude]
api_key_helper = "vault read -field=key secret/anthropic"
```

*Works on: all*

---

### **Role-Based Access Control**

Define granular permissions for who can run which commands, access which tools, and modify which files using the policy engine.

```bash
/policy create --role junior-dev --allow "read,test,lint" --deny "deploy,delete,db-write"
/policy assign --user alice --role junior-dev
```

*Works on: all*

---

### **Audit and Compliance Logging**

Every agent interaction produces JSONL traces with full context. Export to SIEM systems for enterprise compliance requirements.

```bash
/trace export --format siem --destination splunk://logs.corp.com:8088
```

*Works on: all*

---

### **Gateway for 18 Messaging Platforms**

Deploy VibeCody as a shared team resource accessible from Slack, Teams, Discord, Telegram, and 14 more platforms. Centralized AI assistant for the entire organization.

```bash
vibecli --serve --port 7878 --gateway all --auth-mode oauth2
```

*Works on: cloud*

---

### **Profile-Based Multi-Tenancy**

Separate configurations for different teams, projects, or environments. Each profile has isolated sessions, memory, and provider settings.

```bash
/profile create team-backend --provider claude --model claude-opus-4-6 --rules strict
/profile create team-frontend --provider openai --model gpt-4o --rules relaxed
```

*Works on: all*

---

### **Self-Hosted Deployment**

Run VibeCody entirely on your own infrastructure. No SaaS dependency, no data exfiltration risk. Ship the single binary or use the Docker image.

```bash
docker run -d -p 7878:7878 -v /data:/data vibecody/vibecli:latest --serve
```

*Works on: cloud*

---

### **Custom Rules and Guardrails**

Define project-specific rules that the agent must follow: coding standards, forbidden patterns, required review steps, and more.

```bash
# .vibecli/rules.toml
[[rules]]
name = "no-unwrap-in-prod"
pattern = "unwrap()"
action = "block"
message = "Use proper error handling instead of unwrap() in production code"
```

*Works on: all*

---

### **SSO and Token Management**

Integrate with enterprise SSO providers for authentication. API token rotation, expiry tracking, and BYOK management are built in.

```bash
/tokens list --provider all
/tokens rotate --provider openai --confirm
/tokens usage --period this-month
```

*Works on: all*

---

## 10. Creative and Personal

### **Model Arena**

Compare responses from multiple AI providers side-by-side. Run the same prompt through Claude, GPT-4o, Gemini, and local models to find the best answer.

```bash
/arena --providers claude,openai,gemini,ollama --prompt "Write a retry function with exponential backoff in Rust"
```

*Works on: all*

---

### **Inline Chat Editing**

Edit code interactively in your terminal. Select a range of lines and describe the change you want. VibeCody applies the edit in place.

```bash
/inline src/main.rs:42-58 "Add proper error handling with anyhow context"
```

*Works on: all*

---

### **One-Shot Chat**

Quick answers without entering the REPL. Great for shell aliases, editor integrations, and scripts.

```bash
vibecli -c "What is the time complexity of Rust's BTreeMap::insert?"
echo "SELECT * FORM users" | vibecli -c "Fix the SQL syntax error"
```

*Works on: all*

---

### **Voice Pair Programming**

Speak naturally to VibeCody while coding. It listens, understands context from your open files, and writes code or runs commands based on your voice input.

```bash
/voice start --continuous
# "Add a health check endpoint that returns the git commit hash and uptime"
```

*Works on: desktop, Pi*

---

### **Personal Knowledge Base**

Use VibeCody as a persistent personal knowledge assistant. Store notes, code snippets, and research across sessions with full semantic search.

```bash
/memory add "Rust lifetimes: &'a means the reference lives at least as long as 'a"
/memory search "lifetimes"
/snippet save "tokio-spawn-pattern" --from-clipboard
```

*Works on: all*

---

### **Learning and Exploration**

Ask VibeCody to explain unfamiliar codebases, libraries, or concepts. It reads source code, documentation, and examples to teach you.

```bash
/agent "I am new to this project. Walk me through the architecture, entry points, and key data flows."
```

*Works on: all*

---

### **Blog and Technical Writing**

Generate technical blog posts, tutorials, and documentation from your code and commit history.

```bash
/agent "Write a blog post explaining how our new authentication system works, with code examples from the actual implementation"
```

*Works on: all*

---

### **Git Bisect Assistance**

VibeCody automates git bisect by running test commands and binary-searching for the commit that introduced a bug.

```bash
/gitflow bisect --good v2.0.0 --bad HEAD --test "cargo test auth_tests"
```

*Works on: all*

---

### **HTTP Playground**

Test APIs interactively with an AI-assisted HTTP client. VibeCody reads your API schema and generates request examples.

```bash
/http GET http://localhost:8080/api/users --auth bearer
/http POST http://localhost:8080/api/users --body '{"name":"Alice"}' --analyze-response
```

*Works on: all*

---

### **Speculative Execution**

VibeCody predicts what you will ask next and pre-computes likely responses using TurboQuant KV-cache. Results appear instantly when you confirm.

```bash
/speculative on
# VibeCody pre-generates the next likely edit after each interaction
```

*Works on: desktop*

---

---

## Comparison: VibeCody vs myclaw.ai / OpenClaw

| Feature | VibeCody | myclaw.ai / OpenClaw |
|---------|----------|---------------------|
| **Pricing** | Free and open source (MIT) | Monthly subscription |
| **AI Providers** | 23 providers (local + cloud) | Limited provider selection |
| **Self-Hosted** | Single binary, Docker, or Pi | Cloud-only SaaS |
| **Runs on Raspberry Pi** | Yes (ARM binary, 2GB RAM) | No |
| **Offline Mode** | Full functionality with Ollama | Requires internet |
| **Agent Loop** | Autonomous with tool calling, checkpoints, rollback | Basic agent capabilities |
| **Code Review** | 7 detectors, 8-linter aggregation, quality gates | Basic review |
| **Multi-Agent Teams** | 5 specialized roles with shared memory | Single agent |
| **MCP Integration** | Full MCP server support with lazy loading | No MCP support |
| **Security Scanning** | Red team, OWASP, blue/purple team, secret detection | Basic scanning |
| **Desktop App** | VibeUI with 196+ panels (Tauri + React) | Web-only interface |
| **REPL Commands** | 106+ commands with subcommands | Limited CLI |
| **Skill Library** | 550+ skill files | No skill system |
| **Messaging Gateway** | 18 platforms (Slack, Teams, Discord, etc.) | Slack only |
| **Database Support** | 6 databases (Postgres, MySQL, SQLite, Mongo, Redis, DuckDB) | None |
| **Collaborative Editing** | CRDT-based real-time sync | No collaboration |
| **Voice Control** | Voice pairing via Tailscale | No voice support |
| **Policy Engine** | RBAC/ABAC with 14 operators, audit trail | No policy engine |
| **Architecture Governance** | TOGAF/Zachman, C4 diagrams, ADRs | No architecture tools |
| **Cost Tracking** | Built-in cost observatory with budgets | No cost visibility |
| **Always-On Server** | HTTP daemon with webhook support | No server mode |
| **Container Sandbox** | Docker/Podman with resource limits | No sandboxing |
| **Provider Failover** | Automatic chain failover across providers | No failover |
| **BYOK** | Full bring-your-own-key with vault integration | Vendor-locked keys |
| **Audit Trail** | JSONL traces with context sidecars | No audit trail |
| **Data Ownership** | 100% self-hosted, your data stays on your machines | Vendor-hosted data |

---

## Getting Started

Ready to try these use cases? Start here:

1. **Easy Setup:** Run `vibecli --setup` or see the [Easy Setup Guide](/vibecody/setup/)
2. **Install VibeCody:** `curl -fsSL https://raw.githubusercontent.com/TuringWorks/vibecody/main/install.sh | sh`
3. **Configure a provider:** See the [Provider Guide](/vibecody/providers/)
4. **Run your first agent task:** `vibecli --agent "Hello, review my project structure"`
5. **Explore REPL commands:** Type `/help` in the REPL
6. **Deploy anywhere:** See the [Deployment Guides](/vibecody/guides/) for 12 platforms (AWS, GCP, Azure, Pi, etc.)
