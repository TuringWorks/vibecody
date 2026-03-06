---
triggers: ["load test", "k6", "artillery", "performance test", "latency percentile", "capacity planning", "stress test"]
tools_allowed: ["read_file", "write_file", "bash"]
category: testing
---

# Load & Performance Testing

When conducting load tests:

1. Use k6 for scripted load tests — JavaScript-based, CLI-driven, CI-friendly
2. Define scenarios: ramp-up (gradual), constant (steady state), spike (sudden burst), soak (long-running)
3. Key metrics: requests/sec, latency (p50, p90, p95, p99), error rate, throughput
4. Set thresholds: `p(95) < 200ms`, error rate < 1%, min throughput > 1000 req/s
5. Realistic data: use production-like payloads and user patterns — not identical requests
6. Think time: add pauses between requests to simulate real user behavior
7. Start small: establish a baseline with low load before running stress tests
8. Monitor server resources during tests: CPU, memory, disk I/O, network, DB connections
9. Identify bottlenecks: is it CPU-bound, memory-bound, I/O-bound, or network-bound?
10. Test in a production-like environment — dev instances have different resource limits
11. Capacity planning: determine at what load the system degrades — plan for 2x expected peak
12. Regression: run load tests in CI — catch performance regressions before production
