---
triggers: ["OCaml", "dream ocaml", "opium", "dune", "ocaml web", "ocaml lwt"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["ocaml"]
category: ocaml
---

# OCaml Web Development

When working with OCaml web frameworks:

1. Use Dream as the primary web framework — define routes with `Dream.get "/path" handler` and compose them with `Dream.router [...]` for a concise, type-safe routing tree.
2. Chain middleware with `Dream.logger` and `Dream.memory_sessions` in the pipeline — middleware are functions wrapping handlers (`handler -> handler`), applied outermost first.
3. Use `Dream.json` to send JSON responses and `Dream.body` to read request bodies — pair with `yojson` and `ppx_deriving_yojson` for automatic serialization/deserialization.
4. Manage async operations with Lwt — use `let%lwt` (with `ppx_lwt`) or `let*` syntax to chain promises and avoid callback nesting in handlers.
5. Define database queries with Caqti — create a connection pool using `Caqti_lwt.connect_pool`, pass it through Dream middleware, and write typed queries with `Caqti_type.(int ->. string)`.
6. Use `dune` as the build system — define libraries with `(library ...)` stanzas, keep domain logic in a separate library from the web layer, and use `(preprocess (pps ...))` for PPX deriving.
7. Handle errors with the `result` type (`Ok/Error`) throughout the stack — use `Lwt_result` combinators or `let*` bindings to propagate errors without exceptions in request handlers.
8. Use Dream's built-in CSRF protection with `Dream.csrf_tag` in forms and `Dream.verify_csrf_token` — never skip this for any form-handling endpoint.
9. Validate input with custom validators returning `(value, error list) result` — check all fields and accumulate errors rather than failing on the first invalid field.
10. Serve static files with `Dream.static "public/"` route — place assets in a `public/` directory and reference them with absolute paths in templates.
11. Use `Dream.sql` with Caqti for transactional database access — wrap related queries in `Dream.sql (fun db -> ...)` to get automatic connection checkout and return.
12. Test handlers by constructing `Dream.request` values and calling handlers directly — assert on response status with `Dream.status` and body with `Dream.body` for unit-level route tests.
