---
triggers: ["cross-environment agents", "remote SSH agent", "cloud VM dispatch", "parallel environments", "env dispatch"]
tools_allowed: ["read_file", "write_file", "bash"]
category: agent
---

# Environment Dispatch for Agents

When dispatching agents across local, SSH, and cloud environments:

1. **Environment Selection Criteria** — Select the target environment based on: data locality (run near the data), required hardware (GPU workloads → cloud), security boundary (sensitive code → local or on-prem SSH), and task duration (ephemeral tasks → serverless, long-running → VM). Never dispatch to a remote environment when local execution is sufficient.
2. **Cost vs Latency Tradeoffs** — Local execution has zero egress cost and lowest latency but limited resources. SSH remotes have low cost and moderate latency; prefer them for CPU-bound tasks. Cloud VMs have high cost and higher latency; reserve them for GPU or memory-intensive workloads. Always estimate cost before dispatching to cloud.
3. **Health Monitoring Before Dispatch** — Before dispatching, verify the target environment is reachable: ping SSH targets with a lightweight probe command, check cloud VM health endpoints, and verify local resource availability (CPU %, free memory). Refuse to dispatch if health check fails; surface the failure to the user instead of silently queuing.
4. **Fallback Strategies** — Define a fallback chain per task type: primary (cloud GPU) → fallback1 (SSH remote CPU) → fallback2 (local CPU with reduced quality). Implement automatic failover with configurable retry delay. Log every fallback event with the reason and the fallback target chosen.
5. **Resource Capping Per Environment** — Enforce hard caps per environment: max concurrent agent tasks, max memory per task, max execution wall time, and max egress bytes. Caps should be stored in environment config and enforced at the dispatcher layer before any work begins. Tasks exceeding caps must be rejected with a descriptive error, not silently throttled.
6. **Credential and Secret Isolation** — Never embed secrets in dispatched task payloads. Use environment-scoped credential references (e.g., SSH agent forwarding, cloud IAM roles, vault-injected env vars). The dispatcher resolves credentials at launch time and injects them via secure channels only, never via task arguments or logs.
7. **Parallel Environment Coordination** — When dispatching the same task to multiple environments in parallel (e.g., for benchmarking or redundancy), use a result-arbitration policy: first-to-succeed wins, majority-vote for deterministic tasks, or explicit primary/shadow for shadow testing. Cancel in-flight tasks on all other environments once the winning result is accepted.
8. **State Synchronization** — For stateful agents spanning multiple environments, synchronize checkpoints via a shared persistent store (e.g., S3, NFS, or workspace DB), not via direct agent-to-agent transfer. Each environment reads from and writes to the checkpoint store; the dispatcher coordinates sequencing to prevent write conflicts.
