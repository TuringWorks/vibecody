---
triggers: ["Service Fabric", "service fabric", "reliable services", "reliable actors", "service fabric cluster", "service fabric partition", "azure service fabric"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["sfctl"]
category: cloud-azure
---

# Azure Service Fabric

When working with Azure Service Fabric:

1. Choose between Reliable Services (stateless/stateful) and Reliable Actors based on your concurrency model; use actors for isolated single-threaded state and services for shared-state scenarios with concurrent reads.
2. Design stateful service partitioning early using named or ranged partitions, ensuring even key distribution to prevent hot partitions; use `Int64RangePartitionScheme` for numeric keys and `NamedPartitionScheme` for categorical routing.
3. Implement `RunAsync` with the provided `CancellationToken` and check it frequently in loops to ensure graceful shutdown during upgrades and node decommissions.
4. Use Reliable Collections (`IReliableDictionary`, `IReliableQueue`) with transactions for stateful services; always call `CommitAsync` inside a `using` block on `ITransaction` and keep transaction lifetimes short.
5. Configure rolling upgrade domains and health policies in `ApplicationManifest.xml`; set `HealthCheckStableDurationSec`, `UpgradeDomainTimeoutSec`, and `HealthCheckWaitDurationSec` to catch regressions before proceeding.
6. Register health reports proactively via `IServicePartition.ReportHealth` with appropriate TTL values; use `HealthState.Warning` for degraded states and `HealthState.Error` for failures to integrate with cluster health monitoring.
7. Configure the built-in reverse proxy (`ApplicationGateway/Http`) for service-to-service communication using the `fabric:/AppName/ServiceName` URI scheme, and set `SecureOnlyMode` to enforce TLS in production clusters.
8. Integrate ASP.NET Core services using `KestrelCommunicationListener` or `HttpSysCommunicationListener`; register the listener in `CreateServiceInstanceListeners` and bind to the endpoint declared in `ServiceManifest.xml`.
9. Deploy guest executables by packaging them in a `Code` folder under the service package with a proper `ServiceManifest.xml` specifying `EntryPoint/ExeHost`; use `ConsoleRedirection` for log capture.
10. Use placement constraints (`NodeType`, custom properties) and load metrics to control where services land; report dynamic load via `IServicePartition.ReportLoad` so the cluster resource manager can rebalance.
11. Implement `ICommunicationListener` for custom protocols and handle the `OpenAsync`/`CloseAsync`/`Abort` lifecycle correctly; return the published address including the partition ID for stateful service resolution.
12. Monitor cluster health with `sfctl cluster health` and `sfctl application health`; automate chaos testing using the Fault Analysis Service APIs (`StartPartitionQuorumLossAsync`, `RestartNodeAsync`) to validate resilience before production upgrades.
