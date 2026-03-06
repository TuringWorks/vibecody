---
triggers: ["flamegraph", "perf", "profiling", "Chrome DevTools", "benchmark", "CPU profiling", "memory profiling"]
tools_allowed: ["read_file", "write_file", "bash"]
category: performance
---

# Performance Profiling

When profiling application performance:

1. Measure first, optimize second — never guess where the bottleneck is
2. Use flamegraphs to visualize CPU time distribution — identify hot paths visually
3. Rust: use `cargo flamegraph` or `perf record + perf script | inferno-flamegraph`
4. Node.js: use `--prof` flag → `node --prof-process` or `clinic.js` suite
5. Python: use `cProfile` + `snakeviz` for call graphs; `py-spy` for production profiling
6. Browser: Chrome DevTools Performance tab — record, analyze Main thread activity
7. Memory profiling: Rust `dhat`, Node.js `--heap-prof`, Python `tracemalloc`, Go `pprof`
8. Benchmark critical paths: Rust `criterion`, Go `testing.B`, JS `benchmark.js`
9. Use `EXPLAIN ANALYZE` for database query profiling — check sequential scans, sort costs
10. Look for: unnecessary allocations, N+1 queries, synchronous I/O, large JSON serialization
11. Profile in production-like conditions — dev mode has overhead (debug symbols, hot reload)
12. Set performance budgets: API latency p99 < 200ms, page load < 3s, bundle size < 200KB
