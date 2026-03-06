---
triggers: ["TDD", "test driven", "red green refactor", "test first", "failing test", "fire-tdd"]
tools_allowed: ["read_file", "write_file", "bash"]
category: testing
---

# Test-Driven Development Workflow

When practicing TDD (inspired by fire-flow /fire-tdd):

1. **Red**: Write a failing test first — it must fail for the right reason
2. **Green**: Write the minimum code to make the test pass — no more
3. **Refactor**: Clean up the code while keeping tests green — extract, rename, simplify
4. Start with the simplest case: empty input, zero, null, single element
5. Add one behavior per test — small incremental steps build confidence
6. Name tests after behaviors: `should_return_empty_list_when_no_items_match`
7. Test the contract, not the implementation — tests should survive refactoring
8. Use the "Transformation Priority Premise": nil→constant→variable→collection progression
9. When stuck, write a more specific test to drive the next bit of implementation
10. Don't refactor red tests — only refactor when all tests are green
11. Use test doubles (mocks/stubs) at boundaries — database, network, file system
12. Run the full test suite after each refactor cycle to catch regressions
