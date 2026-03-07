---
triggers: ["Lapis", "openresty", "lua web", "luajit", "lua nginx"]
tools_allowed: ["read_file", "write_file", "bash"]
category: lua
---

# Lua Web (OpenResty, Lapis)

When working with Lua web frameworks:

1. In Lapis, define routes in the app class: `match("/users/:id", function(self) return { json = self.params.id } end)` — path parameters are available via `self.params`.
2. Use OpenResty's `content_by_lua_block` in nginx.conf for inline handlers, but prefer `content_by_lua_file` pointing to separate `.lua` files for maintainability.
3. Leverage OpenResty's shared dictionary (`lua_shared_dict`) for in-process caching: `ngx.shared.cache:set("key", value, ttl)` — this persists across requests within the worker.
4. Access request data in OpenResty with `ngx.req.get_uri_args()` for query params, `ngx.req.get_body_data()` for POST body, and `ngx.req.get_headers()` for headers — always call `ngx.req.read_body()` first.
5. In Lapis, define database models with `class Users extends Model` and use `Users:find(id)`, `Users:select("where active = ?", true)`, and `Users:create({name = "val"})` for type-safe queries.
6. Run database migrations in Lapis with `lapis migrate` — define them in `migrations.lua` as numbered functions that call `schema.create_table` and `schema.add_column`.
7. Use `ngx.location.capture` for subrequests to other nginx locations — this enables composing internal APIs without external HTTP calls and runs in the same request context.
8. Handle non-blocking I/O with `cosocket` API: `ngx.socket.tcp()` for TCP, `ngx.socket.udp()` for UDP — these yield the current coroutine and resume on data, enabling high concurrency.
9. Implement authentication middleware in Lapis with `before_filter`: `before_filter(function(self) if not self.session.user then return { redirect_to = "/login" } end end)`.
10. Use `cjson.decode(body)` and `cjson.encode(table)` for JSON handling — cjson is bundled with OpenResty and is significantly faster than pure-Lua JSON libraries.
11. Configure connection pooling for databases with `ngx.socket.tcp`'s `setkeepalive(max_idle_ms, pool_size)` to reuse connections across requests and avoid connection overhead.
12. Test Lapis applications with `busted` framework and `Test` helpers: use `mock_request(app, "/path", {method = "GET"})` to simulate requests without starting a server.
