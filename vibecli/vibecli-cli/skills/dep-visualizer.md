# Dependency Visualizer

Import graph generation with Mermaid and DOT output, cycle detection, and coupling metrics. Matches Cursor 4.0's dependency graph visualizer.

## When to Use
- Understanding which modules are tightly coupled before a refactor
- Detecting circular imports (cycles) that will cause compilation errors
- Generating architecture diagrams from actual import relationships
- Finding the most-imported modules (high fan-in = high blast radius)

## Graph Concepts
- **DepNode** — module/file/package with an ID, label, and kind
- **DepEdge** — directed import relationship (`from` → `to`)
- **fan-in** — number of modules that import this one
- **fan-out** — number of modules this one imports
- **coupling** — fan-in + fan-out (higher = more coupled)

## Cycle Detection
Uses DFS with a recursion stack. Returns each cycle as a path of node IDs.

## Render Formats
- **Mermaid** — `graph TD` format, embeddable in Markdown
- **DOT** — Graphviz digraph format for `dot -Tpng`

## Commands
- `/deps graph` — show full dependency graph
- `/deps mermaid` — output Mermaid diagram
- `/deps dot` — output DOT/Graphviz diagram
- `/deps cycles` — list all detected cycles

## Examples
```
/deps cycles
# Cycle detected: src/a.rs → src/b.rs → src/a.rs

/deps mermaid
# ---
# title: Project Deps
# ---
# graph TD
#     src_lib["lib"]
#     src_utils["utils"]
#     src_lib --> src_utils
```
