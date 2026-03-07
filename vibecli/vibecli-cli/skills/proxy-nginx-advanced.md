---
triggers: ["nginx upstream", "nginx lua", "nginx rate limit", "nginx load balancer", "nginx caching", "nginx stream", "nginx map", "nginx rewrite"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["nginx"]
category: devops
---

# Advanced Nginx

When working with advanced Nginx configurations:

1. Define upstream blocks with health checks: use `upstream backend { server 10.0.0.1:8080 max_fails=3 fail_timeout=30s; server 10.0.0.2:8080 backup; }` to handle failover gracefully.
2. Use `map` directives for conditional logic instead of nested if blocks: `map $uri $backend { ~^/api api_upstream; default static_upstream; }` is more efficient and avoids the "if is evil" pitfalls.
3. Configure proxy caching with `proxy_cache_path /var/cache/nginx levels=1:2 keys_zone=app:10m max_size=1g inactive=60m` and activate per-location with `proxy_cache app` and `proxy_cache_valid 200 10m`.
4. Implement rate limiting with `limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s` in http context and `limit_req zone=api burst=20 nodelay` in the location block.
5. Use the `stream` module for TCP/UDP load balancing: `stream { upstream db { server db1:5432; server db2:5432; } server { listen 5432; proxy_pass db; } }` for non-HTTP protocols.
6. Leverage OpenResty/Lua for complex logic: `access_by_lua_block { if not validate_token(ngx.var.http_authorization) then ngx.exit(403) end }` for custom auth without external modules.
7. Set `proxy_buffering on` with `proxy_buffer_size 4k` and `proxy_buffers 8 16k` to decouple slow clients from backend connections; disable only for SSE/streaming endpoints.
8. Use `rewrite` sparingly; prefer `return 301 https://$host$request_uri` for redirects and `try_files $uri $uri/ /index.html` for SPA routing over chained rewrites.
9. Configure connection keepalives to upstreams: `upstream backend { keepalive 32; }` with `proxy_http_version 1.1` and `proxy_set_header Connection ""` to reuse backend connections.
10. Enable microcaching for dynamic content: `proxy_cache_valid 200 1s` with `proxy_cache_use_stale updating` dramatically reduces backend load while serving near-real-time content.
11. Use `split_clients` for A/B testing: `split_clients $request_id $variant { 50% backend_v1; * backend_v2; }` to route traffic deterministically without application changes.
12. Monitor with the stub_status module on a restricted endpoint and export metrics; use `access_log` with custom `log_format` including `$upstream_response_time` and `$request_time` for latency tracking.
