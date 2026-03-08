---
triggers: ["Prisma", "prisma schema", "prisma migrate", "prisma client", "prisma studio", "prisma orm", "@prisma/client"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["npx"]
category: database
---

# Prisma ORM

When working with Prisma:

1. Define models in `prisma/schema.prisma` with typed fields: `model User { id Int @id @default(autoincrement()) email String @unique name String? posts Post[] }`. Use `@relation` for explicit foreign keys and `@@index` for composite indexes.
2. Generate the client after schema changes: `npx prisma generate`. The generated `@prisma/client` is fully type-safe — all queries, includes, selects, and where clauses are statically typed.
3. Migrations workflow: `npx prisma migrate dev --name add_users` creates SQL migration + applies it; `npx prisma migrate deploy` for production (CI/CD); `npx prisma migrate reset` to reset dev DB.
4. Use `prisma db push` for rapid prototyping without migrations (syncs schema to DB directly); switch to `prisma migrate` when you need reproducible, versioned migrations.
5. Querying: use `findMany({ where, select, include, orderBy, take, skip })` for reads; `create`, `update`, `upsert`, `delete` for writes; `createMany` for bulk inserts; `groupBy` for aggregations.
6. Relations: use `include: { posts: true }` for eager loading; use `select` to pick specific fields and reduce payload; nested writes: `create: { data: { posts: { create: [{ title: '...' }] } } }`.
7. Transactions: use `prisma.$transaction([query1, query2])` for batch operations or `prisma.$transaction(async (tx) => { ... })` for interactive transactions with rollback on error.
8. Raw SQL when needed: `prisma.$queryRaw\`SELECT * FROM users WHERE age > ${age}\`` (parameterized, injection-safe); `$executeRaw` for DDL/DML without return values.
9. Connection pooling: Prisma uses a built-in pool; configure `connection_limit` in the connection URL (`?connection_limit=10`); for serverless, use Prisma Accelerate or PgBouncer.
10. Multi-database support: PostgreSQL, MySQL, SQLite, SQL Server, CockroachDB, MongoDB (preview). Set `provider` in `datasource db { provider = "postgresql" url = env("DATABASE_URL") }`.
11. Use `@@map("table_name")` and `@map("column_name")` to map Prisma model names to existing database table/column names without renaming.
12. Prisma Studio: `npx prisma studio` opens a visual DB browser at localhost:5555 for quick data inspection and editing.
13. Middleware: use `prisma.$use(async (params, next) => { ... })` for logging, soft deletes, audit trails, or query timing.
14. Seeding: define `prisma/seed.ts` and run `npx prisma db seed`; configure in `package.json` under `prisma.seed`.
