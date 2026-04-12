# Multi-Repo Context

Aggregate context across multiple repositories with cross-repo import graph analysis. Closes gap vs Cursor 4.0, Copilot Workspace v2, and Cody 6.0.

## When to Use
- Working in a monorepo with distinct service boundaries
- Tracking which service imports which shared library
- Injecting cross-repo context into the LLM window
- Detecting circular dependencies between repos

## Commands
- `/multirepo add <alias> <path> <lang>` — Register a repo
- `/multirepo remove <alias>` — Unregister a repo
- `/multirepo list` — Show all registered repos
- `/multirepo imports <alias>` — Show what `alias` imports
- `/multirepo importedby <alias>` — Show what imports `alias`
- `/multirepo cycles` — Detect circular imports
- `/multirepo order` — Topological build order
- `/multirepo context` — Generate LLM context summary

## Import Graph
Edges are added manually or auto-detected from source files.
Topological order loads dependency repos before consumers.

## Example
```
/multirepo add api /repos/api-server Rust
/multirepo add ui /repos/frontend TypeScript
/multirepo imports ui
# → ui → api (src/api/mod.rs)
```
