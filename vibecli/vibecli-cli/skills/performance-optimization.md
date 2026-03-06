---
triggers: ["performance", "optimization", "slow", "memory", "profiling", "benchmark"]
tools_allowed: ["read_file", "write_file", "bash"]
category: performance
---

# Performance Optimization

1. Measure before optimizing — use profilers (flamegraph, perf, Chrome DevTools)
2. Optimize the algorithm first, micro-optimize last
3. Avoid unnecessary allocations: reuse buffers, use `&str` over `String` where possible
4. Use streaming/iterators for large data sets instead of collecting into Vec
5. Cache expensive computations (LRU cache, memoization, HTTP cache headers)
6. Use async I/O for network and file operations
7. Batch database queries — avoid N+1 patterns
8. Use connection pooling for HTTP clients and database connections
9. Prefer stack allocation over heap: `[u8; N]` over `Vec<u8>` for small fixed buffers
10. Profile memory usage: watch for leaks, unbounded caches, and large clones
