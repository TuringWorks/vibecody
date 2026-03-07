---
triggers: ["Clojure", "ring", "compojure", "reitit", "pedestal", "luminus", "clojure web", "leiningen"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["java"]
category: clojure
---

# Clojure Web Development

When working with Clojure web frameworks:

1. Use Ring as the foundation — handlers are functions `(fn [request] response)` and middleware are higher-order functions that wrap handlers; keep this model in mind for all frameworks.
2. Prefer Reitit over Compojure for new projects — Reitit uses data-driven route definitions, supports coercion, and generates OpenAPI specs from the same route data.
3. Define routes as plain data vectors in Reitit (`["/users" {:get handler :post create}]`) and add middleware, coercion, and documentation as metadata on the route map.
4. Use `muuntaja` for content negotiation — configure it as Reitit middleware to automatically encode/decode JSON, EDN, and Transit based on Accept and Content-Type headers.
5. Manage application state with `mount`, `integrant`, or `component` — define system components (db pool, HTTP server, caches) with explicit start/stop lifecycle methods.
6. Use `next.jdbc` for database access — create a connection pool with `hikari-cp`, pass the datasource as a component, and use `execute!` / `execute-one!` for queries.
7. Validate request data with `malli` or `spec` — define schemas as data (`[:map [:name string?] [:age int?]]`) and wire validation into Reitit coercion middleware.
8. Use `ring.middleware.defaults/wrap-defaults` with `site-defaults` or `api-defaults` to apply standard security headers, anti-forgery, and session handling in one call.
9. Prefer `core.async` channels for streaming responses and long-polling — return a channel from the handler and close it when done to avoid tying up threads.
10. Test handlers as plain functions: pass a request map `{:request-method :get :uri "/users"}` and assert on the response map's `:status` and `:body` keys.
11. Use the REPL-driven workflow: start the system in the REPL with `(integrant.repl/go)`, redefine functions, and `(reset)` to reload changed namespaces without restarting the JVM.
12. Deploy as an uberjar with `lein uberjar` or `clj -T:build uber` — embed the HTTP server (Jetty or http-kit) so the artifact runs with a simple `java -jar app.jar` command.
