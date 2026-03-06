---
triggers: ["Dockerfile", "docker compose", "multi-stage build", "container image", "docker build", "layer caching"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["docker"]
category: devops
---

# Docker & Containers

When building Docker images and containers:

1. Use multi-stage builds: build stage with dev deps → final stage with runtime only
2. Order Dockerfile instructions by change frequency — COPY code last for layer caching
3. Use `.dockerignore` to exclude `node_modules`, `.git`, `target/`, build artifacts
4. Pin base image versions: `FROM node:20-alpine` not `FROM node:latest`
5. Use `alpine` or `distroless` base images to minimize attack surface
6. Run as non-root: `RUN adduser -D app && USER app`
7. Use `HEALTHCHECK` instruction for container orchestrator integration
8. Combine `RUN` commands with `&&` to reduce layers — clean up in the same layer
9. Use `COPY --from=builder` for multi-stage — only copy built artifacts to final image
10. Docker Compose: use `depends_on` with healthchecks, named volumes for persistence
11. Set resource limits: `deploy.resources.limits` in compose for memory/CPU
12. Use build args for build-time variables, env vars for runtime — never bake secrets into images
