---
triggers: ["monorepo", "workspace", "turborepo", "nx", "dependency graph", "build caching", "pnpm workspace"]
tools_allowed: ["read_file", "write_file", "bash"]
category: architecture
---

# Monorepo Architecture

When managing monorepos:

1. Use workspace tools: `pnpm workspaces`, `cargo workspaces`, Turborepo, Nx, or Bazel
2. Organize by package/project, not by file type: `packages/api/`, `packages/web/`, `packages/shared/`
3. Shared packages: extract common code into internal packages — `@org/utils`, `@org/types`
4. Use task runners with dependency-aware scheduling: only rebuild what changed
5. Remote caching: cache build artifacts (Turborepo Remote Cache, Nx Cloud) — skip redundant builds
6. Enforce dependency boundaries: no circular deps, leaf packages don't import from apps
7. Use `tsconfig.json` paths or Cargo workspace members for cross-package imports
8. CI: use affected/changed detection to only test modified packages + dependents
9. Versioning: use Changesets (`@changesets/cli`) for independent package versioning
10. Code ownership: `CODEOWNERS` file mapping directories to teams
11. Consistent tooling: shared ESLint/Prettier/clippy config at root, inherited by packages
12. Benefits: atomic cross-package changes, shared CI, consistent tooling, code reuse
