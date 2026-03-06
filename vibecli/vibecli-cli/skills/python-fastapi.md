---
triggers: ["fastapi", "pydantic", "uvicorn", "dependency injection python", "async endpoint", "python API"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["python3"]
category: python
---

# Python FastAPI

When building FastAPI applications:

1. Use Pydantic models for request/response validation — define schemas as classes with type hints
2. Use `Depends()` for dependency injection — auth, DB sessions, rate limiting
3. Use `async def` for I/O-bound endpoints; regular `def` for CPU-bound (runs in thread pool)
4. Structure: `main.py` → `routers/` → `services/` → `models/` → `schemas/`
5. Use `HTTPException(status_code=404, detail="Not found")` for error responses
6. Add `response_model=Schema` to endpoints to auto-filter response fields
7. Use `BackgroundTasks` for fire-and-forget work (emails, logging, cleanup)
8. Use path parameters for resources (`/users/{id}`), query params for filters (`?status=active`)
9. Use `lifespan` context manager for startup/shutdown events (DB pool, cache warming)
10. Use `APIRouter` with `prefix` and `tags` to organize endpoints by domain
11. Always use `python-multipart` for form data and file uploads
12. Use `status_code=201` for creation, `204` for deletion — not everything is 200
