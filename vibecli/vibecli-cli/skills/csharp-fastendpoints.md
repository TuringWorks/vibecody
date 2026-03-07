---
triggers: ["FastEndpoints", "fastendpoints", ".NET minimal api", "dotnet AOT", "kestrel performance"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["dotnet"]
category: csharp
---

# FastEndpoints and High-Performance .NET

When working with FastEndpoints:

1. Define one endpoint per class inheriting `Endpoint<TRequest, TResponse>` — override `Configure()` for route/verb/auth and `HandleAsync()` for logic to keep concerns separated.
2. Use the built-in `Validator<TRequest>` with FluentValidation rules — FastEndpoints runs validation automatically before `HandleAsync` and returns 400 with structured errors.
3. Prefer `AddSingleton` and pre-resolved dependencies for hot paths; avoid `AddScoped` for services that do not genuinely need per-request lifetime in high-throughput scenarios.
4. Use `SendAsync()`, `SendOkAsync()`, `SendNotFoundAsync()`, and `SendErrorsAsync()` helper methods instead of manually constructing `IResult` or `IActionResult` objects.
5. Group related endpoints with `Group("/api/v1/orders", ...)` to share route prefixes, middleware, and auth policies without repeating configuration in each endpoint.
6. Enable Native AOT compilation (`<PublishAot>true</PublishAot>`) and replace reflection-based serialization with source-generated `JsonSerializerContext` for startup and throughput gains.
7. Use pre- and post-processors (`IPreProcessor<TRequest>`, `IPostProcessor<TRequest, TResponse>`) for cross-cutting concerns like logging, audit trails, and response enrichment.
8. Bind route parameters, query strings, headers, and claims directly to request DTO properties using `[FromRoute]`, `[FromQuery]`, `[FromHeader]`, and `[FromClaim]` attributes.
9. Configure Kestrel for performance: set `Limits.MaxConcurrentConnections`, enable `Http2`, tune `MaxRequestBodySize`, and use `ListenUnixSocket` for reverse-proxy setups.
10. Use `HttpContext.Response.StartAsync()` with `PipeWriter` for streaming large payloads instead of buffering entire responses in memory.
11. Test endpoints with `FastEndpoints.Testing` — create a `WebApplicationFactory`, call `Client.POSTAsync<TEndpoint, TRequest>()`, and assert on typed response objects.
12. Implement `ICommandHandler<TCommand, TResult>` for domain operations and call them via `new TCommand { ... }.ExecuteAsync()` to decouple endpoint handlers from business logic.
