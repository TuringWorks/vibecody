---
triggers: ["Drogon", "oatpp", "userver", "cpp web framework", "C++ REST", "C++ http server", "crow", "cinatra"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["cmake"]
category: cpp
---

# C++ Web Frameworks

When working with C++ web frameworks (Drogon, Oat++, userver, Crow):

1. Use Drogon for high-performance async APIs: define controllers with `DROGON_DECLARE_HANDLER(getUser)` macros or `HttpSimpleController`; register routes via `app().registerHandler("/api/users/{id}", &getUser)` or JSON config.
2. Leverage Drogon's ORM (drogon_ctl) to generate model classes from database schemas: run `drogon_ctl create model models` and use `Mapper<User>` with `findByPrimaryKey` and `findBy` for type-safe queries.
3. In Oat++ (oatpp), define DTOs with `DTO_INIT` and `DTO_FIELD` macros for automatic serialization: use `ENDPOINT("GET", "/api/users", getUsers)` in ApiController subclasses for declarative routing.
4. Use Oat++'s built-in Swagger integration: add `oatpp-swagger` and annotate endpoints with `ENDPOINT_INFO(getUser) { info->summary = "Get user"; }` for auto-generated OpenAPI docs.
5. For userver, define handlers as `HttpHandlerBase` subclasses: return `std::string` from `HandleRequestThrow` and register in the component list; userver manages coroutine scheduling automatically.
6. Manage dependencies with CMake `FetchContent` or Conan/vcpkg: `FetchContent_Declare(drogon GIT_REPOSITORY ...)` keeps builds reproducible and avoids system-wide installation issues.
7. Use connection pooling for databases: Drogon provides `DbClientPtr` with built-in pooling; in other frameworks, use `libpqxx` or `mysql-connector-cpp` with a pool wrapper to avoid connection churn.
8. Handle JSON with `nlohmann/json` or framework-native JSON: Drogon uses `Json::Value`, Oat++ uses DTO mapping; parse with `req->getJsonObject()` and validate before accessing fields.
9. Implement middleware/filters for auth and logging: Drogon filters use `doFilter(req, callback, chain)` pattern; Crow uses `before_handle` and `after_handle` in middleware structs.
10. Compile with `-O2 -DNDEBUG` for release builds; enable link-time optimization (`-flto`) and strip symbols for production binaries; benchmark with `wrk` or `h2load` to verify throughput gains.
11. Use Crow for simpler REST services: `CROW_ROUTE(app, "/api/users/<int>")([](int id) { return crow::response(200, user_json(id)); })` provides Express-like simplicity with C++ performance.
12. Containerize with multi-stage Docker builds: compile in a builder stage with all dev dependencies, copy only the static binary to a minimal runtime image (Alpine/distroless) to keep images under 20MB.
