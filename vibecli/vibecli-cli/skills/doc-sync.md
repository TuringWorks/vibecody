# Doc Sync

Bidirectional synchronization between documentation and code. Detects when code changes make docs stale, when spec changes require code updates, and automatically generates patches to keep both in sync.

## When to Use
- Detecting stale documentation after API or interface changes
- Generating doc updates from code changes automatically
- Checking if code matches its specification or design document
- Maintaining README, API docs, and inline comments in sync with code
- Running doc freshness audits as part of CI/CD

## Commands
- `/docsync check` — Scan for doc-code drift and report mismatches
- `/docsync fix` — Auto-generate patches to sync stale docs with code
- `/docsync spec <doc>` — Check if code matches a specification document
- `/docsync watch` — Enable continuous sync monitoring during development
- `/docsync report` — Generate a freshness report for all documentation
- `/docsync link <doc> <code>` — Create a tracked link between doc and code
- `/docsync unlink <id>` — Remove a doc-code link
- `/docsync config` — Show sync rules and linked pairs

## Examples
```
/docsync check
# Found 7 doc-code mismatches:
# [1] README.md:42 — CLI flag --verbose removed in v2.3
# [2] docs/api.md:118 — POST /users now requires email field
# [3] src/lib.rs:15 — Doc comment says "returns Option" but returns Result
# [4-7] ...

/docsync fix
# Generated 7 patches:
# README.md: Updated CLI usage section (+3 -5 lines)
# docs/api.md: Updated /users endpoint docs (+8 -3 lines)
# src/lib.rs: Fixed doc comment return type
# Apply all? [y/n]

/docsync spec docs/auth-spec.md
# Spec compliance: 87% (26/30 requirements implemented)
# Missing: rate limiting (spec 4.2), audit log (spec 4.5)
```

## Best Practices
- Link critical specs to their implementation files for tracked sync
- Run docsync check in CI to prevent stale docs from merging
- Fix doc comments first as they are closest to the code
- Use spec mode for compliance-critical features with formal requirements
- Review auto-generated patches before applying as context may be needed
