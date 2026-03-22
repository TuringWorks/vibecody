---
layout: page
title: "Demo 11 — Docker & Container Management"
permalink: /demos/11-docker/
---


## Overview

VibeCody provides first-class Docker and container management through both the CLI and the VibeUI Docker panel. Build images, run and manage containers, view logs, exec into running containers, and manage Docker Compose stacks. VibeCody also supports **Podman** and **OpenSandbox** as alternative container runtimes, and uses containers as sandboxed execution environments for AI agents.


## Prerequisites

- VibeCody installed (`vibecli --version` returns 0.1+)
- Docker (or Podman) installed and the daemon running:

```bash
docker --version   # Docker 24.0+
docker info        # Verify daemon is running
```

- For VibeUI: `cd vibeui && npm install && npm run tauri dev`
- (Optional) Docker Compose for multi-service management:

```bash
docker compose version   # v2.20+
```


## Step-by-Step Walkthrough

### 1. Docker Panel Overview (VibeUI)

**VibeUI:**
1. Open the **Docker** panel (AI Panel > Docker tab)
2. The panel displays:
   - **Images** — Local images with tags, sizes, and creation dates
   - **Containers** — Running and stopped containers with status, ports, resource usage
   - **Compose** — Active Compose stacks with service status
   - **Volumes** — Named volumes and their mount points
   - **Networks** — Docker networks and connected containers

### 2. Build an Image

**CLI:**

```bash
# Build from a Dockerfile in the current directory
vibecli docker build --tag myapp:latest .

# Build with build args
vibecli docker build --tag myapp:latest --build-arg ENV=production .

# Build with AI-generated Dockerfile
vibecli docker build --generate --lang rust --tag myapp:latest
```

The `--generate` flag asks VibeCody's AI to create an optimized Dockerfile based on your project's language, dependencies, and structure.

**REPL:**

```bash
vibecli
/docker build myapp:latest
```

Example output:

```
Building image: myapp:latest
  Context: . (14 files, 2.3 MB)
  Dockerfile: ./Dockerfile

Step 1/8: FROM rust:1.75-slim AS builder
Step 2/8: WORKDIR /app
Step 3/8: COPY Cargo.toml Cargo.lock ./
Step 4/8: RUN cargo fetch
Step 5/8: COPY src/ ./src/
Step 6/8: RUN cargo build --release
Step 7/8: FROM debian:bookworm-slim
Step 8/8: COPY --from=builder /app/target/release/myapp /usr/local/bin/

Successfully built: myapp:latest (42.1 MB)
```

**VibeUI:**
1. In the Docker panel, click **Build Image**
2. Select the project directory and Dockerfile
3. Enter image name and tag
4. Click **Build** and monitor the build log in real time

### 3. Run a Container

**CLI:**

```bash
# Run a container
vibecli docker run myapp:latest

# Run with port mapping and environment variables
vibecli docker run myapp:latest \
  --port 8080:3000 \
  --env DATABASE_URL=postgres://localhost/mydb \
  --name myapp-dev \
  --detach

# Run interactively
vibecli docker run --interactive --tty ubuntu:22.04 /bin/bash
```

**REPL:**

```bash
/docker run myapp:latest -p 8080:3000 -d --name myapp-dev
```

**VibeUI:**
1. In the Docker panel Images tab, find your image
2. Click the **Run** button next to it
3. Configure ports, environment variables, volumes, and network in the dialog
4. Click **Start**

### 4. Container Logs and Exec

**CLI:**

```bash
# View container logs
vibecli docker logs myapp-dev

# Stream logs in real time
vibecli docker logs myapp-dev --follow

# Tail the last 50 lines
vibecli docker logs myapp-dev --tail 50

# Exec into a running container
vibecli docker exec myapp-dev /bin/sh

# Run a one-off command
vibecli docker exec myapp-dev -- ls -la /app
```

**REPL:**

```bash
/docker logs myapp-dev --follow
/docker exec myapp-dev /bin/sh
```

**VibeUI:**
1. In the Docker panel Containers tab, click a running container
2. The detail view shows:
   - **Logs** tab — live-streaming log output with search and filtering
   - **Exec** tab — an embedded terminal connected to the container
   - **Inspect** tab — container configuration, mounts, networking
   - **Stats** tab — CPU, memory, network I/O graphs

