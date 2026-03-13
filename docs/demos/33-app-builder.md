---
layout: page
title: "Demo 33: App Builder"
permalink: /demos/33-app-builder/
---

# Demo 33: App Builder

## Overview

The App Builder lets you describe an application in natural language and receive a fully scaffolded, AI-enhanced project. It combines a template library, an AI-powered scaffolder, and managed backend provisioning into a single workflow. Use the CLI or the 4-tab VibeUI panel to go from idea to running code in minutes.

## Prerequisites

- VibeCLI installed and on your PATH
- At least one AI provider configured (e.g., `ANTHROPIC_API_KEY` or `OPENAI_API_KEY`)
- Node.js and/or Rust toolchain (depending on the template you choose)
- For VibeUI: the desktop app running with the **App Builder** panel visible

## Step-by-Step Walkthrough

### 1. Quick Start: Natural Language to Scaffold

Describe your application in plain English and let the AI generate the full project structure.

**CLI:**

```bash
vibecli appbuilder quickstart "A task management API with user auth, team workspaces, and real-time notifications"
```

Example output:

```
App Builder: Quick Start
Analyzing description...
Detected components:
  - REST API (task CRUD, team CRUD)
  - Authentication (JWT + refresh tokens)
  - Authorization (team-based RBAC)
  - WebSocket server (real-time notifications)
  - Database (PostgreSQL, 6 tables)

Generating scaffold...
Created: task-manager-api/
  src/
    main.rs
    routes/
      tasks.rs, teams.rs, users.rs, auth.rs, notifications.rs
    models/
      task.rs, team.rs, user.rs, notification.rs
    middleware/
      auth.rs, rbac.rs
    db/
      schema.rs, migrations/
    ws/
      handler.rs
  Cargo.toml
  .env.example
  docker-compose.yml
  README.md

Files: 18 | Lines: 2,340
```

**VibeUI:**

Open the **App Builder** panel and select the **Quick Start** tab. Type your description into the text area and click **Generate**. The scaffolded project appears in the file tree.

### 2. Browse the Template Library

Choose from pre-built templates for common application types.

**CLI:**

```bash
vibecli appbuilder templates list
```

Example output:

```
Available Templates:
  web-app          Full-stack web application (React + API)
  rest-api         REST API with OpenAPI spec
  graphql-api      GraphQL server with schema-first design
  cli-tool         Command-line tool with argument parsing
  mobile-app       Cross-platform mobile app (React Native)
  desktop-app      Desktop application (Tauri + React)
  microservice     Single microservice with health checks
  monorepo         Multi-package monorepo setup
  static-site      Static site with SSG
  chrome-ext       Browser extension scaffold
```

**CLI (use a template):**

```bash
vibecli appbuilder create --template rest-api --name my-api --language rust
```

**VibeUI:**

Switch to the **Templates** tab to browse templates as cards with descriptions, tech stacks, and preview screenshots. Click **Use Template** to scaffold.

### 3. AI-Enhanced Scaffolding

After the initial scaffold, the AI enhancer reviews the generated code and adds production-ready improvements: error handling, logging, input validation, tests, and documentation.

**CLI:**

```bash
vibecli appbuilder enhance ./task-manager-api
```

Example output:

```
AI Enhancement Pass:
  + Added structured error handling (12 files)
  + Added request validation middleware
  + Generated 34 unit tests
  + Added OpenAPI spec (openapi.yaml)
  + Added Dockerfile and CI workflow
  + Added rate limiting middleware
  + Improved logging with tracing spans
  Enhancement complete: 847 lines added across 19 files
```

**VibeUI:**

In the **Quick Start** or **Templates** tab, toggle **AI Enhance** before generating. The enhancement runs automatically after scaffolding.

### 4. Provision a Managed Backend

Set up the infrastructure for your application: database, object storage, authentication service, and hosting.

**CLI:**

```bash
vibecli appbuilder provision --project ./task-manager-api \
  --database postgres \
  --storage s3 \
  --auth supabase \
  --hosting fly-io
```

