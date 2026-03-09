---
triggers: ["app builder", "scaffold app", "project template", "new project", "quick start", "app generator", "full stack generator", "bolt.new", "provision database", "provision auth"]
tools_allowed: ["read_file", "write_file", "bash"]
category: workflow
---

# App Builder

When building or scaffolding new applications:

1. Start by enhancing the user's idea into a structured specification — extract title, user stories, tech stack recommendation, database schema, API endpoints, and UI components before writing any code.
2. Use project templates when available: check `~/.vibecli/templates/` for reusable starters; offer to save successful projects as templates with `save_template()` for team reuse.
3. Auto-provision resources: set up database (SQLite for prototypes, PostgreSQL for production, Supabase for managed), authentication (JWT for APIs, OAuth for web apps, Supabase Auth for full-stack), and hosting configuration in one step.
4. Generate the complete project structure: package.json/Cargo.toml, .gitignore, .env.example, docker-compose.yml, CI config, README — don't leave scaffolding gaps that require manual setup.
5. For full-stack apps, generate a unified backend configuration: docker-compose.yml with all services (DB, auth, API, frontend), deployment manifests, and environment variable templates.
6. Use the AI Enhancer to convert rough ideas ("make me a todo app") into structured technical specs with user stories, acceptance criteria, and implementation plans before scaffolding.
7. Support team templates: save completed projects as reusable starters with `team_template_store.save_template()` — standardize tech stack, directory structure, and configuration across the team.
8. Estimate complexity before starting: Simple (landing page, single CRUD), Medium (multi-entity with auth), Complex (real-time, multi-service, payments) — adjust scaffolding depth accordingly.
