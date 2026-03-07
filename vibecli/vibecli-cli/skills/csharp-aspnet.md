---
triggers: ["ASP.NET", "aspnet core", "dotnet web api", "blazor", "minimal api", "entity framework", "SignalR"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["dotnet"]
category: csharp
---

# ASP.NET Core

When working with ASP.NET Core:

1. Use minimal APIs (`app.MapGet`, `app.MapPost`) for simple endpoints and controllers for complex domains — avoid mixing both styles in the same route group.
2. Register services with the correct lifetime: `AddScoped` for per-request (DbContext, repositories), `AddSingleton` for shared state, `AddTransient` for stateless utilities.
3. Use `IOptions<T>` / `IOptionsSnapshot<T>` pattern to bind configuration sections to strongly-typed classes instead of reading `IConfiguration` keys directly in services.
4. Implement global exception handling with `app.UseExceptionHandler` or a custom middleware that catches exceptions, logs them, and returns a `ProblemDetails` JSON response.
5. Use Entity Framework Core migrations (`dotnet ef migrations add`, `dotnet ef database update`) and never modify the database schema manually in production.
6. Apply `[Authorize]` attribute with policy-based authorization (`AddPolicy`) for role/claim checks; avoid hardcoding role strings — define them as constants.
7. Use `CancellationToken` in async controller actions and pass it through to EF queries and HTTP calls so requests can be cancelled when clients disconnect.
8. Configure CORS explicitly with `AddCors` and named policies; never use `AllowAnyOrigin()` combined with `AllowCredentials()` — the spec forbids it.
9. Use health checks (`AddHealthChecks().AddDbContextCheck<AppDbContext>()`) and map them to `/health` for load balancer and orchestrator probes.
10. Prefer `TypedResults` (e.g., `Results<Ok<T>, NotFound, BadRequest<ProblemDetails>>`) in minimal APIs to get accurate OpenAPI schema generation.
11. Use `OutputCache` or `ResponseCaching` middleware for GET endpoints returning stable data; set `VaryByQueryKeys` to avoid serving stale responses for different parameters.
12. Validate input models with FluentValidation or `DataAnnotations` and return `ValidationProblemDetails` — wire validation into the pipeline with a filter rather than checking in each handler.
