---
triggers: ["A2A protocol", "A2A v0.3", "gRPC agent", "security card", "agent-to-agent"]
tools_allowed: ["read_file", "write_file", "bash"]
category: protocols
---

# Agent-to-Agent (A2A) Protocol v0.3

When implementing or integrating with the A2A v0.3 specification:

1. **Transport Selection: gRPC vs HTTP/SSE** — Use gRPC when: both agents are in the same datacenter or VPC, latency is critical (<20ms), or task streams are long-lived and high-throughput. Use HTTP/SSE when: agents cross network boundaries, firewall rules block gRPC, or the consumer is a browser or lightweight client. Never mix transports within a single agent session; commit at session initiation.
2. **Security Card Signing for Agent Identity** — Every agent must present a signed security card at session establishment. The card contains: agent ID (UUID v4), capability list, public key (Ed25519), issuer DID, and expiry timestamp. Sign the card with the agent's private key and verify the issuer DID chain before accepting any task from an unverified agent. Reject expired cards immediately.
3. **Backward Compatibility with v0.2** — A2A v0.3 adds gRPC transport and security cards; the HTTP/SSE envelope and task schema are backward compatible with v0.2. When receiving a v0.2 peer, negotiate down to v0.2 capabilities automatically. Never send gRPC-only features (streaming acknowledgments, card assertions) to a v0.2 peer; detect version via the `a2a-version` header.
4. **Capability Negotiation Patterns** — During handshake, both agents exchange capability manifests. Capabilities are versioned strings (e.g., `code.review@1`, `deploy.k8s@2`). Accept only capabilities your agent can fulfill; never advertise capabilities you cannot satisfy. If a required capability is missing from the peer, fail fast with a `CapabilityMissing` error rather than attempting the task.
5. **Task Envelope Structure** — Wrap every task in the standard A2A envelope: `task_id`, `parent_task_id` (for nested delegation), `sender_card_id`, `created_at`, `ttl_seconds`, `payload` (opaque bytes), and `result_schema` (JSON Schema the sender expects). Validate the result schema before returning a response to ensure type safety across agent boundaries.
6. **Streaming Task Updates** — For long-running tasks, use A2A streaming updates rather than polling: emit progress events every 5 seconds minimum when work is ongoing. Each update must include: `task_id`, `progress` (0.0–1.0), `status` (running/paused/blocked), and an optional `partial_result`. Receiving agents must handle out-of-order update delivery gracefully.
7. **Error Taxonomy** — Use the A2A v0.3 standard error codes: `AUTH_FAILED` (card invalid or expired), `CAPABILITY_MISSING`, `TASK_TIMEOUT`, `RESOURCE_LIMIT`, `POLICY_DENIED`, `MALFORMED_PAYLOAD`, and `INTERNAL_ERROR`. Always include a human-readable `detail` field. Never swallow errors silently; propagate them upstream to the originating agent.
8. **Audit and Compliance Logging** — Log every A2A task lifecycle event (received, started, updated, completed, failed) with: task ID, sender card ID, capability invoked, input hash, output hash, and wall time. Store audit logs append-only and immutably. For regulated environments, sign audit log entries with the receiving agent's key to prevent tampering.
