---
triggers: ["go test", "go benchmark", "table driven test", "go fuzzing", "testify", "go testing"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["go"]
category: testing
---

# Go Testing & Benchmarks

When testing Go code:

1. Use table-driven tests: define `[]struct{name, input, expected}`, loop with `t.Run(tc.name, ...)`
2. Use `t.Helper()` in test helper functions so failures report the caller's line
3. Use `t.Parallel()` for independent tests — speeds up the test suite
4. Use `testify/assert` for readable assertions: `assert.Equal(t, expected, actual)`
5. Use `testify/require` for fatal assertions that should stop the test
6. Benchmark with `func BenchmarkFoo(b *testing.B) { for i := 0; i < b.N; i++ { ... } }`
7. Run benchmarks: `go test -bench=. -benchmem` to see allocations
8. Use `t.TempDir()` for test files — auto-cleaned after test
9. Use `t.Cleanup(func() { ... })` for teardown — runs even if test fails
10. Fuzz testing: `func FuzzFoo(f *testing.F) { f.Add(seed); f.Fuzz(func(t *testing.T, data []byte) {...}) }`
11. Use `httptest.NewServer` for integration testing HTTP clients
12. Use build tags `//go:build integration` to separate unit and integration tests