### 5. Docker Compose Management

**CLI:**

```bash
# Start all services defined in docker-compose.yml
vibecli docker compose up

# Start in detached mode
vibecli docker compose up --detach

# View service status
vibecli docker compose ps

# View logs for a specific service
vibecli docker compose logs api --follow

# Scale a service
vibecli docker compose up --scale worker=3

# Stop all services
vibecli docker compose down

# Stop and remove volumes
vibecli docker compose down --volumes
```

**REPL:**

```bash
/docker compose up -d
/docker compose ps
/docker compose logs api --follow
/docker compose down
```

**VibeUI:**
1. In the Docker panel Compose tab, your `docker-compose.yml` stacks are detected automatically
2. Click **Up** to start all services
3. Each service shows its status, ports, and health check results
4. Click a service to view its logs, exec into it, or scale it
5. Click **Down** to tear down the stack

### 6. Container Sandbox for Agent Execution

VibeCody can run AI agents inside sandboxed containers, providing an isolated environment where agents can execute code, install packages, and run tests without affecting your host system.

**CLI:**

```bash
# Run an agent task in a sandboxed container
vibecli agent --sandbox docker --task "Set up a Python project with FastAPI and write tests"

# Specify the sandbox image
vibecli agent --sandbox docker --image python:3.12-slim --task "Fix the failing tests"

# Use Podman instead of Docker
vibecli agent --sandbox podman --task "Refactor the database layer"
```

**Config (`~/.vibecli/config.toml`):**

```toml
[sandbox]
runtime = "docker"           # "docker", "podman", or "opensandbox"
default_image = "ubuntu:22.04"
memory_limit = "2g"
cpu_limit = 2
network = "bridge"
timeout_s = 300
mount_project = true         # Mount the project directory read-only
```

**REPL:**

```bash
/sandbox start --image python:3.12-slim
/sandbox exec pip install fastapi pytest
/sandbox exec pytest tests/
/sandbox stop
```

### 7. Podman and OpenSandbox Support

VibeCody's `ContainerRuntime` trait abstracts over Docker, Podman, and OpenSandbox, providing a unified interface.

**CLI:**

```bash
# Use Podman for all container operations
vibecli docker --runtime podman build --tag myapp:latest .
vibecli docker --runtime podman run myapp:latest

# Use OpenSandbox (cloud-managed containers)
vibecli docker --runtime opensandbox run myapp:latest
```

**Config:**

```toml
[container]
default_runtime = "podman"   # Override the default runtime
```

Runtime-specific features:
- **Docker** — Full Docker API, BuildKit, Compose
- **Podman** — Rootless containers, systemd integration, pods
- **OpenSandbox** — Cloud-managed ephemeral containers with auto-cleanup


## Demo Recording

