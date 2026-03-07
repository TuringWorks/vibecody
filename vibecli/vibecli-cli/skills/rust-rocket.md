---
triggers: ["Rocket", "rocket.rs", "rocket framework", "rocket fairings"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["cargo"]
category: rust
---

# Rocket Framework

When working with Rocket:

1. Use `#[get]`, `#[post]`, `#[put]`, `#[delete]` attribute macros with typed path parameters like `fn get_user(id: i64)` — Rocket validates and parses parameters automatically.
2. Manage shared state with `rocket.manage(MyState::new())` and access it in handlers via `&State<MyState>` — state must be `Send + Sync`.
3. Implement the `Responder` trait on custom types to control status codes, headers, and body formatting for consistent API responses.
4. Use `FromRequest` guard trait for authentication and authorization — return `Outcome::Success(user)` or `Outcome::Error((Status, err))` to gate handler access.
5. Attach fairings with `.attach(MyFairing)` for cross-cutting concerns like CORS, request logging, or database connection setup — fairings are Rocket's middleware equivalent.
6. Prefer `rocket::serde::json::Json<T>` for request and response bodies; derive `Serialize` and `Deserialize` on your DTOs.
7. Configure the application via `Rocket.toml` with environment profiles (`[default]`, `[debug]`, `[release]`) for port, log level, and custom values.
8. Use `#[catch(404)]` and `#[catch(500)]` with `register(catchers![...])` to provide structured JSON error responses instead of default HTML.
9. Use `rocket_db_pools` with `#[derive(Database)]` for async connection pooling — configure pool size in `Rocket.toml` under `[default.databases.mydb]`.
10. Test endpoints using `Client::tracked(rocket).await` from `rocket::local::asynchronous` — chain `.get("/path").dispatch().await` and assert on status and body.
11. Use `FromForm` derive for query parameters and form data; combine with `FromFormField` for custom field validation logic.
12. Mount route groups with `rocket.mount("/api/v1", routes![...])` to version your API and keep route definitions organized by domain.
