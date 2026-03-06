---
triggers: ["go http", "chi router", "gin", "go REST", "go middleware", "go JSON API", "net/http"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["go"]
category: go
---

# Go Web Services

When building Go web services:

1. Use `net/http` stdlib for simple services; `chi` or `gin` for routing and middleware
2. Structure: `cmd/` (entrypoints), `internal/` (business logic), `pkg/` (shared libraries)
3. Use middleware chains for logging, auth, CORS, recovery, request ID
4. Parse JSON requests with `json.NewDecoder(r.Body).Decode(&req)` — validate after decode
5. Return consistent JSON errors: `{"error": "message", "code": "NOT_FOUND"}`
6. Use `context.Context` for request-scoped values, cancellation, and timeouts
7. Use `http.Server{ReadTimeout, WriteTimeout, IdleTimeout}` — never use defaults in production
8. Implement graceful shutdown: `signal.NotifyContext` → `server.Shutdown(ctx)`
9. Use `database/sql` with connection pool settings: `SetMaxOpenConns`, `SetMaxIdleConns`
10. Use `go:embed` for static assets — embed templates and files at compile time
11. Health check endpoint: `GET /healthz` returning 200 with DB connectivity check
12. Use `httptest.NewRecorder()` and `httptest.NewRequest()` for handler unit tests
