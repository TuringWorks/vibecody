---
triggers: ["gRPC", "protobuf", "Protocol Buffers", "proto3", "streaming RPC", "tonic", "grpc-go"]
tools_allowed: ["read_file", "write_file", "bash"]
category: api
---

# gRPC & Protocol Buffers

When building gRPC services:

1. Define service contracts in `.proto` files — use proto3 syntax
2. Use `protoc` or `buf` for code generation — generate client and server stubs
3. Four RPC types: Unary, Server streaming, Client streaming, Bidirectional streaming
4. Use `google.protobuf.Timestamp` for dates, `google.protobuf.Duration` for intervals
5. Field numbering: never reuse or change numbers — mark removed fields as `reserved`
6. Use `oneof` for mutually exclusive fields; `repeated` for lists; `map<K,V>` for dictionaries
7. Error handling: use `google.rpc.Status` with standard codes (NOT_FOUND, INVALID_ARGUMENT, etc.)
8. Use deadlines/timeouts on every RPC call — prevent hanging connections
9. Implement health checking with `grpc.health.v1.Health` service
10. Use interceptors (middleware) for logging, auth, metrics, tracing
11. Backward compatibility: only add new fields — never rename, remove, or change types
12. Use `buf lint` for style enforcement and `buf breaking` for compatibility checking