Example output:

```
Provisioning Backend:
  [1/4] PostgreSQL database... configured (connection string in .env)
  [2/4] S3-compatible storage... configured (bucket: task-manager-assets)
  [3/4] Supabase Auth... configured (project linked)
  [4/4] Fly.io deployment... configured (fly.toml generated)

Backend ready. Run `fly deploy` to go live.
```

**VibeUI:**

Switch to the **Backend** tab. Select your infrastructure components from dropdowns and click **Provision**. Connection strings and configuration are written to `.env` automatically.

### 5. Full-Stack Generation from a Prompt

Combine all steps into a single command for end-to-end generation.

**CLI:**

```bash
vibecli appbuilder fullstack "An e-commerce store with product catalog, shopping cart, Stripe payments, and admin dashboard" \
  --language typescript \
  --database postgres \
  --enhance
```

This runs Quick Start, AI Enhancement, and Backend Provisioning in sequence, producing a deployable full-stack application.

**VibeUI:**

In the **Quick Start** tab, check **Full-Stack Mode**, enter your description, select infrastructure options, and click **Generate Full Stack**.

### 6. Customize and Iterate

After generation, use VibeCLI's agent or VibeUI's chat panel to refine the scaffolded code. The App Builder output is standard code -- no lock-in or proprietary formats.

**CLI (REPL):**

```
vibecli
> /appbuilder status
Project: task-manager-api
Template: rest-api (enhanced)
Backend: PostgreSQL + S3 + Supabase Auth
Status: Ready for development
```

## Demo Recording JSON

```json
{
  "demo_id": "33-app-builder",
  "title": "App Builder",
  "version": "1.0.0",
  "steps": [
    {
      "action": "cli_command",
      "command": "vibecli appbuilder quickstart \"A task management API with user auth, team workspaces, and real-time notifications\"",
      "description": "Generate a full project scaffold from a natural language description"
    },
    {
      "action": "cli_command",
      "command": "vibecli appbuilder templates list",
      "description": "Browse available project templates"
    },
    {
      "action": "cli_command",
      "command": "vibecli appbuilder create --template rest-api --name my-api --language rust",
      "description": "Scaffold a project from a template"
    },
    {
      "action": "cli_command",
      "command": "vibecli appbuilder enhance ./task-manager-api",
      "description": "Run AI enhancement pass on the scaffold"
    },
    {
      "action": "cli_command",
      "command": "vibecli appbuilder provision --project ./task-manager-api --database postgres --storage s3 --auth supabase --hosting fly-io",
      "description": "Provision managed backend infrastructure"
    },
    {
      "action": "cli_command",
      "command": "vibecli appbuilder fullstack \"An e-commerce store with product catalog, shopping cart, Stripe payments, and admin dashboard\" --language typescript --database postgres --enhance",
      "description": "End-to-end full-stack generation from a single prompt"
    },
    {
      "action": "vibeui_interaction",
      "panel": "AppBuilder",
      "tab": "Quick Start",
      "description": "Generate a scaffold from natural language in the GUI"
    },
    {
      "action": "vibeui_interaction",
      "panel": "AppBuilder",
      "tab": "Templates",
      "description": "Browse and select from the template library"
    },
    {
      "action": "vibeui_interaction",
      "panel": "AppBuilder",
      "tab": "Provision",
      "description": "Configure and provision backend services"
    },
    {
      "action": "vibeui_interaction",
      "panel": "AppBuilder",
      "tab": "Backend",
      "description": "View and manage provisioned infrastructure"
    }
  ]
}
```

## What's Next

- [Demo 31: Batch Builder](../31-batch-builder/) -- Scale up to multi-million line generation with Batch Builder
- [Demo 32: Legacy Migration](../32-legacy-migration/) -- Migrate existing legacy codebases to modern stacks
- [Demo 34: Usage Metering](../34-usage-metering/) -- Monitor token consumption across your projects
