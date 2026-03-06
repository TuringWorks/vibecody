---
triggers: ["jest", "vitest", "testing typescript", "mock function", "snapshot test", "test coverage", "describe it expect"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["node"]
category: testing
---

# TypeScript Testing with Jest/Vitest

When writing TypeScript tests:

1. Use `vitest` for new projects (faster, ESM-native, Vite-compatible); `jest` for existing setups
2. Structure: `describe` for grouping, `it`/`test` for cases, `expect` for assertions
3. Use `beforeEach`/`afterEach` for setup/teardown — avoid shared mutable state between tests
4. Mock modules with `vi.mock('module')` (vitest) or `jest.mock('module')` — mock at boundaries
5. Use `vi.fn()` / `jest.fn()` for spy functions — assert with `.toHaveBeenCalledWith()`
6. Snapshot testing: use `toMatchSnapshot()` sparingly — prefer explicit assertions for logic
7. Test async code with `async/await` — always `await` the expect: `await expect(fn()).resolves.toBe()`
8. Use `test.each` for parameterized tests: `test.each([[1,2,3], [4,5,9]])('add %i + %i', (a,b,expected) => ...)`
9. Coverage: aim for 80%+ on critical paths — don't chase 100% on UI glue code
10. Mock timers with `vi.useFakeTimers()` for testing debounce, throttle, setTimeout
11. Use `@testing-library/react` for component tests — query by role/label, not CSS selectors
12. Never test implementation details — test behavior and outputs, not internal state
