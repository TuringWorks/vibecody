---
triggers: ["goroutine", "go channel", "go select", "sync.WaitGroup", "worker pool go", "go concurrency"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["go"]
category: go
---

# Go Concurrency

When writing concurrent Go code:

1. Use goroutines for concurrent work — they're cheap (2KB stack), spawn thousands freely
2. Communicate via channels, not shared memory: `ch := make(chan Result, bufferSize)`
3. Use `sync.WaitGroup` to wait for a group of goroutines to finish
4. Use `select` with `context.Done()` for cancellation-aware goroutines
5. Always pass `context.Context` as the first parameter for cancellation propagation
6. Use buffered channels for producer-consumer patterns; unbuffered for synchronization
7. Worker pool pattern: N goroutines reading from a shared job channel
8. Use `sync.Mutex` for simple shared state; `sync.RWMutex` when reads dominate
9. Use `sync.Once` for one-time initialization (singleton pattern, config loading)
10. Use `errgroup.Group` from `golang.org/x/sync` for parallel tasks with error collection
11. Avoid goroutine leaks: always ensure goroutines can exit (context cancellation, done channels)
12. Use `-race` flag during testing: `go test -race ./...` to detect data races
