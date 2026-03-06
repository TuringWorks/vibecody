---
triggers: ["property based test", "proptest", "hypothesis", "fast-check", "fuzzing test", "quickcheck", "shrinking"]
tools_allowed: ["read_file", "write_file", "bash"]
category: testing
---

# Property-Based Testing

When using property-based testing:

1. Identify properties: "for all valid inputs, this invariant holds" (roundtrip, idempotency, commutativity)
2. Rust: use `proptest!` macro with strategies: `prop::string::string_regex("[a-z]{1,10}")`
3. Python: use `hypothesis` with `@given(st.integers(), st.text())` decorators
4. JavaScript: use `fast-check` with `fc.assert(fc.property(fc.integer(), (n) => ...))`
5. Common properties to test: encode/decode roundtrip, sort idempotency, no crashes on any input
6. Let the framework shrink failing cases — the minimal counterexample is the bug report
7. Use `assume()` to filter invalid inputs — don't generate data you'll immediately reject
8. Test algebraic properties: commutativity (a+b = b+a), associativity, identity element
9. Combine with example-based tests — properties catch edge cases humans miss
10. Set a reasonable number of test cases (100-1000) — balance coverage vs. speed
11. Use custom generators for domain objects — build from primitives up
12. When a property test fails, add the shrunk case as a regression unit test
