---
triggers: ["TDD cycle", "red green refactor", "test first development", "test driven cycle"]
tools_allowed: ["read_file", "write_file", "bash"]
category: workflow
---

# TDD Cycle Orchestration

When orchestrating a TDD workflow:

1. **Red phase**: write ONE failing test — the test defines the next behavior to implement
2. Verify the test fails for the RIGHT reason — not a syntax error or missing import
3. **Green phase**: write the MINIMUM code to pass — resist the urge to generalize
4. Hardcode return values if needed — the next test will force you to generalize
5. **Refactor phase**: clean up while green — extract, rename, remove duplication
6. Run ALL tests after refactor — ensure nothing broke
7. Cycle time: 2-10 minutes per red→green→refactor cycle — if longer, your step is too big
8. When stuck: write a simpler test, or break the current test into smaller pieces
9. Test naming: describe the behavior, not the method — `should_reject_negative_amounts`
10. Outside-in: start from the API/interface test, work inward to implementation
11. Inside-out: start from domain logic tests, build outward to integration
12. Commit after each green+refactor cycle — each commit represents a working increment
