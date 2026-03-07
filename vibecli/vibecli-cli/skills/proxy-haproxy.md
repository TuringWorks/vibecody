---
triggers: ["HAProxy", "haproxy", "haproxy config", "haproxy backend", "haproxy frontend", "haproxy ACL", "haproxy ssl"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["haproxy"]
category: devops
---

# HAProxy Load Balancer

When working with HAProxy:

1. Structure configuration into global, defaults, frontend, and backend sections; keep defaults minimal and override per-frontend/backend only when necessary.
2. Use `mode http` for Layer 7 routing with path-based ACLs and `mode tcp` for Layer 4 passthrough; never mix modes within a single frontend/backend pair.
3. Define health checks with `option httpchk GET /health` and set `inter 3s fall 3 rise 2` to detect failures quickly without overwhelming backends.
4. Terminate SSL at the frontend using `bind *:443 ssl crt /etc/haproxy/certs/combined.pem` and forward plaintext to backends with `X-Forwarded-Proto` headers.
5. Use ACLs for routing: `acl is_api path_beg /api` followed by `use_backend api_servers if is_api` to split traffic across backend pools.
6. Enable stick tables for rate limiting: `stick-table type ip size 100k expire 30s store http_req_rate(10s)` and deny with `http-request deny if { sc_http_req_rate(0) gt 100 }`.
7. Configure connection limits with `maxconn` at global, frontend, and server levels; set backend `maxconn` lower than frontend to create backpressure.
8. Use `balance roundrobin` for uniform servers, `leastconn` for long-lived connections, and `source` or `cookie` for session persistence.
9. Enable the stats page on a dedicated port: `listen stats bind *:8404` with `stats enable`, `stats uri /`, and strong `stats auth` credentials.
10. Set timeouts explicitly: `timeout connect 5s`, `timeout client 30s`, `timeout server 30s`; add `timeout http-request 10s` to mitigate slowloris attacks.
11. Use `option forwardfor` to pass client IPs and `http-request set-header X-Real-IP %[src]` so backends can log and authorize by source address.
12. Reload without downtime using `haproxy -f /etc/haproxy/haproxy.cfg -p /run/haproxy.pid -sf $(cat /run/haproxy.pid)` or systemd `reload`; validate config first with `haproxy -c -f /etc/haproxy/haproxy.cfg`.
