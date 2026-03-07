---
triggers: ["F#", "fsharp", "giraffe", "falco", "saturn", "suave", "fsharp web", "SAFE stack"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["dotnet"]
category: fsharp
---

# F# Web Development

When working with F# web frameworks:

1. Use Giraffe's `HttpHandler` composition with `>=>` (fish operator) to chain handlers — `route "/api" >=> setStatusCode 200 >=> json data` reads as a left-to-right pipeline.
2. Define routes using `choose [ route "/"; subRoute "/api" apiRoutes ]` to build a declarative routing tree that short-circuits on the first matching handler.
3. Use `bindJsonAsync<'T>` in Giraffe or `Request.mapJson` in Falco to deserialize request bodies into discriminated unions or record types with automatic validation.
4. Model domain types with F# discriminated unions and record types — use pattern matching in handlers to ensure exhaustive handling of all cases at compile time.
5. In Saturn, use the `application` and `controller` computation expressions to define RESTful resources with `index`, `show`, `create`, `update`, `delete` actions in a structured DSL.
6. Use Falco for high-performance minimal APIs — its `mapRoute`, `bindQuery`, and response helpers avoid the overhead of Giraffe's `HttpContext` pipeline for hot paths.
7. Apply the SAFE stack (Saturn/Azure/Fable/Elmish) for full-stack F# — share domain types between server and Fable client via a shared project to eliminate serialization mismatches.
8. Use `task { }` computation expressions (not `async { }`) for ASP.NET Core handlers to avoid the overhead of converting between F# `Async` and .NET `Task` at every await point.
9. Inject dependencies through ASP.NET Core DI — access services in Giraffe handlers with `ctx.GetService<IMyService>()` and register them in `ConfigureServices`.
10. Test HTTP handlers with `Microsoft.AspNetCore.TestHost.TestServer` — create a test host, send requests via `HttpClient`, and assert on status codes and deserialized JSON bodies.
11. Use Thoth.Json or System.Text.Json with F# source generators for JSON serialization — define encoders/decoders explicitly for discriminated unions that have no direct JSON mapping.
12. Structure projects with a `Domain` module (pure types and logic), an `Application` module (use cases), and a `Web` module (handlers and routing) to maintain separation of concerns.
