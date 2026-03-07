---
triggers: ["Crystal", "crystal lang", "kemal", "amber crystal", "lucky framework", "crystal shards"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["crystal"]
category: crystal
---

# Crystal Web Development

When working with Crystal web frameworks:

1. Use Kemal for lightweight APIs — define routes with `get "/users" { |env| ... }` and access params via `env.params.url["id"]` for path params and `env.params.query["q"]` for query strings.
2. In Lucky Framework, use action classes that inherit from `BrowserAction` or `ApiAction` — define `route { get "/users" }` and `render` methods for type-safe, compile-time-checked routing.
3. Leverage Crystal's type system by defining models as `class` or `struct` with type annotations — the compiler catches nil-reference errors at compile time via union types like `String?`.
4. Use Lucky's Avram ORM for database operations — define models with `table`, columns with `column name : String`, and queries with `UserQuery.new.name("alice").first`.
5. Handle JSON APIs by including `JSON::Serializable` on your structs and using `from_json`/`to_json` — set response content type with `env.response.content_type = "application/json"`.
6. Use `spawn` and `Channel` for concurrent operations — Crystal's fiber-based concurrency handles thousands of concurrent connections efficiently without threading complexity.
7. Install dependencies via `shard.yml` and `shards install` — pin versions with `version: ~> 1.0` and run `shards check` in CI to ensure reproducible builds.
8. Add before/after filters in Kemal with `before_all` and `after_all` blocks for authentication, logging, and CORS headers — use `before_get`/`before_post` for method-specific filters.
9. Use Lucky's `Serializer` classes to control JSON output shape — define `def render` returning a named tuple to decouple API response format from internal model structure.
10. Connect to PostgreSQL with the `pg` shard or Lucky's built-in Avram — use connection pooling via `DB.open("postgres://...")` and pass the pool to handlers.
11. Write tests with Crystal's built-in `spec` framework — use `describe`/`it` blocks and `Spec::Client` (Lucky) or direct handler invocation (Kemal) for HTTP endpoint testing.
12. Compile with `crystal build src/app.cr --release --no-debug` for production — the resulting static binary is fast to deploy and has no runtime dependencies beyond libc.
