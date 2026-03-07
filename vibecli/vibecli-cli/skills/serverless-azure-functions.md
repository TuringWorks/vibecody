---
triggers: ["Azure Functions", "azure function app", "azure durable functions", "azure event grid", "azure function binding"]
tools_allowed: ["read_file", "write_file", "bash"]
category: devops
---

# Azure Functions

When working with Azure Functions:

1. Choose the right hosting plan: Consumption (auto-scale, pay-per-execution, cold starts), Premium (pre-warmed instances, VNET), or Dedicated (App Service plan for predictable workloads). Use Premium for production APIs needing consistent latency.

2. Use the v4 programming model for Node.js with `app.http()`, `app.timer()`, `app.storageQueue()` — it co-locates triggers with handler code instead of separate `function.json` files: `app.http('getUser', { methods: ['GET'], route: 'users/{id}', handler: getUserHandler })`.

3. Leverage input and output bindings to avoid boilerplate SDK code — read from Cosmos DB and write to Queue Storage declaratively: `extraInputs: [cosmosInput]` and `extraOutputs: [queueOutput]` in the function registration.

4. Implement Durable Functions for complex orchestrations: define orchestrator functions that call activity functions in sequence, parallel, or fan-out/fan-in patterns: `const results = yield context.df.Task.all(cities.map(c => context.df.callActivity("GetWeather", c)))`.

5. Use Durable Entities for stateful singletons (counters, aggregators, device state) — they provide actor-model semantics with guaranteed single-threaded access: `context.df.signalEntity(entityId, "add", 1)`.

6. Configure `host.json` for performance: set `"maxConcurrentRequests"` and `"dynamicThrottlesEnabled"`, batch size for queue triggers, and logging levels. Enable Application Insights sampling to control telemetry costs.

7. Store secrets in Azure Key Vault and reference them in app settings with `@Microsoft.KeyVault(SecretUri=https://myvault.vault.azure.net/secrets/mysecret/)` — the runtime resolves them at startup without custom code.

8. Use Event Grid triggers for reactive event-driven architectures — subscribe to Azure resource events (blob created, resource deployed) or custom topics. Event Grid guarantees at-least-once delivery with 24-hour retry.

9. Deploy with `func azure functionapp publish <app-name>` or use GitHub Actions with `Azure/functions-action@v1`. Enable deployment slots for staging and swap to production with zero downtime.

10. Handle cold starts by keeping dependencies minimal, using the `WEBSITE_RUN_FROM_PACKAGE=1` setting for faster startup, and enabling the `"always ready"` instances count on Premium plan.

11. Implement retry policies on triggers: `"retry": {"strategy": "exponentialBackoff", "maxRetryCount": 5, "minimumInterval": "00:00:10", "maximumInterval": "00:15:00"}` — combine with poison message queues for unrecoverable failures.

12. Test locally with Azure Functions Core Tools (`func start`) and use the Azurite storage emulator for queue, blob, and table triggers. Write integration tests that target local endpoints: `http://localhost:7071/api/functionName`.
