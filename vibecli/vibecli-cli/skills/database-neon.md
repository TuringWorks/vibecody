---
triggers: ["Neon", "neon database", "neon postgres", "neon serverless", "neon branching", "serverless postgres"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# Neon (Serverless PostgreSQL)

When working with Neon:

1. Neon is serverless PostgreSQL — fully compatible PostgreSQL with compute that scales to zero and separates storage from compute; connect with any PostgreSQL driver using the connection string from the Neon console.
2. Database branching: `neonctl branches create --name dev` creates an instant copy-on-write branch of your database — use for development, testing, or preview environments without duplicating data.
3. Use the Neon serverless driver for edge/serverless: `import { neon } from '@neondatabase/serverless'; const sql = neon(process.env.DATABASE_URL); const rows = await sql\`SELECT * FROM users\`` — HTTP-based, no TCP connection needed.
4. Autoscaling: configure min/max compute units (CUs) in project settings; Neon scales from 0.25 CU to 8 CU based on load; compute suspends after 5 minutes of inactivity (configurable).
5. Connection pooling: Neon provides built-in PgBouncer pooling — use the pooled connection string (port 5432 with `-pooler` suffix) for serverless/Lambda workloads; use direct connection for migrations.
6. Branching for CI/CD: create a branch per pull request with `neonctl branches create --parent main --name pr-${PR_NUMBER}`; run migrations and tests on the branch; delete on merge.
7. Point-in-time restore: Neon retains WAL history (7-30 days depending on plan); restore to any point: `neonctl branches create --name recovery --parent main --point-in-time '2024-01-15T10:30:00Z'`.
8. Use standard PostgreSQL features: CTEs, window functions, JSONB, full-text search, extensions (`pg_trgm`, `pgvector`, `PostGIS`). Enable extensions: `CREATE EXTENSION IF NOT EXISTS vector`.
9. Neon API for automation: `POST /projects/{id}/branches` to create branches; `PATCH /projects/{id}/endpoints/{id}` to resize compute; integrate with GitHub Actions or CI/CD pipelines.
10. Prisma + Neon: use `@neondatabase/serverless` adapter with Prisma for edge deployments; configure in `schema.prisma` with `previewFeatures = ["driverAdapters"]`.
11. Logical replication: Neon supports `PUBLICATION` and `SUBSCRIPTION` for replicating data to/from other PostgreSQL instances or change data capture pipelines.
12. Cost optimization: auto-suspend saves compute costs during inactivity; storage is billed based on actual data size (not provisioned); branches share storage efficiently via copy-on-write.
