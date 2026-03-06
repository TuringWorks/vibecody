---
triggers: ["REST API", "API design", "endpoint", "HTTP method", "status code"]
tools_allowed: ["read_file", "write_file", "bash"]
category: api-design
---

# REST API Design

1. Use nouns for resources: `/users`, `/orders` — not `/getUsers`
2. HTTP methods: GET (read), POST (create), PUT (full update), PATCH (partial), DELETE
3. Status codes: 200 OK, 201 Created, 204 No Content, 400 Bad Request, 401 Unauthorized, 403 Forbidden, 404 Not Found, 409 Conflict, 422 Unprocessable, 429 Rate Limited, 500 Internal
4. Paginate list endpoints: `?page=1&per_page=20` or cursor-based `?cursor=abc`
5. Use consistent error format: `{ "error": { "code": "NOT_FOUND", "message": "..." } }`
6. Version via URL prefix: `/api/v1/...` or `Accept` header
7. Use HATEOAS links for discoverability when practical
8. Filter with query params: `?status=active&sort=-created_at`
9. Rate limit and document limits in response headers: `X-RateLimit-*`
