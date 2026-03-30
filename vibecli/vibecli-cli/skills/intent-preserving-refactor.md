# Intent-Preserving Refactoring

Transform code based on high-level intent while preserving behavioral equivalence. Each refactoring step is verified to ensure the public API and behavior remain unchanged.

## Supported Intents
- **make-testable** — Inject dependencies, extract interfaces
- **reduce-coupling** — Break tight coupling between modules
- **improve-performance** — Optimize hot paths, reduce allocations
- **add-error-handling** — Replace panics with proper Result types
- **extract-service** — Pull functionality into a separate service
- **consolidate-duplicates** — DRY up repeated code
- **modernize-syntax** — Update to modern language idioms
- **add-typing** — Add type annotations to untyped code
- **split-module** — Break large modules into focused ones
- **merge-modules** — Combine related small modules
- **add-caching** — Add memoization or caching layers
- **add-logging** — Instrument code with structured logging

## Commands
- `/refactor intent <description>` — Parse intent and generate a step-by-step plan
- `/refactor suggest <code>` — Suggest refactoring opportunities
- `/refactor plan` — Show the current refactoring plan
- `/refactor execute` — Execute the next planned step
- `/refactor verify` — Verify behavioral equivalence
- `/refactor rollback` — Rollback all completed steps
- `/refactor metrics` — Show refactoring statistics

## Workflow
1. Describe your intent: `/refactor intent "make this module testable"`
2. Review the generated plan
3. Execute steps one at a time with `/refactor execute`
4. Each step is auto-verified for equivalence
5. Rollback anytime with `/refactor rollback`

## Example
```
/refactor intent "reduce coupling between auth and session modules"
/refactor plan
/refactor execute
```
