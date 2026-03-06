---
triggers: ["golang", "go module", "goroutine", "go test", "go fmt"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["go"]
category: go
---

# Go Patterns

1. Use `error` return values — Go doesn't have exceptions
2. Check errors immediately: `if err != nil { return fmt.Errorf("context: %w", err) }`
3. Use `context.Context` for cancellation and timeouts
4. Use `sync.WaitGroup` or channels for goroutine coordination
5. Avoid goroutine leaks: always ensure goroutines can exit
6. Use `defer` for cleanup (file close, mutex unlock)
7. Use table-driven tests with `t.Run()` subtests
8. Use `go vet`, `staticcheck`, and `golangci-lint` for static analysis
9. Use interfaces for dependency injection — accept interfaces, return structs
10. Use `internal/` packages for unexported code boundaries
