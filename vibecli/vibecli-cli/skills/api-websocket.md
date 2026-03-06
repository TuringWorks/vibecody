---
triggers: ["WebSocket", "ws://", "real-time", "heartbeat", "reconnection", "socket.io", "ws protocol"]
tools_allowed: ["read_file", "write_file", "bash"]
category: api
---

# WebSocket API Design

When implementing WebSocket APIs:

1. Use WebSocket for bidirectional real-time communication — not for request-response patterns
2. Implement heartbeat/ping-pong: server sends ping every 30s, client responds with pong
3. Use JSON messages with a `type` field for routing: `{ "type": "chat.message", "payload": {...} }`
4. Implement reconnection with exponential backoff: 1s, 2s, 4s, 8s... cap at 30s
5. Buffer messages during disconnection — replay on reconnect with sequence numbers
6. Authentication: validate token during WebSocket upgrade (handshake), not in messages
7. Use rooms/channels for scoping broadcasts — don't send everything to everyone
8. Implement rate limiting per connection — prevent message flooding
9. Close connections gracefully: send close frame with code (1000=normal, 1001=going away)
10. Use binary frames for large payloads (files, images); text frames for JSON messages
11. Consider SSE (Server-Sent Events) for server-to-client only — simpler than WebSocket
12. Load testing: WebSocket connections are long-lived — account for connection limits per server
