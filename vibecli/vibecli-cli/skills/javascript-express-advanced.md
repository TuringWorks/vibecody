---
triggers: ["Express middleware", "express router", "express error handling", "express async", "express production"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["node"]
category: javascript
---

# Express.js Advanced Patterns

When working with Express.js advanced patterns:

1. Wrap async handlers to catch promise rejections: `const asyncHandler = (fn) => (req, res, next) => Promise.resolve(fn(req, res, next)).catch(next)` — use this on every async route to avoid unhandled rejections.
2. Structure error handling with a final four-argument middleware: `app.use((err, req, res, next) => { ... })` — classify errors by type (`ValidationError`, `AuthError`) and map them to appropriate HTTP status codes and JSON responses.
3. Organize routes with `express.Router()` per resource: create `usersRouter`, `ordersRouter`, then mount with `app.use('/api/users', usersRouter)` to keep route files focused and independently testable.
4. Implement request validation middleware using Joi or Zod: `const validate = (schema) => (req, res, next) => { const result = schema.safeParse(req.body); if (!result.success) return next(result.error); next(); }`.
5. Use `express.json({ limit: '1mb' })` and `express.urlencoded({ extended: false })` with explicit size limits — never accept unbounded payloads in production.
6. Apply rate limiting with `express-rate-limit`: `app.use('/api/', rateLimit({ windowMs: 15 * 60 * 1000, max: 100 }))` — configure per-route limits for sensitive endpoints like login.
7. Set security headers with Helmet: `app.use(helmet())` enables CSP, HSTS, X-Frame-Options, and other headers; customize individual headers with `helmet({ contentSecurityPolicy: { directives: { ... } } })`.
8. Implement graceful shutdown: listen for `SIGTERM`, stop accepting new connections with `server.close()`, drain existing requests, close database pools, then `process.exit(0)`.
9. Use `app.locals` for app-wide config and `res.locals` for request-scoped data — middleware sets `res.locals.user` after auth and all downstream handlers access it without re-parsing.
10. Enable request correlation by generating a unique ID in early middleware: `req.id = crypto.randomUUID()` — pass it to loggers and downstream services via headers for distributed tracing.
11. Configure `trust proxy` correctly when behind a reverse proxy: `app.set('trust proxy', 1)` ensures `req.ip`, `req.protocol`, and rate limiting use the real client IP from `X-Forwarded-For`.
12. Test routes with supertest: `const res = await request(app).get('/api/users').set('Authorization', 'Bearer token').expect(200)` — this runs the full middleware chain without starting a listening server.
