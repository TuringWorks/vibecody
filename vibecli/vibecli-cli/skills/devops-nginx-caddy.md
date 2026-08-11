---
name: "Nginx & Caddy Reverse Proxy"
description: "Nginx & Caddy Reverse Proxy: Guidance for configuring reverse proxies. Use when the task involves nginx, Caddy, reverse proxy, load balancing, TLS termination."
category: devops
triggers: ["nginx", "Caddy", "reverse proxy", "load balancing", "TLS termination", "rate limiting proxy"]
tools_allowed: ["read_file", "write_file", "bash"]
---

# Nginx & Caddy Reverse Proxy

When configuring reverse proxies:

1. Use Caddy for automatic HTTPS (Let's Encrypt) — zero config TLS for most setups
2. Nginx: use `upstream` blocks for load balancing — `least_conn` or `ip_hash` strategies
3. TLS: redirect HTTP→HTTPS, use TLS 1.2+ minimum, enable HSTS with `max-age=31536000`
4. Set `proxy_pass` headers: `X-Real-IP`, `X-Forwarded-For`, `X-Forwarded-Proto`, `Host`
5. Rate limiting: Nginx `limit_req_zone` with burst; Caddy `rate_limit` directive
6. Gzip compression: enable for text/html, application/json, text/css, application/javascript
7. Connection timeouts: `proxy_connect_timeout`, `proxy_read_timeout` — match backend SLOs
8. Cache static assets: `location ~* \.(js|css|png|jpg)$ { expires 30d; add_header Cache-Control "public"; }`
9. WebSocket proxying: set `Upgrade` and `Connection` headers in `location /ws/`
10. Health checks: upstream health with `max_fails=3 fail_timeout=30s`
11. Security headers: `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, CSP
12. Use `access_log` with JSON format for structured log analysis
