---
triggers: [".NET Aspire", "aspire", "aspire dashboard", "aspire orchestration", "aspire service defaults"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["dotnet"]
category: csharp
---

# .NET Aspire

When working with .NET Aspire:

1. Create an Aspire solution with `dotnet new aspire-starter`: this generates an AppHost project, ServiceDefaults project, and sample services; the AppHost is the orchestration entry point.
2. Define service topology in the AppHost `Program.cs`: use `builder.AddProject<Projects.ApiService>("api")` and `builder.AddProject<Projects.WebFrontend>("web").WithReference(api)` to wire service dependencies.
3. Add backing resources declaratively: `builder.AddRedis("cache")`, `builder.AddPostgres("db").AddDatabase("appdb")`, `builder.AddRabbitMQ("messaging")` provision containers automatically during local development.
4. Use the ServiceDefaults project for shared configuration: call `builder.AddServiceDefaults()` to inject OpenTelemetry, health checks, resilience policies, and service discovery into every project uniformly.
5. Pass connection information via resource references: `WithReference(cache)` injects connection strings as configuration; access in services with `builder.Configuration.GetConnectionString("cache")`.
6. Leverage the Aspire dashboard for local observability: it starts automatically and shows distributed traces, structured logs, and metrics for all orchestrated services at `https://localhost:15888`.
7. Configure health checks in ServiceDefaults: `builder.Services.AddHealthChecks().AddNpgSql(connectionString).AddRedis(connectionString)` ensures readiness probes work for all dependencies.
8. Use Aspire's resilience integration: `builder.Services.ConfigureHttpClientDefaults(http => http.AddStandardResilienceHandler())` adds retries, circuit breakers, and timeouts to all `HttpClient` instances.
9. Add custom containers and executables: `builder.AddContainer("legacy-api", "legacy/api", "v2")` and `builder.AddExecutable("worker", "dotnet", "run")` integrate non-.NET components into the topology.
10. Use `WithEndpoint` to configure ports and schemes: `api.WithEndpoint("https", e => { e.Port = 5001; e.IsProxied = false; })` controls how services bind and discover each other.
11. Deploy to Azure Container Apps with `azd up`: Aspire's manifest (`aspire-manifest.json`) maps resources to cloud equivalents; Redis becomes Azure Cache, Postgres becomes Azure Database for PostgreSQL.
12. Write integration tests using `DistributedApplicationTestingBuilder`: call `await using var app = await DistributedApplicationTestingBuilder.CreateAsync<Projects.AppHost>()` to spin up the full topology and test service interactions end-to-end.
