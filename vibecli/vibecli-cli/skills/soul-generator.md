# Soul.md Generator

Generate a SOUL.md file that captures a project's philosophy, core beliefs, and design principles.

## Commands

- `/soul` or `/soul generate` — Scan the project and generate SOUL.md
- `/soul show` — View the existing SOUL.md
- `/soul scan` — Display detected project signals without generating
- `/soul regenerate` — Overwrite an existing SOUL.md
- `/soul prompt` — Get an LLM prompt for richer AI-powered generation

## What Gets Detected

The scanner reads the project directory and extracts:

| Signal | How |
|--------|-----|
| **Languages** | Cargo.toml (Rust), package.json (JS/TS), pyproject.toml (Python), go.mod (Go), mix.exs (Elixir), etc. |
| **Frameworks** | Dependency names: React, Next.js, Vue, Axum, Django, FastAPI, Gin, Phoenix, etc. |
| **License** | LICENSE file content: MIT, Apache-2.0, GPL, BSD, MPL-2.0, ISC |
| **Package Manager** | cargo, npm, pnpm, yarn, bun |
| **Monorepo** | Cargo workspace, npm/pnpm workspaces, lerna.json, nx.json |
| **Testing** | tests/ directory, jest/vitest/pytest config files |
| **CI** | .github/workflows, .gitlab-ci.yml, Jenkinsfile |
| **Docker** | Dockerfile, docker-compose.yml |

## Generated Sections

1. **Why This Project Exists** — Purpose and motivation
2. **Core Beliefs** — 3-6 principles (adapts based on signals: open source, testing, monorepo, CI/Docker)
3. **Design Principles** — Language rationale, framework policy, dependency philosophy
4. **What This Project Is Not** — Explicit boundaries
5. **How to Know If a Change Belongs** — 5-question decision framework

## Architecture

- `soul_generator.rs` — Core module: scan_project(), generate_template_soul(), build_generation_prompt(), write_soul()
- `SoulPanel.tsx` — VibeUI panel (View/Generate/Signals tabs)
- 4 Tauri commands: soul_scan, soul_generate, soul_regenerate, soul_read
- Output: SOUL.md in the project root
