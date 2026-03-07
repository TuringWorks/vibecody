---
triggers: ["JHipster", "jhipster", "jhipster generator", "jhipster microservice", "jhipster monolith", "jdl"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["java", "node"]
category: java
---

# JHipster Full-Stack Generator

When working with JHipster:

1. Use JDL (JHipster Domain Language) for entity modeling — define entities, relationships, and options in `.jdl` files and import with `jhipster jdl my-model.jdl`; this is more maintainable than interactive entity prompts for non-trivial schemas.
2. Choose the right application type at generation: `monolith` for simpler deployments, `microservice` with a gateway for distributed architectures; microservices use Spring Cloud (Consul/Eureka) for service discovery by default.
3. Customize generated code cautiously — use side-by-side files and the `_override` pattern rather than editing generated files directly; re-running `jhipster` will overwrite changes unless they are in `.jhipster/` blueprints or custom files.
4. Leverage JHipster's built-in security setup: JWT (default), OAuth 2.0/OIDC (Keycloak), or session-based auth; switch at generation time and get full login/registration flows, user management, and role-based access for free.
5. Use `jhipster entity MyEntity` to scaffold entities with CRUD endpoints, repository, service layer, DTOs, and Angular/React/Vue frontend components in one command; add `--skip-client` or `--skip-server` for partial generation.
6. Configure CI/CD with `jhipster ci-cd` to generate pipelines for Jenkins, GitHub Actions, GitLab CI, Travis, or Azure; the generated pipeline includes build, test, Docker image creation, and deployment steps.
7. Run the full stack locally with `./mvnw` (backend) and `npm start` (frontend with hot-reload); use Docker Compose files in `src/main/docker/` for dependencies like PostgreSQL, Elasticsearch, Kafka, and Keycloak.
8. Write backend tests using the generated test infrastructure — JHipster creates integration tests with `@SpringBootTest`, Testcontainers for databases, and `MockMvc` for REST endpoints; run with `./mvnw verify`.
9. Use JHipster's Liquibase integration for database migrations — entity changes generate changelog files in `src/main/resources/config/liquibase/changelog/`; review generated SQL and add custom changesets for data migrations.
10. For microservices, generate a gateway application to handle routing, rate limiting, and frontend serving; configure routes in `application.yml` and use the gateway's built-in circuit breaker (Resilience4j) for fault tolerance.
11. Customize the frontend by modifying components in `src/main/webapp/app/`; JHipster generates TypeScript entities with full CRUD screens — extend these rather than building from scratch to maintain upgrade compatibility.
12. Deploy to production using `./mvnw -Pprod verify jib:dockerBuild` for containerized builds or `jhipster kubernetes` to generate Kubernetes manifests; use JHipster's Helm charts for multi-service deployments with Istio service mesh support.
