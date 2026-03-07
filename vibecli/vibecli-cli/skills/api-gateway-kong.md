---
triggers: ["Kong", "kong gateway", "kong plugin", "kong deck", "kong route", "kong service", "kong ingress controller"]
tools_allowed: ["read_file", "write_file", "bash"]
category: devops
---

# Kong API Gateway

When working with Kong API Gateway:

1. Define services and routes declaratively with Kong decK — maintain a `kong.yaml` file in version control and sync with `deck sync`: `services: [{name: user-api, url: http://user-svc:8080, routes: [{name: users-route, paths: [/api/users], strip_path: true}]}]`.

2. Apply rate limiting with the `rate-limiting` plugin at the service, route, or consumer level: `plugins: [{name: rate-limiting, config: {minute: 100, hour: 1000, policy: redis, redis_host: redis}}]`. Use Redis policy for multi-node consistency.

3. Implement authentication by stacking plugins — use `key-auth` or `jwt` for API keys/tokens, `oauth2` for full OAuth flows, and `openid-connect` (Enterprise) for OIDC providers. Apply to routes or globally.

4. Use the `request-transformer` plugin to modify upstream requests — add headers, rename query parameters, or inject values: `config: {add: {headers: ["X-Request-ID:$(request_id)"]}, rename: {querystring: ["api_key:key"]}}`.

5. Configure load balancing with upstream targets and health checks: `upstreams: [{name: user-upstream, healthchecks: {active: {http_path: /health, healthy: {interval: 5}, unhealthy: {interval: 5, http_failures: 3}}}, targets: [{target: "10.0.0.1:8080", weight: 100}]}]`.

6. Deploy Kong Ingress Controller (KIC) in Kubernetes — annotate Ingress resources to apply Kong plugins: `metadata: {annotations: {konghq.com/plugins: rate-limit-users}}` and define plugins as `KongPlugin` custom resources.

7. Write custom plugins in Lua for specialized logic — place plugin files in `/usr/local/share/lua/5.1/kong/plugins/my-plugin/` with `handler.lua` and `schema.lua`. Use the PDK (Plugin Development Kit): `kong.service.request.set_header("X-Custom", value)`.

8. Implement circuit breaking with the upstream `circuit_breaker` settings — Kong marks targets unhealthy after configured failure thresholds and stops routing to them until health checks pass again.

9. Use the `correlation-id` plugin to propagate trace IDs across services: `config: {header_name: X-Correlation-ID, generator: uuid}`. Combine with `tcp-log`, `http-log`, or `datadog` plugins for centralized logging.

10. Manage consumers and credentials for multi-tenant APIs: `consumers: [{username: partner-a, keyauth_credentials: [{key: "abc123"}], plugins: [{name: rate-limiting, config: {minute: 50}}]}]` — per-consumer rate limits and access control.

11. Enable response caching with the `proxy-cache` plugin to reduce backend load: `config: {response_code: [200], request_method: ["GET"], content_type: ["application/json"], cache_ttl: 300, strategy: memory}`. Use Redis strategy for distributed caching.

12. Use `deck diff` in CI pipelines to preview configuration changes before applying, and `deck dump` to export the current state. Tag configurations with `_format_version: "3.0"` and validate with `deck validate` before deployment.
