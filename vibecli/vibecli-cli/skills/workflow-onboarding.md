---
triggers: ["onboarding", "project orientation", "codebase tour", "new developer", "fire-0-orient", "getting started"]
tools_allowed: ["read_file", "write_file", "bash"]
category: workflow
---

# Project Onboarding Workflow

When onboarding to a new project (inspired by fire-flow /fire-0-orient):

1. **Entry points**: find `main()`, `index.ts`, `app.py` — trace the startup sequence
2. **Architecture**: identify layers — API routes, business logic, data access, external services
3. **Key files**: map the 10-15 most important files — where does core logic live?
4. **Data model**: understand the database schema — tables, relationships, constraints
5. **Config**: find environment variables, config files, feature flags — what knobs exist?
6. **Build & run**: get the project running locally — follow README, fix gaps in docs
7. **Test**: run the test suite — understand what's tested and how
8. **Dependencies**: review package.json/Cargo.toml/requirements.txt — what libraries are used?
9. **Git history**: read recent commits and PRs — understand recent changes and conventions
10. **Domain language**: learn the business terms used in code — build a glossary
11. **Conventions**: code style, naming patterns, file organization, PR process
12. **First task**: pick a small bug or improvement — learn by doing in a low-risk change
