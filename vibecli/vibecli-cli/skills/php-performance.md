---
triggers: ["Swoole", "openswoole", "RoadRunner", "FrankenPHP", "php async", "php performance", "hyperf", "reactphp"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["php"]
category: php
---

# High-Performance PHP (Swoole, RoadRunner, FrankenPHP)

When working with high-performance PHP:

1. Use Swoole's coroutine-based HTTP server with `Swoole\Http\Server` for long-lived processes: enable coroutines with `Co::set(['hook_flags' => SWOOLE_HOOK_ALL])` to automatically make PDO, Redis, curl, and file I/O non-blocking without code changes.
2. Avoid global and static mutable state in Swoole/RoadRunner applications since the worker process persists across requests; use request-scoped containers or coroutine context (`Co::getContext()`) to prevent data leaking between requests.
3. Configure Swoole worker counts with `'worker_num' => swoole_cpu_num() * 2` for CPU-bound work or `* 4` for I/O-bound work; use `'task_worker_num'` for offloading blocking operations to separate task workers.
4. Use connection pools for database and Redis in long-running servers: Swoole provides `ConnectionPool` and `Channel`-based pooling; set pool size to match `worker_num` and implement health checks to evict stale connections.
5. Run Laravel or Symfony on RoadRunner by installing `spiral/roadrunner-laravel` or `baldinof/roadrunner-bundle`; configure `.rr.yaml` with worker count, max jobs per worker, and memory limits to prevent memory leaks.
6. Use FrankenPHP for zero-config high-performance hosting: it embeds PHP in a Go server with automatic HTTPS via Caddy, supports early hints (103), and works as a drop-in replacement for php-fpm with `frankenphp php-server --root public/`.
7. Implement async tasks in Swoole with `go(function() { ... })` coroutines for concurrent I/O: fan out multiple API calls with `Co\batch()` or use `Channel` for producer-consumer patterns to process requests in parallel.
8. Use Hyperf framework (built on Swoole) for microservices: leverage its DI container, annotation-based routing (`#[GetMapping]`), gRPC support, and built-in connection pooling for database, Redis, and HTTP clients.
9. Implement ReactPHP for event-driven applications: use `React\Http\HttpServer` with promises and streams for non-blocking I/O; combine with `react/mysql` and `react/redis` for fully async database access without the Swoole extension.
10. Profile and benchmark with `wrk`, `bombardier`, or `k6`; compare requests/sec and p99 latency between php-fpm, RoadRunner, Swoole, and FrankenPHP for your specific workload before committing to a runtime.
11. Handle graceful shutdown in long-lived servers: listen for SIGTERM, stop accepting new connections, finish in-flight requests within a deadline, close connection pools, and flush metrics; configure Kubernetes `terminationGracePeriodSeconds` accordingly.
12. Monitor memory usage per worker with `memory_get_usage(true)` and configure max requests per worker (`max_jobs` in RoadRunner, `max_request` in Swoole) to recycle workers before memory grows unbounded; use Prometheus exporters for runtime metrics in production.
