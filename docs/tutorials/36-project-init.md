---
layout: page
title: "Tutorial: Project Init & Auto-Context"
permalink: /tutorials/project-init/
---

# Project Init & Auto-Context

Learn how VibeCody's smart project scanner gives the AI deep understanding of your codebase — automatically, in every conversation.

**Prerequisites:** VibeCody installed with a working provider. See [First Provider](/tutorials/first-provider/) if needed.

---

## The Problem

When you ask an AI agent to "fix the tests" or "add auth", it doesn't know:
- What language or framework your project uses
- What the build and test commands are
- Where the entry points and config files are
- What environment variables are needed

Without this context, the agent guesses — and often guesses wrong.

---

## The Solution: `/init`

VibeCody's project scanner detects all of this automatically and injects it into every agent conversation.

### Step 1: Scan Your Project

```bash
vibecli
> /init
```

Output:

```
Scanning project...

Project: my-saas-app
  A full-stack SaaS application with auth and billing.
Architecture: full-stack application
Languages: TypeScript, Rust
Frameworks: React, Vite, Axum, Tokio
Package managers: pnpm, cargo

Build commands:
  pnpm run build → pnpm run build
  Cargo build → cargo build --workspace

Test commands:
  Vitest → pnpm run test (Vitest)
  Cargo test → cargo test --workspace (cargo test)

Lint commands:
  ESLint → npx eslint .
  Clippy → cargo clippy --workspace -- -D warnings

Entry points: src/App.tsx, src/main.rs
Expected env vars: DATABASE_URL, STRIPE_SECRET_KEY, JWT_SECRET

Key files: README.md (readme), Dockerfile (Dockerfile), .github/workflows/ci.yml (CI config)

Yes Project profile cached to .vibecli/project-profile.json
   This context will be auto-injected into every agent session.
```

### Step 2: Use the Agent — It Already Knows Your Project

Now when you run an agent task, VibeCody automatically injects the project profile:

```bash
> /agent add rate limiting to the API
```

The agent now knows:
- Your API is built with Axum (Rust)
- Tests run with `cargo test`
- The entry point is `src/main.rs`
- It should use `cargo clippy` to verify after changes

Without `/init`, the agent would have to spend multiple steps figuring this out.

---

## What Gets Detected

| Category | Detected For |
|----------|-------------|
| **Languages** | Rust, TypeScript/JavaScript, Python, Go, Java, C#, Ruby, PHP |
| **Frameworks** | React, Next.js, Vue, Svelte, Angular, Express, FastAPI, Django, Flask, Axum, Gin, Rails, Laravel, and 15+ more |
| **Build tools** | cargo, npm/pnpm/yarn/bun, go, maven, gradle, dotnet, bundler, composer |
| **Test frameworks** | cargo test, Jest, Vitest, Playwright, Cypress, pytest, go test, JUnit, PHPUnit, RSpec |
| **Lint tools** | Clippy, ESLint, Prettier, Biome, Ruff, go vet |
| **Architecture** | Monorepo, library, CLI tool, full-stack app, microservice cluster, single package |
| **Key files** | README, Dockerfile, CI config, API specs, env examples, test configs, schemas |
| **Env vars** | Extracted from `.env.example` / `.env.sample` / `.env.template` |

---

## Task-Based Auto-Context

When you give the agent a task, VibeCody also analyzes the task description and auto-gathers relevant files:

| Task mentions... | Auto-gathered files |
|-----------------|-------------------|
| "test", "spec" | `tests/`, `jest.config.js`, `pytest.ini` |
| "build", "compile" | `Cargo.toml`, `package.json`, `tsconfig.json` |
| "deploy", "CI" | `Dockerfile`, `.github/workflows/`, `docker-compose.yml` |
| "auth", "login" | `src/auth/`, `src/middleware/` |
| "database", "schema" | `prisma/`, `migrations/`, `src/models/` |
| "API", "endpoint" | `src/api/`, `src/routes/`, `pages/api/` |
| "style", "CSS" | `tailwind.config.js`, `src/styles/` |

Up to 5 relevant files are previewed and injected into the agent's context window.

---

## How It Works Under the Hood

1. **`/init`** calls `project_init::scan_workspace()` which reads package manifests, config files, and directory structure
2. The profile is cached to `.vibecli/project-profile.json` (1-hour TTL)
3. On every agent run, `run_agent_repl_with_context()` loads the cached profile
4. `build_system_prompt()` injects the profile summary into the LLM system prompt
5. `extract_relevant_files_for_task()` analyzes the task and gathers file previews

The profile is re-scanned automatically if the cache is older than 1 hour.

---

## AI Panel Context (VibeUI)

In VibeUI, the same project context is available:

- **@project** in the chat context picker injects the project profile
- **ProjectContextPanel** shows the full profile with 4 tabs: Overview, Commands, Key Files, AI Context
- One-click build/test/lint execution from the Commands tab

---

## Combining with `/orient`

For deeper analysis, use `/orient` after `/init`:

```bash
> /orient
```

This sends the auto-detected profile to the AI and asks it to analyze:
- Architecture patterns and design decisions
- Recommended next steps for a new developer
- Potential improvements and technical debt
- CI/CD recommendations

`/orient` uses the cached profile as a starting point, so it's faster and more accurate than running without `/init`.

---

## Tips

- Run `/init` once when you open a new project — it caches for 1 hour
- The `.vibecli/project-profile.json` file is auto-generated; add it to `.gitignore`
- For monorepos, `/init` detects the workspace structure and lists all members
- The profile works with all 22 providers — the context is injected at the prompt level
