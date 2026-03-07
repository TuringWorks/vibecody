---
triggers: ["Fastify", "fastify plugin", "fastify schema", "fastify hooks", "fastify decorator"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["node"]
category: javascript
---

# Fastify Framework

When working with Fastify:

1. Register routes with JSON Schema validation: `fastify.post('/users', { schema: { body: userSchema, response: { 200: responseSchema } } }, handler)` — Fastify compiles schemas to fast validation and serialization functions.
2. Encapsulate features as plugins using `fastify.register(plugin, opts)` — each plugin gets its own encapsulated context, preventing decorator and hook leakage between unrelated modules.
3. Use decorators to extend the Fastify instance: `fastify.decorate('db', dbConnection)` for app-wide services and `fastify.decorateRequest('user', null)` for per-request properties set in hooks.
4. Implement authentication with hooks: `fastify.addHook('onRequest', async (req, reply) => { req.user = await verifyToken(req.headers.authorization) })` — hooks run in order and can short-circuit with `reply.send()`.
5. Use `@fastify/autoload` to automatically load plugins and routes from a directory: `fastify.register(autoload, { dir: path.join(__dirname, 'routes') })` — file structure maps to URL prefixes.
6. Leverage Fastify's built-in logging with Pino: access `request.log.info({data}, 'message')` in handlers for request-scoped structured logs with automatic request ID correlation.
7. Handle errors with `fastify.setErrorHandler((error, request, reply) => { ... })` — check `error.validation` for schema errors and `error.statusCode` for HTTP errors to return appropriate responses.
8. Use `fastify.inject()` for testing without starting the server: `const res = await fastify.inject({ method: 'GET', url: '/users' })` — this runs the full request lifecycle in-memory.
9. Define shared schemas with `fastify.addSchema({ $id: 'User', ... })` and reference them via `{ $ref: 'User#' }` in route schemas to avoid duplication across endpoints.
10. Use the `onSend` hook to transform responses globally: `fastify.addHook('onSend', async (req, reply, payload) => { return modifiedPayload })` — useful for envelope wrapping or compression.
11. Enable TypeScript support with `@fastify/type-provider-typebox` or `@fastify/type-provider-json-schema-to-ts` to get full type inference from route schemas without manual type definitions.
12. Configure graceful shutdown with `fastify.addHook('onClose', async () => { await db.close() })` and use `process.on('SIGTERM', () => fastify.close())` to drain connections before exit.
