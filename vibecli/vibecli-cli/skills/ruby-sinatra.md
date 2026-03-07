---
triggers: ["Sinatra", "sinatra ruby", "rack", "sinatra-activerecord", "roda", "hanami"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["ruby"]
category: ruby
---

# Sinatra and Lightweight Ruby

When working with Sinatra:

1. Use the modular style with `class App < Sinatra::Base` instead of the classic top-level DSL for any application beyond a prototype; this prevents global namespace pollution and enables mounting multiple apps via Rack.
2. Organize larger Sinatra apps by splitting route groups into separate classes and combining them in `config.ru` with `map("/api") { run ApiApp }` and `map("/admin") { run AdminApp }` for clean separation.
3. Use `sinatra-activerecord` gem with Rake tasks for database management: `rake db:create`, `rake db:migrate`, `rake db:seed`; store migrations in `db/migrate/` and configure via `DATABASE_URL` environment variable.
4. Implement middleware via `use Rack::Protection`, `use Rack::Deflater`, and `use Rack::Session::Cookie, secret: ENV['SESSION_SECRET']` in `config.ru` or inside `configure` blocks for security and compression.
5. Use `before` and `after` filters for cross-cutting concerns: `before { content_type :json }` for API apps, `before("/admin/*") { authenticate! }` for scoped auth; keep filters focused and avoid complex logic.
6. Return JSON responses with `json(data)` from `sinatra/json` helper and set `content_type :json` globally in API apps; use `halt 404, json(error: "Not found")` for error responses with correct status codes.
7. Write tests with `rack-test` gem: `include Rack::Test::Methods`, define `def app; App; end`, then use `get "/path"`, `post "/path", params.to_json`; assert on `last_response.status` and parse `JSON.parse(last_response.body)`.
8. Use Roda for routing-tree style apps when you need faster performance than Sinatra: `route do |r| r.on "api" do r.on "users" do ... end end end` with the plugin system for composable functionality.
9. For Hanami projects, use the slice architecture with `slices/api/` for bounded contexts; define repositories with `Hanami::DB::Repo`, use Actions for HTTP handling, and keep business logic in standalone operation objects.
10. Configure Puma as the application server with `puma.rb`: set `workers ENV.fetch("WEB_CONCURRENCY", 2)`, `threads_count = ENV.fetch("RAILS_MAX_THREADS", 5)`, and `preload_app!` for copy-on-write memory savings.
11. Implement background jobs with Sidekiq or GoodJob; keep job classes in `app/jobs/`, pass only primitive arguments (IDs, not objects), and set appropriate retry and dead-letter configurations per job type.
12. Deploy with Docker using a multi-stage Dockerfile: `bundle install --without development test` in the build stage, copy only necessary files to the runtime stage, run with `bundle exec puma -C config/puma.rb`, and use health check endpoints for orchestrator readiness probes.