```json
{
  "id": "demo-docker",
  "title": "Docker & Container Management",
  "description": "Demonstrates building images, running containers, Docker Compose, container logs/exec, and sandboxed agent execution",
  "estimated_duration_s": 180,
  "steps": [
    {
      "action": "Navigate",
      "target": "vibeui://open?folder=/home/user/my-project"
    },
    {
      "action": "Narrate",
      "value": "Let's explore VibeCody's Docker management capabilities. We'll build an image, run a container, and use Docker Compose."
    },
    {
      "action": "Click",
      "target": ".panel-tab[data-panel='docker']",
      "description": "Open the Docker panel"
    },
    {
      "action": "Screenshot",
      "label": "docker-panel-overview"
    },
    {
      "action": "Assert",
      "target": ".docker-status-badge",
      "value": "contains:Connected"
    },
    {
      "action": "Narrate",
      "value": "The Docker panel shows our local images, running containers, and Compose stacks. Let's build a new image."
    },
    {
      "action": "Click",
      "target": "#build-image-btn",
      "description": "Open the Build Image dialog"
    },
    {
      "action": "Type",
      "target": "#image-name-input",
      "value": "myapp:latest"
    },
    {
      "action": "Click",
      "target": "#start-build-btn",
      "description": "Start the build"
    },
    {
      "action": "WaitForSelector",
      "target": ".build-log",
      "timeout_ms": 3000
    },
    {
      "action": "Screenshot",
      "label": "docker-build-in-progress"
    },
    {
      "action": "Narrate",
      "value": "The build log streams in real time, showing each Dockerfile step. Our multi-stage Rust build produces a slim final image."
    },
    {
      "action": "WaitForSelector",
      "target": ".build-success-badge",
      "timeout_ms": 60000
    },
    {
      "action": "Screenshot",
      "label": "docker-build-complete"
    },
    {
      "action": "Narrate",
      "value": "Image built successfully. Now let's run a container from it with port mapping."
    },
    {
      "action": "Click",
      "target": ".image-row[data-image='myapp:latest'] .run-btn",
      "description": "Click Run on the myapp image"
    },
    {
      "action": "Type",
      "target": "#port-mapping-input",
      "value": "8080:3000"
    },
    {
      "action": "Type",
      "target": "#container-name-input",
      "value": "myapp-dev"
    },
    {
      "action": "Click",
      "target": "#start-container-btn",
      "description": "Start the container"
    },
    {
      "action": "Wait",
      "duration_ms": 2000
    },
    {
      "action": "Screenshot",
      "label": "container-running"
    },
    {
      "action": "Assert",
      "target": ".container-row[data-name='myapp-dev'] .status-badge",
      "value": "contains:Running"
    },
    {
      "action": "Click",
      "target": ".container-row[data-name='myapp-dev']",
      "description": "Click the container to see details"
    },
    {
      "action": "Click",
      "target": ".container-detail-tab[data-tab='logs']",
      "description": "View container logs"
    },
    {
      "action": "Wait",
      "duration_ms": 1500
    },
    {
      "action": "Screenshot",
      "label": "container-logs"
    },
    {
      "action": "Click",
      "target": ".container-detail-tab[data-tab='exec']",
      "description": "Switch to the exec terminal"
    },
    {
      "action": "Type",
      "target": ".exec-terminal-input",
      "value": "ls -la /app"
    },
    {
      "action": "Type",
      "target": "keyboard",
      "value": "Enter"
    },
    {
      "action": "Wait",
      "duration_ms": 1000
    },
    {
      "action": "Screenshot",
      "label": "container-exec"
    },
    {
      "action": "Narrate",
      "value": "We can view logs and exec into running containers directly from the UI. Now let's try Docker Compose."
    },
    {
      "action": "Click",
      "target": ".docker-tab[data-tab='compose']",
      "description": "Switch to Compose tab"
    },
    {
      "action": "Click",
      "target": ".compose-stack-row:first-child .up-btn",
      "description": "Start the Compose stack"
    },
    {
      "action": "Wait",
      "duration_ms": 5000
    },
    {
      "action": "Screenshot",
      "label": "compose-stack-running"
    },
    {
      "action": "Assert",
      "target": ".compose-service-status",
      "value": "contains:Running"
    },
    {
      "action": "Narrate",
      "value": "All Compose services are running. Each service shows its status, ports, and health. Finally, let's see the container sandbox for AI agent execution."
    },
    {
      "action": "Click",
      "target": ".panel-tab[data-panel='sandbox']",
      "description": "Switch to the Sandbox panel"
    },
    {
      "action": "Click",
      "target": "#start-sandbox-btn",
      "description": "Start a sandboxed container for agent work"
    },
    {
      "action": "Wait",
      "duration_ms": 3000
    },
    {
      "action": "Screenshot",
      "label": "sandbox-running"
    },
    {
      "action": "Narrate",
      "value": "The sandbox container provides an isolated environment where AI agents can safely execute code, install packages, and run tests without affecting the host system."
    }
  ],
  "tags": ["docker", "containers", "compose", "podman", "sandbox", "devops"]
}
```


## What's Next

- [Demo 12 — Kubernetes Operations](../12-kubernetes/) — Deploy containers to Kubernetes clusters
- [Demo 09 — Autofix & Diagnostics](../09-autofix/) — Run autofix agents in sandboxed containers
- [Demo 10 — Code Transforms](../10-code-transforms/) — Refactor Dockerfiles and Compose files
