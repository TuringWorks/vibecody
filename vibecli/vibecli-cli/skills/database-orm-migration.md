---
triggers: ["ORM", "database migration", "zero downtime migration", "Prisma", "Drizzle", "SQLAlchemy", "ActiveRecord migration"]
tools_allowed: ["read_file", "write_file", "bash"]
category: database
---

# ORM Patterns & Database Migrations

When working with ORMs and migrations:

1. Use migrations for ALL schema changes — never modify the database manually
2. Zero-downtime migration pattern: add new column → backfill → update code → drop old column
3. Always make migrations reversible — implement both `up` and `down` methods
4. Use ORM for CRUD operations; raw SQL for complex queries and bulk operations
5. Lazy loading vs eager loading: default to eager loading with `include`/`select_related` to avoid N+1
6. Index migration: add indexes concurrently to avoid table locks in production
7. Data migrations: separate from schema migrations — run them independently
8. Use transactions in migrations: if any step fails, the whole migration rolls back
9. Test migrations against a copy of production data — schema may work, data may not
10. Naming convention: `YYYYMMDDHHMMSS_descriptive_name` for chronological ordering
11. Never rename columns directly in production — use the expand-contract pattern
12. Use `NOT NULL` with default values — adding a nullable column is always safe
