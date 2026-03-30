---
layout: page
title: "Demo 57: Internal Developer Platform"
permalink: /demos/57-idp/
nav_order: 57
parent: Demos
---


## Overview

VibeCody includes a full Internal Developer Platform (IDP) module that integrates with 12 platform tools: Backstage, Cycloid, Humanitec, Port, Qovery, Mia Platform, OpsLevel, Roadie, Cortex, Morpheus Data, CloudBolt, and Harness. The IDP provides a unified interface for service catalogs, golden paths, scorecards with DORA metrics, self-service infrastructure provisioning, and team onboarding. It can generate Backstage catalog-info.yaml files, Cycloid blueprints, Humanitec Score files, and Port blueprints from your existing codebase.

**Time to complete:** ~15 minutes

## Prerequisites

- VibeCLI v0.5.1 installed and on your PATH
- At least one AI provider configured
- (Optional) An existing Backstage, Port, or other IDP instance for live integration
- (Optional) VibeUI running with the **IDP** panel visible

## Supported Platforms

| Platform         | Catalog | Golden Paths | Scorecards | Provisioning |
|------------------|---------|--------------|------------|--------------|
| Backstage        | Yes     | Yes          | Yes        | Yes          |
| Port             | Yes     | Yes          | Yes        | Yes          |
| Humanitec        | Yes     | Yes          | Yes        | Yes          |
| Cycloid          | Yes     | Yes          | -          | Yes          |
| Qovery           | Yes     | -            | -          | Yes          |
| Mia Platform     | Yes     | Yes          | -          | Yes          |
| OpsLevel         | Yes     | -            | Yes        | -            |
| Roadie           | Yes     | Yes          | Yes        | -            |
| Cortex           | Yes     | -            | Yes        | -            |
| Morpheus Data    | -       | -            | -          | Yes          |
| CloudBolt        | -       | -            | -          | Yes          |
| Harness          | Yes     | Yes          | -          | Yes          |

## Step-by-Step Walkthrough

### Step 1: List Available Platforms

Open the VibeCLI REPL and check which platforms are configured.

```bash
vibecli
```

```
/idp platforms
```

Expected output:

```
Internal Developer Platforms

  Platform       Status       URL
  Backstage      Connected    https://backstage.internal.acme.com
  Port           Connected    https://app.getport.io
  Humanitec      API key set  https://api.humanitec.io
  Cycloid        Not configured
  Qovery         Not configured
  Mia Platform   Not configured
  OpsLevel       Connected    https://app.opslevel.com
  Roadie         Not configured
  Cortex         Not configured
  Morpheus Data  Not configured
  CloudBolt      Not configured
  Harness        API key set  https://app.harness.io

3 connected, 2 with API keys, 7 not configured.
Configure in ~/.vibecli/config.toml under [idp.<platform>]
```

### Step 2: Browse the Service Catalog

List all services registered across your connected platforms.

```
/idp catalog list
```

```
Service Catalog (3 platforms, 24 services)

  Service              Owner       Platform    Tier   Language   Status
  user-api             team-alpha  Backstage   Tier 1 Rust       Healthy
  payment-service      team-beta   Backstage   Tier 1 Go         Healthy
  notification-worker  team-alpha  Port        Tier 2 Python     Warning
  auth-gateway         team-gamma  Backstage   Tier 1 Rust       Healthy
  search-indexer       team-delta  Port        Tier 2 Java       Healthy
  admin-dashboard      team-beta   Backstage   Tier 3 TypeScript Healthy
  billing-engine       team-beta   Humanitec   Tier 1 Go         Degraded
  ...

Showing 7 of 24. Use --all to see all services.
Filter: /idp catalog list --owner team-alpha --tier 1
```

### Step 3: Create a Golden Path

Golden paths are opinionated templates for creating new services that follow your organization's best practices.

```
/idp golden-path create
```

```
Create Golden Path Template

  Name:        rust-microservice
  Platform:    Backstage
  Description: Production-ready Rust microservice with gRPC, health checks,
               Prometheus metrics, structured logging, and CI/CD pipeline.

  Components included:
    1. Cargo.toml with workspace dependencies
    2. src/main.rs with Axum server, health endpoint, graceful shutdown
    3. src/metrics.rs with Prometheus counters and histograms
    4. Dockerfile (multi-stage, Alpine runtime)
    5. k8s/ manifests (Deployment, Service, HPA, PDB)
    6. .github/workflows/ci.yml (build, test, lint, publish)
    7. catalog-info.yaml (Backstage registration)

  Generated: backstage/templates/rust-microservice/

  To register: /idp golden-path register rust-microservice
  To use:      /idp golden-path apply rust-microservice --name my-new-service
```

### Step 4: Run a Team Scorecard

Scorecards measure team health using DORA metrics (deployment frequency, lead time, change failure rate, MTTR) and custom checks.

```
/idp scorecard team-alpha
```

