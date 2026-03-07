---
triggers: ["TechEmpower", "techempower", "framework benchmark", "json serialization benchmark", "fortunes test", "plaintext benchmark", "database benchmark optimization"]
tools_allowed: ["read_file", "write_file", "bash"]
category: performance
---

# TechEmpower Benchmark Optimization Patterns

When working with TechEmpower-style benchmark optimization:

1. For the plaintext test, bypass all middleware and write raw bytes directly to the socket: use fixed `Content-Length`, pre-encoded `"Hello, World!"` bytes, and avoid string allocation on every request.
2. For the JSON test, pre-compute static portions of the JSON response and use fast serializers (simdjson, sonic-json, dsl-json) instead of reflection-based ones; avoid intermediate string representations.
3. Pin IO threads to CPU cores with thread affinity and use one event loop per core: `SO_REUSEPORT` allows multiple threads to accept connections independently, eliminating accept-lock contention.
4. Use pipelined database queries for multi-query tests: send all N queries simultaneously and await results in bulk rather than sequentially; use prepared statements exclusively to skip query parsing.
5. For the fortunes test, pre-sort the template with the additional "Additional fortune" row included; escape HTML with a SIMD-accelerated or lookup-table-based escaper, not regex or character-by-character loops.
6. Use connection pooling sized to match database parallelism: `pool_size = cpu_cores * 2` is a starting point; tune with `pgbench` and measure latency percentiles, not just throughput.
7. Minimize memory allocation per request: use arena allocators or object pools for request/response buffers; reuse byte buffers across requests to reduce GC pressure in managed languages.
8. Disable unnecessary HTTP features: skip `Date` header computation (update once per second via timer), omit `Server` header, avoid chunked encoding for known-length responses.
9. Configure TCP optimally: enable `TCP_NODELAY` to disable Nagle's algorithm, set `TCP_FASTOPEN` for connection reuse, and tune `SO_SNDBUF`/`SO_RCVBUF` to match expected response sizes.
10. For database tests, use the fastest wire protocol driver available: `pgjdbc-ng` or `vertx-pg-client` for Java, `tokio-postgres` for Rust, `pgx` for Go; avoid ORM overhead in benchmark-critical paths.
11. Batch database updates in the update test: use `UPDATE ... SET val = CASE id WHEN 1 THEN v1 WHEN 2 THEN v2 END WHERE id IN (1,2)` as a single query instead of N individual updates.
12. Profile with `perf stat`, `flamegraph`, and `strace` to find the actual bottleneck: common culprits are syscall overhead (batch writes with `writev`), lock contention (use lock-free structures), and cache misses (keep hot data contiguous).
