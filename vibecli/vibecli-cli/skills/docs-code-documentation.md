---
triggers: ["code comments", "README", "ADR", "architecture decision", "code documentation", "inline comments"]
tools_allowed: ["read_file", "write_file", "bash"]
category: documentation
---

# Code Documentation

When documenting code:

1. Write self-documenting code first — clear names, small functions, obvious structure
2. Comment the "why", not the "what" — code shows what, comments explain intent
3. README.md must include: what, why, quickstart, prerequisites, configuration
4. Architecture Decision Records (ADRs): document major decisions with context, options, rationale
5. Module-level comments: explain the purpose and high-level design of each module/package
6. Public API documentation: every public function/type needs a doc comment with examples
7. Keep comments in sync with code — stale comments are worse than no comments
8. Use `TODO(name):` for planned work, `FIXME(name):` for known bugs, `HACK:` for workarounds
9. Document non-obvious constraints: performance assumptions, thread safety, ordering requirements
10. Use diagrams for complex flows — Mermaid in markdown is version-controllable
11. CONTRIBUTING.md: setup instructions, coding conventions, PR process, testing expectations
12. Don't document obvious getters/setters — focus documentation effort on business logic
