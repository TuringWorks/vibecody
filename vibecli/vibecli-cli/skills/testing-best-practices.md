---
triggers: ["unit test", "integration test", "test coverage", "mocking", "TDD"]
tools_allowed: ["read_file", "write_file", "bash"]
category: testing
---

# Testing Best Practices

1. Name tests descriptively: `test_<function>_<scenario>_<expected>`
2. Follow AAA pattern: Arrange, Act, Assert
3. Test behavior, not implementation — tests should survive refactors
4. One logical assertion per test (multiple asserts on the same result are fine)
5. Use table-driven tests for multiple inputs: `#[test_case]` (Rust), `test.each` (JS)
6. Mock external dependencies (HTTP, DB, filesystem) at boundaries
7. Test edge cases: empty input, max values, unicode, concurrent access
8. Integration tests should use real dependencies when possible (testcontainers)
9. Avoid flaky tests: no sleep-based timing, use deterministic seeds
10. Test error paths — verify errors have actionable messages
