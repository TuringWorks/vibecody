---
triggers: ["express", "fastify", "node backend", "middleware", "zod", "node.js API", "REST server node"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["node"]
category: typescript
---

# TypeScript Node.js Backend

When building Node.js backends with TypeScript:

1. Use `zod` for runtime request validation — define schemas, infer types with `z.infer<typeof Schema>`
2. Structure with layers: routes → controllers → services → repositories
3. Use middleware for cross-cutting concerns: auth, logging, error handling, rate limiting
4. Always validate request body, query params, and path params at the boundary
5. Use `express-async-errors` or wrap handlers to catch async errors automatically
6. Return consistent error responses: `{ error: string, code: string, details?: unknown }`
7. Use environment variables via `dotenv` + `zod` schema for type-safe config
8. Implement graceful shutdown: listen for SIGTERM/SIGINT, close connections, drain requests
9. Use `helmet` for security headers, `cors` for cross-origin, `compression` for gzip
10. Database connections: use connection pooling (pg Pool, Prisma, Drizzle)
11. Use `pino` or `winston` for structured JSON logging — never `console.log` in production
12. Rate limit endpoints with `express-rate-limit` — stricter limits on auth endpoints
