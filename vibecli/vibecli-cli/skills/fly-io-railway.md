---
triggers: ["Fly.io", "flyctl", "fly deploy", "fly machine", "Railway", "railway deploy", "railway service", "fly multi-region"]
tools_allowed: ["read_file", "write_file", "bash"]
category: cloud-paas
---

# Fly.io and Railway Deployment

When working with Fly.io and Railway:

1. Launch a Fly.io application with `fly launch` which auto-detects the framework, generates a `fly.toml` config, and provisions a Machine; for existing projects, create manually with `fly apps create myapp` then configure `fly.toml` with `[build] dockerfile = "Dockerfile"`, `[[services]]` for port mapping, and `[env]` for environment variables.
2. Deploy to Fly.io with `fly deploy` which builds the Docker image remotely, rolls out the new version with health checks, and supports canary deployments with `fly deploy --strategy canary`; use `fly deploy --ha=false` for single-machine hobby projects to minimize costs.
3. Use the Fly Machines API for fine-grained control: `fly machine run . --name worker --region ord --vm-size shared-cpu-1x --vm-memory 512` to create individual machines, `fly machine stop $MACHINE_ID` to pause billing, and `fly machine clone $MACHINE_ID --region ams` for cross-region replication.
4. Configure multi-region deployments in `fly.toml` with `primary_region = "iad"` and add regions with `fly regions add ams sin`; use `fly-replay` response header (`fly-replay: region=iad`) to route write requests to the primary region while serving reads from the nearest edge.
5. Attach persistent storage with `fly volumes create mydata --region iad --size 10` and mount in `fly.toml`: `[mounts] source = "mydata" destination = "/data"`; note volumes are per-region and per-machine, so use Fly Postgres or LiteFS for replicated data across regions.
6. Provision Fly Postgres with `fly postgres create --name mydb --region iad --vm-size shared-cpu-1x --initial-cluster-size 2` for HA, attach to apps with `fly postgres attach mydb --app myapp` which sets `DATABASE_URL` automatically, and add read replicas with `fly machine clone $PG_MACHINE --region ams`.
7. Set secrets securely with `fly secrets set DATABASE_URL="postgres://..." API_KEY="secret"` which triggers a rolling restart; list secrets with `fly secrets list` (values are never shown), and use `fly secrets unset KEY` to remove; secrets are encrypted at rest and injected as environment variables.
8. Configure auto-scaling in `fly.toml`: `[http_service] auto_stop_machines = true auto_start_machines = true min_machines_running = 1` for scale-to-zero behavior, or use `fly scale count 3` for fixed scaling and `fly scale vm shared-cpu-2x` to resize machine specs.
9. Deploy to Railway with `railway init` to create a project, `railway link` to connect a repo, and `railway up` for CLI deploys; Railway auto-detects Nixpacks build configuration or uses a Dockerfile, and each push to a linked branch triggers automatic deployments.
10. Manage Railway services and databases with `railway add --database postgres` or `railway add --database redis` to provision managed data stores; access connection strings via `railway variables` and use `railway connect postgres` to open a direct psql session for debugging.
11. Configure Railway health checks in `railway.toml`: `[deploy] healthcheckPath = "/health" healthcheckTimeout = 300 restartPolicyType = "ON_FAILURE" restartPolicyMaxRetries = 3` and set resource limits with `numReplicas = 2` for horizontal scaling; use `railway logs --follow` to monitor deployment status.
12. Optimize costs on both platforms by using Fly.io's `auto_stop_machines = true` to halt idle machines (billed per-second), leveraging Railway's usage-based pricing with `railway up --detach` for async deploys, setting memory limits to avoid over-provisioning (`fly scale memory 256`), monitoring spend with `fly billing` and Railway's Usage dashboard, and using shared-cpu machine types for non-CPU-intensive workloads.
