---
triggers: ["Phoenix", "phoenix framework", "elixir phoenix", "LiveView", "phoenix liveview", "phoenix channels", "ecto", "elixir plug"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["elixir", "mix"]
category: elixir
---

# Elixir Phoenix Framework

When working with Phoenix:

1. Use `mix phx.gen.live` for LiveView resources and `mix phx.gen.html` for traditional controller-based CRUD; these generators produce contexts, schemas, migrations, templates, and tests in one command — customize after generation.
2. Organize business logic in contexts (e.g., `Accounts`, `Catalog`) rather than in controllers or LiveViews; contexts are the boundary between web and domain layers — keep `MyApp.Accounts.create_user/1` as the public API.
3. In LiveView, minimize assigns in the socket — use `assign/3` and `assign_new/3` to avoid redundant data; push expensive data via `send_update/3` to child components and use `stream/3` for large lists to avoid keeping all items in memory.
4. Use Ecto changesets for all data validation — define `changeset/2` functions on schemas and call `Repo.insert/update` with changesets; leverage `cast/3`, `validate_required/2`, and custom validators for domain-specific rules.
5. Write Ecto queries with the query DSL (`from u in User, where: u.active == true`) and compose them with functions that return `Ecto.Query`; avoid raw SQL unless performance-critical — use `Repo.explain(:all, query)` to inspect query plans.
6. Use Phoenix Channels for real-time features — define a `channel` in your socket, implement `handle_in/3` for incoming messages, and broadcast with `broadcast/3`; use Presence for user tracking across distributed nodes.
7. Leverage Plug pipelines in `router.ex` for cross-cutting concerns; define custom plugs as modules with `init/1` and `call/2` for auth, rate limiting, and request transformation — compose them in `pipeline` blocks.
8. Write tests with `ExUnit` and `DataCase` / `ConnCase` / `ChannelCase` helpers; use `Ecto.Adapters.SQL.Sandbox` for concurrent test isolation and `Phoenix.ConnTest` for controller assertions — run with `mix test --cover` for coverage.
9. Handle background work with Oban (job processing library) instead of raw `Task.async` — Oban persists jobs to PostgreSQL, handles retries, scheduling, and uniqueness; define workers with `use Oban.Worker` and enqueue with `Oban.insert/1`.
10. Configure LiveView uploads with `allow_upload/3` in `mount/2` for direct-to-server or external (S3) uploads; validate file type and size in the allow config and consume uploads in `handle_event` with `consume_uploaded_entries/3`.
11. Deploy with `mix release` to build an OTP release; use multi-stage Docker builds with `elixir:1.16-alpine` for build and `alpine:3.19` for runtime — set `PHX_HOST` and `SECRET_KEY_BASE` as environment variables in production.
12. Use `Phoenix.PubSub` for inter-process communication across nodes; broadcast from contexts (not controllers) to decouple real-time updates from web layer — this enables LiveView updates, Channel broadcasts, and background job notifications from a single publish.
