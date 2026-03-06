---
triggers: ["tokio", "async rust", "await", "spawn", "Arc Mutex", "channel", "select!", "concurrency rust", "async fn"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["cargo"]
category: rust
---

# Rust Async & Concurrency

When working with async Rust and concurrency:

1. Use `tokio` as the async runtime — add `#[tokio::main]` or `#[tokio::test]`
2. Prefer `tokio::spawn` for CPU-light I/O tasks; use `spawn_blocking` for CPU-heavy work
3. Use `Arc<Mutex<T>>` (tokio::sync::Mutex) for shared mutable state across tasks
4. Prefer `tokio::sync::mpsc` channels over `std::sync::mpsc` in async code
5. Use `tokio::select!` for racing multiple futures — always include cancellation safety
6. Avoid holding `MutexGuard` across `.await` points — clone data out first
7. Use `tokio::join!` to run independent futures concurrently, not sequentially
8. Prefer `RwLock` over `Mutex` when reads vastly outnumber writes
9. Use `tokio::sync::Semaphore` to limit concurrent operations (connection pools, rate limits)
10. For fan-out/fan-in, use `FuturesUnordered` or `JoinSet` instead of collecting handles manually
11. Always set timeouts on network operations: `tokio::time::timeout(Duration::from_secs(30), fut)`
12. Use `#[instrument]` from tracing for async function observability