```
Scorecard: team-alpha
Period: 2026-03-01 to 2026-03-29

DORA Metrics
  Deployment Frequency:    4.2/week     (Elite)
  Lead Time for Changes:   2.1 hours    (Elite)
  Change Failure Rate:     3.8%         (Elite)
  Mean Time to Recovery:   18 minutes   (Elite)

  DORA Classification: Elite Performer

Service Health
  Services owned:     3
  Tier 1 services:    2 (all healthy)
  Tier 2 services:    1 (1 warning: notification-worker)

Compliance Checks
  [PASS] All services have catalog-info.yaml
  [PASS] All services have CI/CD pipeline
  [PASS] All Tier 1 services have runbooks
  [WARN] notification-worker missing SLO definition
  [PASS] All services have designated on-call

Overall Score: 92/100 (A)

Recommendations:
  1. Add SLO definition to notification-worker
  2. Consider promoting notification-worker to Tier 1
```

### Step 5: Provision Infrastructure

Request self-service infrastructure through your IDP.

```
/idp provision
```

```
Self-Service Provisioning

  What would you like to provision?

  1. PostgreSQL database (RDS / CloudSQL / Azure DB)
  2. Redis cache (ElastiCache / Memorystore)
  3. S3-compatible object storage
  4. Kubernetes namespace
  5. CI/CD pipeline
  6. Monitoring stack (Prometheus + Grafana)
  7. Custom (describe what you need)

  Select [1-7]: 1
```

```
Provisioning PostgreSQL Database

  Platform:    Humanitec (Score file)
  Environment: staging
  Instance:    db.t3.medium
  Storage:     50 GB (gp3)
  Version:     PostgreSQL 16
  Backup:      Daily, 7-day retention

  Generated: humanitec/score/postgres-staging.yaml

  Score file preview:
    apiVersion: humanitec.io/v1b1
    kind: Workload
    metadata:
      name: user-api-db
    spec:
      resources:
        postgres:
          type: postgres
          params:
            version: "16"
            storage: "50Gi"

  Apply with: humctl apply -f humanitec/score/postgres-staging.yaml
  Or use:     /idp provision apply
```

### Step 6: Generate Backstage catalog-info.yaml

Automatically generate a Backstage catalog file from your current project.

```
/idp catalog generate --platform backstage
```

```
Generated: catalog-info.yaml

  apiVersion: backstage.io/v1alpha1
  kind: Component
  metadata:
    name: vibecody
    description: AI-powered coding assistant
    tags:
      - rust
      - typescript
      - ai
    annotations:
      github.com/project-slug: TuringWorks/vibecody
      backstage.io/techdocs-ref: dir:.
  spec:
    type: library
    lifecycle: production
    owner: team-platform
    system: developer-tools
    dependsOn:
      - resource:ollama
      - resource:anthropic-api

Written to: catalog-info.yaml
Register at: https://backstage.internal.acme.com/catalog-import
```

### Step 7: View in VibeUI

Open VibeUI and navigate to the **IDP** panel. The panel has 7 tabs:

- **Platforms** -- Connection status for all 12 platforms
- **Catalog** -- Unified service catalog with search and filters
- **Golden Paths** -- Browse, create, and apply golden path templates
- **Scorecards** -- Team scorecards with DORA metrics and trends
- **Provisioning** -- Self-service infrastructure requests
- **Onboarding** -- Team onboarding checklists and progress
- **Settings** -- Platform credentials and sync configuration

## Demo Recording JSON

```json
{
  "meta": {
    "title": "Internal Developer Platform",
    "description": "Unified IDP with 12 platforms, service catalogs, golden paths, and DORA scorecards.",
    "duration_seconds": 360,
    "version": "0.5.1"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/idp platforms", "delay_ms": 3000 },
        { "input": "/idp catalog list", "delay_ms": 3000 },
        { "input": "/idp golden-path create", "delay_ms": 5000 },
        { "input": "/idp scorecard team-alpha", "delay_ms": 4000 },
        { "input": "/idp provision", "delay_ms": 5000 },
        { "input": "/idp catalog generate --platform backstage", "delay_ms": 4000 },
        { "input": "/quit", "delay_ms": 500 }
      ],
      "description": "Complete IDP workflow in the REPL"
    },
    {
      "id": 2,
      "action": "vibeui_interaction",
      "panel": "IDP",
      "tab": "Catalog",
      "description": "Browse unified service catalog"
    },
    {
      "id": 3,
      "action": "vibeui_interaction",
      "panel": "IDP",
      "tab": "Scorecards",
      "description": "View team DORA metrics and scorecard"
    }
  ]
}
```

## What's Next

- [Demo 14: Cloud Provider Integration](../14-cloud-providers/) -- AWS/GCP/Azure scanning and IaC generation
- [Demo 13: CI/CD Pipeline](../13-cicd/) -- GitHub Actions and pipeline monitoring
- [Demo 35: Compliance & Audit](../35-compliance/) -- SOC 2 controls for platform governance
