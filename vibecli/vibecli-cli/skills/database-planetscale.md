---
triggers: ["PlanetScale", "planetscale", "vitess", "database branching mysql", "planetscale deploy"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# PlanetScale

When working with PlanetScale:

1. PlanetScale is a MySQL-compatible serverless database built on Vitess (the system that scales YouTube's MySQL) — connect with any MySQL driver using the connection string from the dashboard.
2. Database branching: `pscale branch create mydb dev` creates a schema branch; make schema changes on the branch, then create a deploy request: `pscale deploy-request create mydb dev`.
3. Deploy requests (like pull requests for schemas): `pscale deploy-request deploy mydb 1` applies schema changes with zero-downtime; PlanetScale analyzes schema diff and handles online DDL automatically.
4. No foreign key constraints: PlanetScale (Vitess) doesn't support foreign keys at the database level — enforce referential integrity in application code or use Prisma's implicit relations. This enables horizontal sharding.
5. Connection strings: use `DATABASE_URL='mysql://user:password@host/db?ssl={"rejectUnauthorized":true}'` with SSL required; use `@planetscale/database` serverless driver for edge runtimes.
6. Boost (query caching): enable for specific queries to cache results at the edge; `SELECT /*+ SET_VAR(boost_cached_queries=ON) */ * FROM products` — sub-millisecond cached reads.
7. Insights: built-in query analytics shows slow queries, queries per second, rows read/written; identify missing indexes and N+1 query patterns without additional tooling.
8. Use `pscale shell mydb main` to open a MySQL shell directly; `pscale connect mydb main --port 3306` to proxy locally for development tools.
9. Schema design: use `BIGINT UNSIGNED AUTO_INCREMENT` for primary keys (Vitess-compatible); shard tables with `vitess_sharding_key` for horizontal scaling; keep tables that JOIN together on the same shard.
10. Safe migrations: PlanetScale prevents destructive schema changes by default — dropping columns or tables requires explicit confirmation; revert deploy requests within 30 minutes if issues arise.
11. Prisma integration: `datasource db { provider = "mysql" url = env("DATABASE_URL") relationMode = "prisma" }` — use `relationMode = "prisma"` since PlanetScale doesn't support FK constraints.
12. Backups: automatic daily backups with configurable retention; `pscale backup create mydb main` for on-demand backups; restore creates a new branch from the backup.
