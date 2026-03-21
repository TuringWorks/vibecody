# Project Initialization & Onboarding

## Trigger
When the user opens a new project, asks about project setup, or runs /init.

## Context
VibeCody auto-detects project profiles using the `project_init` module:
- Languages: Rust, TypeScript/JavaScript, Python, Go, Java, C#, Ruby, PHP
- Frameworks: React, Next.js, Vue, Svelte, Django, FastAPI, Express, Axum, Gin, Rails, Laravel, etc.
- Build/test/lint commands auto-detected from package manifests
- Architecture: monorepo, library, full-stack, CLI tool, microservice cluster
- Key files: README, Dockerfile, CI config, API specs, env examples

## Commands
- `/init` — Scan project and cache profile to `.vibecli/project-profile.json`
- `/orient` — AI-powered project analysis (uses cached profile + LLM)

## Workflow for Brownfield Projects
1. Run `/init` to scan and understand the project
2. Review detected build/test/lint commands
3. Check if env vars are set up correctly
4. Use `/orient` for AI-powered architecture analysis
5. Start coding — the agent now has project context in every conversation

## Workflow for Greenfield Projects
1. Use `/appbuilder` or the AppBuilder panel to scaffold a new project
2. Run `/init` after scaffolding to cache the profile
3. The agent will auto-detect the project type and suggest relevant tools

## Auto-Context
The project profile is automatically injected into every agent conversation.
This gives the AI:
- Knowledge of what languages and frameworks are in use
- The correct build, test, and lint commands
- Understanding of the project architecture
- Entry points and key configuration files
- Required environment variables
