---
triggers: ["Flask blueprint", "flask factory", "flask-sqlalchemy", "flask-migrate", "flask celery", "flask async"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["python3"]
category: python
---

# Flask Advanced Patterns

When working with advanced Flask:

1. Use the application factory pattern: define `create_app(config_name)` that creates `Flask(__name__)`, loads config, initializes extensions, and registers blueprints, enabling multiple app instances for testing with different configurations.
2. Organize code into Blueprints with `Blueprint("api", __name__, url_prefix="/api")` and register sub-blueprints for nested route namespaces; keep each blueprint in its own package with models, schemas, and routes.
3. Configure Flask-SQLAlchemy with `SQLALCHEMY_ENGINE_OPTIONS={"pool_size": 10, "pool_recycle": 300}` and use `db.session` as a scoped session; always call `db.session.remove()` in `teardown_appcontext` to prevent connection leaks.
4. Use Flask-Migrate (Alembic) for schema changes: run `flask db migrate -m "description"` to auto-generate migrations, review the generated script for correctness, and test both upgrade and downgrade paths before merging.
5. Integrate Celery by creating the instance in a separate `celery_app.py`, calling `celery.conf.update(app.config)` inside the factory, and wrapping tasks with `app.app_context()` so they access Flask extensions.
6. Use `flask.g` for request-scoped resources (database connections, current user) and `app.config` for app-wide settings; never store mutable request state on the app or extension objects.
7. Implement async views with `async def` handlers (Flask 2.0+) for I/O-bound endpoints; install `asgiref` and deploy with an ASGI server via `WsgiToAsgi` adapter, or use `flask[async]` extras directly.
8. Write tests using `app.test_client()` from the factory with a test config that uses SQLite in-memory or a test PostgreSQL database; use `pytest` fixtures to create and teardown the app context per test.
9. Implement custom error handlers with `@app.errorhandler(HTTPException)` that return JSON for API blueprints and HTML for web blueprints, detected via `request.accept_mimetypes.best` or blueprint prefix.
10. Use Flask-Caching with Redis backend for expensive queries: decorate views with `@cache.cached(timeout=300, key_prefix=make_cache_key)` and invalidate explicitly on write operations.
11. Secure the application with Flask-Talisman for HTTPS/CSP headers, Flask-Limiter for rate limiting per endpoint, and Flask-CORS configured with explicit allowed origins rather than wildcards.
12. Deploy with Gunicorn using `gunicorn "app:create_app()" --workers 4 --worker-class gthread --threads 2` behind nginx; use `--preload` to share memory across workers and `--max-requests 1000` to prevent memory leaks.
