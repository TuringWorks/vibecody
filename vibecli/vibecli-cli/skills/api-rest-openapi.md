---
triggers: ["REST API", "OpenAPI", "API design", "REST best practices", "HTTP API", "resource naming", "API versioning"]
tools_allowed: ["read_file", "write_file", "bash"]
category: api
---

# REST API & OpenAPI Design

When designing REST APIs:

1. Resource naming: plural nouns (`/users`, `/orders`), not verbs — HTTP methods convey the action
2. HTTP methods: GET (read), POST (create), PUT (full update), PATCH (partial update), DELETE (remove)
3. Status codes: 200 (OK), 201 (Created), 204 (No Content), 400 (Bad Request), 401 (Unauthorized), 404 (Not Found), 409 (Conflict), 500 (Server Error)
4. Pagination: use cursor-based (`?after=xyz`) for large datasets; offset-based (`?page=2&limit=20`) for simple cases
5. Filtering: query parameters — `GET /users?status=active&role=admin`
6. Sorting: `?sort=created_at:desc,name:asc`
7. Error response format: `{ "error": { "code": "VALIDATION_ERROR", "message": "...", "details": [...] } }`
8. Versioning: URL prefix (`/v1/users`) or Accept header — be consistent
9. HATEOAS: include links for related resources — `{ "self": "/users/123", "orders": "/users/123/orders" }`
10. OpenAPI 3.1: define spec as source of truth — generate docs, client SDKs, mock servers
11. Rate limiting: return `429 Too Many Requests` with `Retry-After` header
12. Idempotency: support `Idempotency-Key` header for POST requests — safe retries
