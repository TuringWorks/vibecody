---
triggers: ["Julia web", "Genie.jl", "HTTP.jl", "Oxygen.jl", "julia server"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["julia"]
category: julia
---

# Julia Web (HTTP.jl, Oxygen, Genie)

When working with Julia web frameworks:

1. Use `HTTP.jl` for low-level servers: `HTTP.serve(handler, "0.0.0.0", 8080)` where handler is `(req::HTTP.Request) -> HTTP.Response` — this gives full control over request processing.
2. In Oxygen.jl, define routes with macros: `@get "/users/{id}" function(req, id::Int)` — path parameters are automatically parsed to the declared Julia type.
3. For Genie.jl, use the MVC structure: place routes in `routes.jl`, controllers in `app/resources/*/Controller.jl`, and models in `app/resources/*/Model.jl` for clean separation.
4. Parse JSON request bodies with `JSON3.read(String(req.body), MyStruct)` — define Julia structs with `StructTypes.StructType` to control deserialization behavior.
5. Use Genie's SearchLight ORM for database access: define models with `@kwdef mutable struct User <: AbstractModel`, run migrations with `SearchLight.Migration.up()`, and query with `find(User, id)`.
6. Implement middleware in Oxygen with `@middleware function(handler, req) ... handler(req)` and in HTTP.jl by composing handler functions: `server = req -> auth(req) |> cors |> router`.
7. Serve static files in Genie by placing them in `public/` — they are served automatically; in HTTP.jl use `HTTP.serve(HTTP.FileServer("public/"), "0.0.0.0", 8080)`.
8. Handle WebSocket connections with `HTTP.WebSockets.listen("0.0.0.0", 8081) do ws ... end` and process messages in a loop with `for msg in ws ... end`.
9. Use Julia's built-in `@async` and `Threads.@spawn` for concurrent request processing; set `JULIA_NUM_THREADS=auto` at startup to utilize all CPU cores.
10. Manage configuration per environment in Genie with `config/env/dev.jl` and `config/env/prod.jl` — access values via `Genie.config` for database URLs, secrets, and feature flags.
11. Reduce first-request latency by precompiling routes: use `PackageCompiler.create_sysimage` with a precompile script that hits all endpoints to eliminate JIT compilation in production.
12. Write handler tests using `Test` stdlib: construct `HTTP.Request("GET", "/path")`, pass to the handler function, and `@test response.status == 200` — no running server needed for unit tests.
