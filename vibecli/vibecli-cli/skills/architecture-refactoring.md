---
triggers: ["strangler fig", "feature flag", "tech debt", "refactoring strategy", "legacy code", "incremental migration"]
tools_allowed: ["read_file", "write_file", "bash"]
category: architecture
---

# Refactoring & Tech Debt

When managing refactoring and technical debt:

1. Strangler Fig pattern: build new alongside old, gradually route traffic — don't big-bang rewrite
2. Feature flags: deploy new code behind toggles — enable incrementally, roll back instantly
3. Characterization tests: before refactoring, write tests that capture current behavior
4. Boy Scout Rule: leave code better than you found it — small improvements with each change
5. Identify tech debt types: deliberate (known shortcuts), inadvertent (design gaps), bit rot (age)
6. Prioritize by impact: debt on hot paths > debt on rarely-touched code
7. Extract Method: pull cohesive logic into named functions — improves readability and reuse
8. Replace Conditional with Polymorphism: use strategy/visitor pattern for complex switches
9. Introduce Parameter Object: group related parameters into a struct/class
10. Migration checklist: tests green → refactor → tests green → deploy → monitor
11. Track tech debt: use TODO/FIXME comments with ticket references, review in sprint planning
12. Set a "tech debt budget": allocate 15-20% of sprint capacity for cleanup
