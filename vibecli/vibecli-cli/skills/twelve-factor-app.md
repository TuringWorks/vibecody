---
triggers: ["12 factor", "twelve factor", "12-factor app", "twelve-factor", "heroku methodology", "cloud native app", "twelve factor app"]
tools_allowed: ["read_file", "write_file", "bash"]
category: architecture
---

# Twelve-Factor App Methodology

When building cloud-native applications following the 12-Factor methodology:

1. **Codebase** — Maintain exactly one codebase per app tracked in version control, with many deploys (staging, production, dev). Never share code between apps; extract shared code into libraries consumed via dependency managers.
2. **Dependencies** — Explicitly declare and isolate all dependencies using a manifest (package.json, Cargo.toml, requirements.txt). Never rely on implicit system-wide packages; use vendoring or lockfiles to guarantee reproducible builds across environments.
3. **Config** — Store all environment-specific configuration (database URLs, API keys, feature flags) in environment variables, never in code. Config varies between deploys; code does not. Validate required config at startup and fail fast if missing.
4. **Backing Services** — Treat all backing services (databases, caches, message queues, SMTP) as attached resources accessed via URL or locator in config. Swapping a local PostgreSQL for an RDS instance should require only a config change, zero code changes.
5. **Build, Release, Run** — Strictly separate build (compile + bundle), release (build + config), and run (execute in environment) stages. Every release gets an immutable, unique ID. Never patch running code; create a new release instead.
6. **Processes** — Execute the app as one or more stateless processes that share nothing. Any persistent data must live in a backing service. Use sticky sessions only as a cache, never as authoritative state; assume any process can be replaced at any moment.
7. **Port Binding** — Export services by binding to a port and listening for requests. The app is completely self-contained; do not rely on runtime injection of a web server. One app can become another app's backing service via its URL.
8. **Concurrency** — Scale out via the process model, assigning different work types to different process types (web, worker, scheduler). Use the OS process manager rather than daemonizing; this enables horizontal scaling by simply adding more process instances.
9. **Disposability** — Maximize robustness with fast startup and graceful shutdown. Processes should start in seconds, handle SIGTERM gracefully by finishing current requests, and use robust job queues that return work on disconnect. Design for crash-only operation.
10. **Dev/Prod Parity** — Keep development, staging, and production as similar as possible. Minimize time gap (deploy hours after code), personnel gap (devs who wrote it deploy it), and tools gap (use the same backing services locally via containers, not lightweight substitutes).
11. **Logs** — Treat logs as event streams written to stdout. Never manage log files or routing within the app. Let the execution environment capture, collate, and route streams to indexing systems (ELK, Datadog, CloudWatch) for analysis and alerting.
12. **Admin Processes** — Run one-off admin tasks (migrations, REPL sessions, data fixes) as identical processes in the same environment and release as the app. Ship admin code with app code; use the same dependency isolation and config to avoid drift and synchronization issues.
