---
triggers: ["V lang", "vlang", "veb", "v web"]
tools_allowed: ["read_file", "write_file", "bash"]
category: v
---

# V Language Web

When working with V for web development:

1. Use `veb` (V's built-in web framework) by defining a struct with handler methods — annotate routes with `['/path'; get]` attributes and call `veb.run(app, 8080)` to start serving.
2. Access route parameters via method arguments that match the URL pattern: `fn (mut app App) user(id string) veb.Result` automatically binds `/user/:id` path segments.
3. Serve static files by setting `pub const static_directory = 'public'` on your app struct — veb serves everything under that directory without explicit route definitions.
4. Use V's built-in `json.decode(T, raw_string)` for request body parsing and `json.encode(obj)` for serialization — both work with struct types directly without code generation.
5. Handle database access with `db := sqlite.connect('app.db')` or `pg.connect(config)` and use V's ORM with `sql db { select from User where id == user_id }` syntax for type-safe queries.
6. Implement middleware logic in the `before_request()` method on your app struct — this runs before every handler and is the place for auth checks, logging, and CORS headers.
7. Return responses using `app.text('message')`, `app.json(obj)`, `app.html(template)`, or `app.redirect('/path')` — each sets correct Content-Type headers automatically.
8. Use V's compile-time HTML templates with `$tmpl('template.html')` for server-rendered pages — variables from the handler scope are available in the template without explicit passing.
9. Handle errors with V's `or {}` blocks and `Result` types rather than exceptions — return `app.server_error(500)` for unrecoverable failures and log details server-side.
10. Run concurrent tasks with `spawn fn_name(args)` for background work like email sending or webhook delivery — communicate results back via shared channels.
11. Compile the production binary with `v -prod -cc gcc server.v` for optimized output; the resulting static binary has no runtime dependencies and starts in milliseconds.
12. Write tests in `_test.v` files using `assert` statements; test HTTP handlers by calling them directly with mock app state or by using `net.http.get('http://localhost:8080/path')` in integration tests.
