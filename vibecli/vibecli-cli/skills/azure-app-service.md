---
triggers: ["App Service", "azure app service", "Container Apps", "azure container apps", "deployment slot", "azure web app", "dapr azure"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["az"]
category: cloud-azure
---

# Azure App Service + Container Apps

When working with Azure App Service and Container Apps:

1. Use deployment slots for zero-downtime releases: create staging slot (`az webapp deployment slot create --slot staging`), deploy to staging, validate, then swap (`az webapp deployment slot swap --slot staging --target-slot production`); configure slot-sticky settings for connection strings that should not swap (e.g., staging vs production databases).
2. Configure auto-scale rules based on metrics: `az monitor autoscale create --min-count 2 --max-count 10 --count 2` with rules like `--condition "CpuPercentage > 70 avg 5m" --scale out 2` and `--condition "CpuPercentage < 30 avg 10m" --scale in 1`; use schedule-based scaling for predictable traffic patterns.
3. Enable health checks at `/health` endpoint: `az webapp config set --health-check-path /health`; App Service pings this path every minute and removes unhealthy instances from the load balancer after 5 failures — implement deep health checks that verify database and dependency connectivity.
4. Configure custom domains with managed certificates: `az webapp config hostname add --hostname app.example.com` then `az webapp config ssl create --hostname app.example.com` for free managed TLS; for wildcard domains or advanced needs, upload certificates from Key Vault with `az webapp config ssl bind`.
5. Use Container Apps for microservices requiring scale-to-zero, event-driven autoscaling, or Dapr sidecars: `az containerapp create --image myapp:latest --target-port 8080 --ingress external --min-replicas 0 --max-replicas 30` with KEDA scale rules for queue-driven workloads.
6. Configure Container Apps jobs for batch processing: `az containerapp job create --trigger-type Schedule --cron-expression "0 */6 * * *"` for scheduled jobs, or `--trigger-type Event` with KEDA triggers for event-driven jobs; use `--replica-timeout 1800` and `--replica-retry-limit 3` for resilience.
7. Enable Dapr sidecar in Container Apps for service invocation, pub/sub, and state management: `az containerapp dapr enable --dapr-app-id myapp --dapr-app-port 8080`; configure Dapr components (Redis state store, Service Bus pub/sub) via YAML — Dapr handles retries, tracing, and mTLS automatically.
8. Manage Container Apps revisions for traffic splitting: `az containerapp revision copy` creates a new revision, then `az containerapp ingress traffic set --revision-weight latest=20 old=80` for canary deployments; use `--revision-suffix` for named revisions and `--max-inactive-revisions` to control cleanup.
9. Configure App Service application settings and connection strings via `az webapp config appsettings set --settings KEY=VALUE`; use Key Vault references (`@Microsoft.KeyVault(SecretUri=https://vault.vault.azure.net/secrets/name)`) to inject secrets without exposing them in configuration — enable system-assigned managed identity with Key Vault access.
10. Optimize App Service performance: enable Always On (`az webapp config set --always-on true`) to prevent cold starts, configure ARR affinity only if sessions require it (`--generic-configurations '{"stickySessionsEnabled": false}'`), and use local cache (`WEBSITE_LOCAL_CACHE_OPTION=Always`) for read-heavy file access.
11. Implement CI/CD with deployment center: use GitHub Actions (`az webapp deployment github-actions add`), Azure DevOps, or direct container registry webhooks; configure `DOCKER_REGISTRY_SERVER_URL`, `DOCKER_REGISTRY_SERVER_USERNAME`, and `DOCKER_REGISTRY_SERVER_PASSWORD` for private registries.
12. Secure both services with VNet integration: App Service VNet integration (`az webapp vnet-integration add`) for outbound traffic, private endpoints for inbound; Container Apps use managed VNet or custom VNet with NSG rules — restrict ingress with IP restrictions and enable authentication with Easy Auth or Container Apps auth middleware.
