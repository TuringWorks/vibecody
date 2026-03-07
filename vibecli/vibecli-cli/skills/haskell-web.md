---
triggers: ["Haskell", "servant", "yesod", "warp haskell", "ihp", "haskell web", "cabal", "stack haskell"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["ghc"]
category: haskell
---

# Haskell Web Development

When working with Haskell web frameworks:

1. Use Servant to define APIs as types (`type API = "users" :> Get '[JSON] [User]`) — the compiler ensures handlers match the type-level specification exactly.
2. Derive client functions, documentation, and mock servers from the same Servant API type using `client`, `servant-docs`, and `servant-mock` to keep them in sync automatically.
3. In Yesod, define routes in `config/routes` and use type-safe URL rendering (`@{HomeR}`) in templates — never hardcode URL strings in Hamlet or Julius templates.
4. Use `ReaderT AppEnv IO` (or `RIO`) as your application monad to thread configuration, database pools, and loggers through handlers without explicit parameter passing.
5. Manage resources with `bracket` or `ResourceT` — acquire database connections, file handles, and HTTP managers in a resource-safe way that guarantees cleanup on exceptions.
6. Use `persistent` with `esqueleto` for type-safe database queries — define models in a `persistLowerCase` quasi-quoter and generate migrations with `runMigration migrateAll`.
7. Prefer `Text` and `ByteString` over `String` everywhere — use `OverloadedStrings` extension and `text`/`bytestring` libraries for all request/response handling.
8. Apply `aeson` with explicit `FromJSON`/`ToJSON` instances using `genericParseJSON defaultOptions { fieldLabelModifier = ... }` instead of default generic instances to control JSON field names.
9. Use `warp` as the underlying HTTP server for Servant and standalone apps — configure `setPort`, `setTimeout`, and `setOnException` in `Settings` for production tuning.
10. Test Servant APIs with `hspec-wai` — use `with (pure app)` to create a WAI application and `get "/users" \`shouldRespondWith\` 200` for concise integration tests.
11. Enable strict data with `StrictData` or `BangPatterns` in performance-critical modules to avoid space leaks from accumulated thunks in request-processing pipelines.
12. Use `katip` or `co-log` for structured logging — thread a `LogEnv` through your monad stack and log with severity levels and JSON payloads rather than `putStrLn` debugging.
