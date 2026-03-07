---
triggers: ["Envoy proxy", "envoy", "envoy filter", "envoy cluster", "envoy listener", "envoy sidecar", "xDS"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["envoy"]
category: devops
---

# Envoy Proxy

When working with Envoy proxy:

1. Structure config with listeners (inbound), clusters (outbound), and routes; use `static_resources` for simple setups and switch to xDS (dynamic resources) when managing more than a handful of services.
2. Define clusters with active health checking: set `health_checks` with `http_health_check` on a dedicated path, and configure `healthy_threshold`, `unhealthy_threshold`, and `interval` for responsive failover.
3. Use route matching with `prefix`, `path`, or `safe_regex` in `route_config`; prefer `prefix` for performance and reserve regex for complex patterns that cannot be expressed otherwise.
4. Configure circuit breakers per cluster: set `max_connections`, `max_pending_requests`, `max_requests`, and `max_retries` in `circuit_breakers.thresholds` to prevent cascade failures.
5. Enable automatic retries with `retry_policy` on routes: specify `retry_on: "5xx,connect-failure,reset"` with `num_retries: 3` and `per_try_timeout` to handle transient backend errors.
6. Use HTTP filters in the correct order: `envoy.filters.http.ratelimit` before `envoy.filters.http.router`; custom Lua or Wasm filters go between auth and routing filters.
7. Implement load balancing with `lb_policy`: use `ROUND_ROBIN` for uniform backends, `LEAST_REQUEST` for varied latencies, and `RING_HASH` or `MAGLEV` for session affinity.
8. Configure TLS with `transport_socket` using `DownstreamTlsContext` on listeners and `UpstreamTlsContext` on clusters; use SDS (Secret Discovery Service) for certificate rotation without restarts.
9. Leverage Envoy as a sidecar proxy by binding the listener to `127.0.0.1`, using iptables or CNI to redirect traffic, and configuring `original_dst` cluster for transparent proxying.
10. Use access logging with `envoy.access_loggers.file` or `envoy.access_loggers.open_telemetry`; include `%RESPONSE_FLAGS%` and `%UPSTREAM_HOST%` in the format for debugging connection issues.
11. Deploy Wasm filters for custom logic: compile to `.wasm`, reference via `envoy.filters.http.wasm` with a `vm_config`, and use shared data for cross-filter state without modifying Envoy source.
12. Use the admin interface (`:9901`) for debugging: `/clusters` shows backend health, `/config_dump` shows effective config, `/stats` exposes counters; always restrict admin to localhost or internal networks in production.
