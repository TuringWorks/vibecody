---
triggers: ["OAuth login", "OAuth credentials", "Claude Pro", "Max subscription", "GitHub Copilot auth", "Gemini CLI login", "ChatGPT Plus", "device code flow", "token refresh", "subscription auth", "OAuthManager", "OAuthProvider", "oauth_login"]
tools_allowed: ["read_file", "write_file", "bash"]
category: security
---

# OAuth Login for AI Providers

When implementing or extending subscription-based OAuth authentication for AI providers:

1. **Prefer OAuth over API keys for subscription tiers** — users with Claude Pro/Max, GitHub Copilot, Gemini CLI free tier, or ChatGPT Plus/Pro should authenticate via `OAuthManager::simulate_device_flow` (or a real device-flow HTTP implementation) rather than managing raw API keys. This reduces credential exposure and aligns with providers' intended usage patterns.

2. **Use the device-code flow wherever supported** — `OAuthProvider::uses_device_flow()` returns `true` for Anthropic, GitHub, and Google. For providers that return `None` from `device_code_url()` (e.g. `OpenAICodex`), fall back to a browser-redirect flow. Always display the `user_code` and `verification_url` to the user via `OAuthLoginCallbacks::on_device_code`.

3. **Implement proactive token refresh** — check `OAuthCredentials::needs_refresh()` before each API call. The 5-minute window gives enough time to complete the refresh without the caller ever hitting a 401. Chain refresh attempts before returning an error; only surface the error to the user if the refresh itself fails.

4. **Respect provider-specific cache retention** — use `CacheRetentionConfig::for_provider` to configure prompt-cache TTLs. Anthropic and GitHub use 1-hour retention (3 600 s); OpenAI Codex uses 24-hour retention (86 400 s). Setting the correct TTL avoids unnecessary cache misses and reduces per-token costs for long sessions.

5. **Store credentials in ProfileStore, never in plaintext** — `OAuthCredentials` must be serialised and encrypted via `ProfileStore::open_with` before persistence. Never write tokens to `*.toml`, `*.json`, or any file outside the encrypted store. The `redacted()` helper produces a log-safe string (`Bearer ****...{last4}`) — use it in all trace/debug output.

6. **Build auth headers through `OAuthManager::auth_header`** — this method applies a consistent priority: valid OAuth token beats API key fallback. This ensures callers do not need to know which authentication method is active and the transition from API-key to OAuth login is transparent.

7. **Use `NoopCallbacks` in unit tests and simulations** — never wire real UI callbacks into BDD or unit tests. Inject `&NoopCallbacks` for `simulate_device_flow` in test code so callbacks are silently discarded. For integration tests, provide a `RecordingCallbacks` struct that stores events in a `Vec` for assertion.

8. **Validate `OAuthProvider::from_str` slugs at CLI boundaries** — when parsing provider names from CLI arguments or config files, call `OAuthProvider::from_str` and emit a clear error if `None` is returned. Supported slugs: `claude`/`anthropic`/`anthropic_claude`, `copilot`/`github`/`github_copilot`, `gemini`/`gemini_cli`/`google_gemini_cli`, `codex`/`openai`/`openai_codex`/`chatgpt`. Avoid hard-coding strings elsewhere.

9. **Guard against token scope mismatch** — after a successful flow, verify that `OAuthCredentials::scopes` contains the scopes your feature requires before using the token. If required scopes are missing, trigger a re-authorization rather than silently degrading functionality.

10. **Handle `OAuthFlowResult::Cancelled` gracefully** — a user aborting the device-code flow is not an error. Surface a friendly message (e.g. "Login cancelled — you can retry with `vibecli login <provider>`") and return the caller to a usable state rather than propagating a hard error.
