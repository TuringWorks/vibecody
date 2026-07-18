---
layout: default
title: "IDP — Internal Developer Platform"
parent: Demos
---

# IDP — Internal Developer Platform

The IDP panel provides a comprehensive Internal Developer Platform for managing services, teams, infrastructure, and platform integrations. All data is persisted locally to `~/.vibecli/idp/`.

## Capabilities

### 1. Service Catalog
Register and manage all services in your organization.

- **Register services** with name, owner, tier (0-3), language, framework, repo URL, and description
- **Search** by name, owner, or language
- **Tier classification**: Tier 0 (Critical) through Tier 3 (Low) with color-coded badges
- **Status tracking**: Active, Deprecated, Incubating, Sunset
- **Delete services** that are no longer needed
- **Persistence**: Services are saved to `~/.vibecli/idp/services.json`

**How to use:**
1. Open the IDP panel and go to "Service Catalog"
2. Click "+ Register Service"
3. Fill in the service details (Name and Owner are required)
4. Click "Register Service" — the service appears in the table
5. Use the search bar to filter services

### 2. Golden Paths
Pre-built, opinionated project templates encoding production best practices.

**8 built-in templates:**
- **React + Vite** (TypeScript) — SPA with Vitest, ESLint, Tailwind, CI/CD, Docker
- **Next.js 15** (TypeScript) — Full-stack with App Router, Prisma, NextAuth, Vercel
- **Actix Web** (Rust) — REST API with SQLx, OpenTelemetry, OpenAPI, Docker multi-stage
- **Chi + sqlc** (Go) — REST API with type-safe SQL, slog, K8s manifests
- **FastAPI** (Python) — Microservice with SQLAlchemy, Alembic, pytest, Docker
- **Spring Boot 3** (Java) — Enterprise with Spring Data JPA, Security, Testcontainers
- **Express + Prisma** (TypeScript) — Node.js API with Zod, Jest, Swagger
- **Ktor** (Kotlin) — Microservice with Exposed ORM, Koin DI, coroutines

**How to use:**
1. Go to "Golden Paths" tab
2. Filter by language using the search field
3. Review template features and descriptions
4. Use the template repo reference to scaffold new projects

### 3. Scorecards
Evaluate service quality against governance, standards, and DORA metrics.

**9 metrics across 4 categories:**
- **Quality**: Documentation, Source Control
- **Governance**: Ownership, Tier Classification
- **Standards**: Tech Stack Defined
- **DORA**: Deploy Frequency, Lead Time for Changes, Mean Time to Recovery, Change Failure Rate

**Grading**: A (90+), B (80+), C (70+), D (60+), F (<60) out of 100

**How to use:**
1. Register services in the Service Catalog first
2. Go to "Scorecards" tab
3. Select a service from the dropdown
4. Click "Evaluate" to compute scores based on service metadata
5. Review recommendations and improve scores by adding missing metadata

### 4. Self-Service Infrastructure
Request pre-approved infrastructure resources through a self-service portal.

**10 infrastructure templates:**
PostgreSQL Database, Redis Cache, S3 Bucket, Kubernetes Namespace, API Gateway, CDN Distribution, Message Queue, Monitoring Stack, CI/CD Pipeline, Load Balancer

**Configurable options:**
- Environment: Development, Staging, Production
- Region: us-east-1, us-west-2, eu-west-1, eu-central-1, ap-southeast-1
- Size: Small, Medium, Large, XLarge

**How to use:**
1. Go to "Infrastructure" tab
2. Click "+ New Request"
3. Select template, environment, region, and size
4. Click "Submit Request" — the request is tracked in the table
5. Requests are persisted to `~/.vibecli/idp/infra_requests.json`

### 5. Teams & Onboarding
Create teams and track their onboarding progress through a structured checklist.

**8-step onboarding checklist:**
1. Set up source control access
2. Configure CI/CD pipeline
3. Register services in catalog
4. Set up monitoring & alerting
5. Configure development environment
6. Review golden path templates
7. Set up staging environment
8. Complete security onboarding

**How to use:**
1. Go to "Teams" tab
2. Click "+ Create Team" and enter the team name
3. Click "Onboarding" on a team card to expand the checklist
4. Click any checklist item to toggle completion
5. Progress bar updates automatically
6. Teams are persisted to `~/.vibecli/idp/teams.json`

### 6. Platform Integrations
Enable and configure 13 supported IDP platforms.

**Supported platforms:**
| Platform | Key Features |
|----------|-------------|
| Backstage | Service Catalog, Templates, TechDocs, Plugins |
| Cycloid | FinOps, GitOps, Stacks, Compliance |
| Humanitec | Score, Resource Graphs, Deployments, Environments |
| Port | Self-Service, Scorecards, Automations, Catalog |
| Qovery | Environments, Deployments, Preview Envs, Cost Mgmt |
| Mia Platform | Microservices, Console, Marketplace, Fast Data |
| OpsLevel | Service Maturity, Ownership, Checks, Actions |
| Roadie | Managed Backstage, Plugins, Scaffolder, TechDocs |
| Cortex | Scorecards, CQL, Plugins, Initiatives |
| Morpheus Data | Hybrid Cloud, Automation, Analytics, Governance |
| CloudBolt | Self-Service IT, Cost Mgmt, Multi-Cloud, Terraform |
| Harness | CI/CD, Feature Flags, Cloud Cost, SRM |
| Custom | API Gateway, Custom Catalog, Webhooks, RBAC |

**How to use:**
1. Go to "Platforms" tab
2. Toggle platforms on/off — enabled platforms are highlighted
3. Platform state is persisted to `~/.vibecli/idp/platforms.json`

### 7. Backstage Integration
Generate Backstage-compatible `catalog-info.yaml` files for your services.

**Generated YAML includes:**
- `apiVersion: backstage.io/v1alpha1`
- Component metadata with name, description, annotations
- GitHub project slug from repo URL
- TechDocs reference
- Language and framework tags
- Lifecycle mapping (Active → production, Incubating → experimental)
- Owner, system, and API references

**How to use:**
1. Register services in the Service Catalog
2. Go to "Backstage" tab
3. Select a service from the dropdown and click "Generate"
4. Or click "Generate YAML" next to any component in the table
5. Copy the YAML and commit it as `catalog-info.yaml` in your repo

## Data Storage

All IDP data is stored as JSON files in `~/.vibecli/idp/`:
- `services.json` — Service catalog entries
- `teams.json` — Teams with onboarding checklists
- `infra_requests.json` — Infrastructure provisioning requests
- `platforms.json` — Platform enable/disable state

## CLI Commands

The IDP is also accessible via VibeCLI REPL:
- `/idp status` — Overview of catalog, teams, and platforms
- `/idp catalog` — List registered services
- `/idp register` — Register a new service
- `/idp golden` — Show golden path templates
- `/idp scorecard` — Evaluate service scorecards
- `/idp infra` — Manage infrastructure requests
- `/idp team` — List teams
- `/idp onboard` — Start team onboarding
- `/idp backstage` — Generate Backstage config
- `/idp platforms` — List supported platforms
- `/idp report` — Generate IDP health report
