---
triggers: ["Orleans", "orleans", "orleans grain", "orleans silo", "virtual actor", "orleans stream", "orleans persistence", "microsoft orleans"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["dotnet"]
category: csharp
---

# Microsoft Orleans

When working with Microsoft Orleans:

1. Define grain interfaces inheriting `IGrainWithStringKey`, `IGrainWithIntegerKey`, or `IGrainWithGuidKey` based on your domain identity; keep grain methods returning `Task` or `Task<T>` and avoid exposing mutable state objects directly.
2. Co-host Orleans silos with ASP.NET Core using `builder.Host.UseOrleans(siloBuilder => ...)` in `Program.cs`; this shares the same DI container and enables calling grains directly from controllers via `IGrainFactory`.
3. Configure grain persistence with `AddAzureTableGrainStorage`, `AddAdoNetGrainStorage`, or `AddMemoryGrainStorage`; decorate grain classes with `[StorageProvider(ProviderName = "name")]` and use `IPersistentState<T>` for explicit read/write control.
4. Use Orleans Streams (`AddMemoryStreams` or `AddAzureQueueStreams`) for reactive pub/sub; subscribe in `OnActivateAsync` using `GetStreamProvider("name").GetStream<T>(streamId)` and always store subscription handles for cleanup.
5. Prefer `RegisterTimer` for high-frequency in-grain polling (subsecond to seconds) and `RegisterReminder` (via `IReminderGrain`) for durable, coarse-grained wake-ups that survive grain deactivation and silo restarts.
6. Configure cluster membership with `UseDevelopmentClustering` for local dev and `UseAzureStorageClustering` or `UseAdoNetClustering` for production; set `ClusterId` and `ServiceId` consistently across all silos.
7. Control grain placement using built-in strategies (`RandomPlacement`, `PreferLocalPlacement`, `ActivationCountBasedPlacement`) via the `[PreferLocalPlacement]` attribute, or implement `IPlacementDirector` for custom topology-aware routing.
8. Use `[Reentrant]` on grain classes or `[AlwaysInterleave]` on specific methods only when you understand the concurrency implications; default single-threaded execution prevents data races without locks.
9. Write grain unit tests using `TestClusterBuilder` from `Microsoft.Orleans.TestingHost`; configure the test silo with in-memory storage and mock services injected via `ConfigureServices` on the `ISiloBuilder`.
10. Enable the Orleans Dashboard (`UseOrleansDashboard()`) in development to monitor grain activations, method calls, and silo health; in production, export metrics to Application Insights or Prometheus using `AddActivityPropagation`.
11. Implement grain call filters (`IIncomingGrainCallFilter`, `IOutgoingGrainCallFilter`) for cross-cutting concerns like logging, authorization, and retry logic; access `IGrainCallContext` for method name and arguments.
12. Design grain keys to distribute load evenly across silos; avoid "god grains" with millions of calls by sharding state across multiple grain instances using composite keys or consistent hashing in your application logic.
