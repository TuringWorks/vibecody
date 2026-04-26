# 02 — Egress Broker

**Scope:** the out-of-sandbox network proxy that every Tier-0..3 sandbox uses to reach the world.
**Parent:** [`README.md`](./README.md)
**Status:** Draft · 2026-04-26

---

## Why a broker, not a firewall rule

Once a process inside the sandbox can `connect()` to anywhere, the FS jail is partly defeated — it can exfiltrate the contents of the bound folder, hit internal services on RFC1918, or pull `169.254.169.254` and steal cloud credentials. So the only design that holds up is:

- **The sandbox has zero direct network. Period.** Net namespace unshared on Linux; `(deny network*)` in the .sb on macOS; AppContainer with no `internetClient*` capability on Windows.
- **A broker process outside the sandbox** is the only path out. It enforces per-tool policy, injects credentials, blocks SSRF targets, MITMs HTTPS with a sandbox-local CA, and writes a structured audit log of every request.

This is the same shape you already have in `docker_runtime.rs:99–136` (iptables OUTPUT allow-list with DNS-resolution-driven rules). The work here is generalizing it from "Linux + Docker" to "all four tiers + all three platforms" and adding credential injection.

## Goals

1. **All four tiers route network through the broker by default.** Same policy DSL, same audit shape.
2. **The sandbox never sees secret values.** The broker reads `ProfileStore` / `WorkspaceStore` and substitutes credentials on egress for matching routes.
3. **HTTPS interception with a per-broker root CA**, trusted only inside the sandbox via env-var injection. The host never trusts it.
4. **Cloud SDKs (AWS / GCP / Azure) work transparently** via per-cloud credential injection and IMDS faking — the agent code does `boto3.client("s3")` and the broker handles SigV4 with a role token scoped to the sandbox session.
5. **Audit log is mandatory.** Every request emits a structured event consumable by the recap subsystem (`docs/design/recap-resume/02-job.md`).

## Non-goals

- A general-purpose forward proxy product. The broker is daemon-internal; we don't expose it to other apps.
- DPI / content scanning. The broker enforces *policy* (host, method, body cap) — it doesn't try to detect malicious content in responses.
- Replacing `agent_executor.rs::SSRF guard`. That logic moves *into* the broker so it covers every tier, not just `fetch_url`.

## Architecture

```
                        ┌────────── vibe-broker (daemon-internal Rust process) ────────────-─┐
                        │                                                                    │
sandbox process ──IPC─▶ │  HTTP(S) accept   ┐                                                │
                        │     │             │                                                │
                        │     ▼             │ matched route                                  │
                        │  parse policy ────┘                                                │
                        │     │                                                              │
                        │     ▼                                                              │
                        │  SSRF guard (bin: cloud-meta IPs, RFC1918, loopback)               │
                        │     │                                                              │
                        │     ▼                                                              │
                        │  hickory-dns resolve (or static cache)                             │
                        │     │                                                              │
                        │     ▼                                                              │
                        │  cred injection (SigV4 / Bearer / OAuth / mTLS client cert)        │
                        │     │                                                              │
                        │     ▼                                                              │
                        │  hyper+rustls outbound ─────────────────────────▶ internet         │
                        │     │                                                              │
                        │     ▼                                                              │
                        │  body-size / time caps applied on response                         │
                        │     │                                                              │
                        │     ▼                                                              │
                        │  audit emit (structured JSON line) ──▶ recap stream                │
                        │                                                                    │
                        └────────────────────────────────────────────────────────────────────┘
```

Every tier connects via a different IPC primitive but speaks plain HTTP/1.1 + CONNECT (the standard "I am a forward proxy" protocol):

| Tier | Transport | Detail |
|---|---|---|
| Tier-0 Linux | Unix domain socket bind-mounted into namespace at `/run/vibe-broker.sock` | Sandbox processes set `HTTPS_PROXY=unix:/run/vibe-broker.sock` |
| Tier-0 macOS | UDS allow-listed in `.sb`: `(allow network-outbound (literal "/private/var/run/vibe-broker.sock"))` | Same env var |
| Tier-0 Windows | Named pipe `\\.\pipe\vibe-broker-{ulid}` granted to the AppContainer SID | `HTTPS_PROXY=http://localhost:0` + `WINHTTP_PROXY=pipe:vibe-broker-{ulid}` (small shim translates) |
| Tier-1 WASI | Host-implemented `wasi:http/outgoing-handler` — broker *is* the implementation | Free; no env var |
| Tier-2 Hyperlight | Host function `vibe_egress_request` registered on the partition | Embedder routes to broker |
| Tier-3 Firecracker | virtio-vsock (CID 2 = host) on a known port | Standard CONNECT proxy in the guest's `/etc/environment` |

## Open-source ingredients

| Component | Crate / project | License | Why |
|---|---|---|---|
| HTTP server + client | `hyper` (1.x) | MIT | Industry-standard async HTTP in Rust |
| TLS (both sides) | `rustls` + `rustls-pemfile` | MPL-2 / Apache-2 | No OpenSSL dependency; constant-time |
| Per-broker root CA | `rcgen` | MIT/Apache | Mints a CA cert + ephemeral leaf certs per origin |
| DNS | `hickory-dns` (formerly trust-dns) | MIT/Apache | Resolves in the broker; sandbox has none |
| Routing / middleware | `tower` + `tower-http` | MIT | Composable request middlewares for policy + audit |
| AWS SigV4 | `aws-sigv4` | Apache-2 | Sign AWS requests in the broker; sandbox sees only role-scoped tokens |
| Google IAM signing | `google-cloud-auth` | Apache-2 | Sign GCP requests; same model |
| Azure MSI | `azure_identity` | MIT | Same |
| Audit emit | use existing `tracing-subscriber` JSON layer | MIT | Already in the workspace |
| Reference for HTTPS interception ergonomics | `mitmproxy` (Python) | MIT | We don't link it; we copy the patterns (per-origin leaf certs, `MITM_HEADERS` markers) |
| Reference for cloud-cred faking | `aws-vault`'s metadata-server mode | MIT | Pattern for IMDS faking |
| Reference for capability-based permission DSL | Deno's `runtime/permissions` | MIT | Inspiration only — Rust has no need to fork |

The broker itself is small — call it ~1500 lines of Rust glue across `accept`, `policy`, `cred`, `audit`, `dns`, and `imds`.

## Policy DSL

Every skill / agent definition gets an `egress.toml`. Resolved at sandbox spawn into a compiled `Policy` the broker holds in memory.

```toml
# vibecli/skills/aws-cost-check/egress.toml
default = "deny"

[[rule]]
match.host = "*.amazonaws.com"
match.methods = ["GET", "POST"]
inject = { type = "aws-sigv4", profile = "@workspace.aws_default" }
limits = { max_request_body = "1MB", max_response_body = "10MB", timeout = "30s" }

[[rule]]
match.host = "api.anthropic.com"
match.methods = ["POST"]
inject = { type = "bearer", key = "@profile.anthropic_api_key" }

[[rule]]
match.host = "api.github.com"
match.methods = ["GET", "POST", "PATCH"]
match.path_prefix = "/repos/me/myrepo/"     # scoped to one repo
inject = { type = "bearer", key = "@workspace.github_token" }

[[rule]]
match.host = "*.tailscale.com"
inject = { type = "none" }                  # internal — pass through

# Default: deny + log
```

### Reference resolution

- `@profile.<key>` → `ProfileStore::get(<key>)` — machine-bound encryption (`profile_store.rs`)
- `@workspace.<key>` → `WorkspaceStore::secret_get(<key>)` — workspace-bound encryption (`workspace_store.rs`)
- `@env.<key>` → daemon process env (use sparingly)
- `@oauth.<provider>` → resolve via `OAuthTokenStore` (separate ticket if not present)

### Match shape

```rust
pub struct RuleMatch {
    pub host: GlobPattern,                 // "*.amazonaws.com"
    pub methods: Vec<HttpMethod>,
    pub path_prefix: Option<String>,
    pub path_pattern: Option<GlobPattern>,
    pub require_tls: bool,                 // default true
    pub require_user_consent: bool,        // first call to a host triggers an approval prompt
}

pub enum Inject {
    None,
    Bearer { key: SecretRef },
    Basic { user: SecretRef, pass: SecretRef },
    AwsSigV4 { profile: SecretRef },
    GcpIam { service_account: SecretRef },
    AzureMsi { client_id: SecretRef },
    MtlsClient { cert: SecretRef, key: SecretRef },
    HeaderTemplate { name: String, value_template: String },
}
```

`require_user_consent` borrows from the existing `ApprovalPolicy` system. First call to a new host pauses the agent and shows an approval prompt in vibeui (or REPL). Subsequent calls within the session are auto-approved unless policy says otherwise.

## Credential injection — how it actually works

```
Sandbox sends:
    POST /api/v3/repos/me/myrepo/issues HTTP/1.1
    Host: api.github.com
    User-Agent: vibe-skill/foo
    Content-Type: application/json
    {...}

Broker:
    1. Match rule: api.github.com + POST + /repos/me/myrepo/* → match
    2. Inject: bearer with @workspace.github_token
       → resolve token from WorkspaceStore (decrypts on the fly)
       → add: Authorization: Bearer ghp_...
       → token never written to disk; lives in RAM for the duration of the request
    3. Forward (with original body)
    4. Receive response, apply max_response_body cap
    5. Strip Authorization from any redirect Location response (defense in depth)
    6. Audit: { event: "egress.request", host: "api.github.com", method: "POST", status: 201, bytes_in: 412, bytes_out: 1283, policy_id: "skill:foo", inject: "bearer" }
    7. Return response to sandbox

The sandbox never sees ghp_..., never sees the WorkspaceStore key, can't request the same token if blocked elsewhere.
```

For cloud SDKs that auto-sign requests from the SDK side (boto3 with explicit creds in env), the model is slightly different — the broker injects *via the IMDS faker* instead of post-signing. See "Cloud IMDS faking" below.

## SSRF + cloud-metadata defense

The broker's first stage (after policy match) is an SSRF guard that rejects:

- `127.0.0.0/8`, `::1/128`
- RFC1918 (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`)
- Link-local: `169.254.0.0/16`, `fe80::/10` — except the IMDS faker, see below
- IPv6 ULA: `fc00::/7`
- Cloud metadata DNS names: `metadata.google.internal`, `metadata.azure.com`, etc.
- DNS rebinding: re-check IP after resolution; reject if `target_host` resolved to a banned IP

This is the existing `agent_executor.rs:21-56` SSRF guard, **moved into the broker so it covers every tier and every tool, not just `fetch_url`.**

Override: a rule can opt-in `match.allow_ssrf_target = "169.254.169.254"` for the IMDS faker only.

## Cloud IMDS faking

Cloud SDKs auto-resolve credentials from a chain that ends at IMDS (`169.254.169.254` for AWS, `metadata.google.internal` for GCP, `169.254.169.254` for Azure). If the sandbox has no network, those calls fail and SDK initialization breaks.

The broker exposes a tiny IMDS faker on `169.254.169.254` (NAT'd via the broker IPC):

```
GET /latest/meta-data/iam/security-credentials/
  → returns: vibe-sandbox-role
GET /latest/meta-data/iam/security-credentials/vibe-sandbox-role
  → returns: { AccessKeyId: "ASIA...", SecretAccessKey: "...", Token: "...", Expiration: "..." }
```

The token is freshly minted by the broker via STS `AssumeRole` against a role the user has authorized for *this sandbox session*, with:
- 15-minute TTL (rotated automatically)
- Session policy that scopes to *just* the resources this skill's `egress.toml` allow-lists
- `sts:SourceIdentity` set to `vibe-sandbox-{session_id}` so CloudTrail logs are auditable

If the user hasn't authorized any cloud role, the IMDS faker returns 404 and SDKs fall back to the next chain link (which also fails, predictably).

GCP and Azure faking work the same shape with their respective metadata APIs.

Reference implementation: [aws-vault metadata-server mode](https://github.com/99designs/aws-vault) does this in Go; we'll do the same in Rust using `aws-sdk-sts`.

## HTTPS interception

The broker mints a per-broker root CA on first run, stored under `~/.vibecli/sandbox-ca/` with mode `0600`:

- `ca.pem` — root cert (public)
- `ca.key.enc` — root private key, encrypted with the same key derivation as `ProfileStore` (machine-bound, ChaCha20-Poly1305)

For each new origin the sandbox connects to, the broker mints an ephemeral leaf cert signed by the root, valid for ~1 hour, with the appropriate SAN. Cached in memory for the broker's lifetime; not written to disk.

The sandbox is told to trust the root via env-var injection — and *only* the sandbox:

```
SSL_CERT_FILE=/etc/ssl/vibe-sandbox-ca.pem        # OpenSSL, curl
NODE_EXTRA_CA_CERTS=/etc/ssl/vibe-sandbox-ca.pem  # Node.js
REQUESTS_CA_BUNDLE=/etc/ssl/vibe-sandbox-ca.pem   # Python requests
CURL_CA_BUNDLE=/etc/ssl/vibe-sandbox-ca.pem       # curl
GIT_SSL_CAINFO=/etc/ssl/vibe-sandbox-ca.pem       # git
```

The host system's CA store is unchanged. The sandbox's CA bundle has only the per-broker root.

### Caveat: pinned TLS roots

Some clients pin TLS roots and refuse the broker's CA — Go's `crypto/tls` with a hard-coded `RootCAs` pool, mobile SDKs, OAuth library variants. For these, two paths:

1. **Bypass mode** for that specific host: the broker's rule sets `mitm = false` and acts as a transparent CONNECT-passthrough — no interception, no body inspection, but policy match still applies (host allow-list, body-size caps via byte-counter, audit log records that the body content was opaque).
2. **Document the limitation** in `docs/security.md`: pinned-cert clients work via passthrough but with reduced inspection.

Pick (2) for v1; (1) is configurable per rule for users who explicitly want MITM bypass.

## Audit event shape

Every request emits one event:

```jsonc
{
  "event": "egress.request",
  "ts": "2026-04-26T17:32:01.123Z",
  "session_id": "01HK...",
  "policy_id": "skill:aws-cost-check",
  "tier": "native",
  "host": "lambda.us-east-1.amazonaws.com",
  "method": "POST",
  "path": "/2015-03-31/functions/cost-check/invocations",
  "status": 200,
  "bytes_request": 412,
  "bytes_response": 1283,
  "duration_ms": 78,
  "inject": "aws-sigv4",
  "matched_rule_index": 0,
  "user_consented": true,
  "outcome": "ok"          // or: "policy_denied", "ssrf_blocked", "body_oversized", "tls_error"
}
```

Events stream through the existing `tracing` infrastructure to:

- `~/.vibecli/audit/egress.jsonl` (rotating, 90-day retention by default — configurable)
- The recap subsystem's audit ingestor (`docs/design/recap-resume/02-job.md`) — a job recap can show "made 14 calls to api.openai.com, 2 to github.com, 0 denied"

## Performance

- Hot path latency: < 1 ms broker overhead per request when the policy is cached. The bottleneck is the upstream host, not the broker.
- Throughput: hyper's tokio executor handles thousands of concurrent connections. Per-sandbox we expect single-digit RPS, so capacity is not a concern.
- Memory: ~10 MB resident for the broker + ~50 KB per active connection.
- Cold-start: broker is launched at daemon startup, not per-sandbox; sandboxes connect to an already-running broker. Broker boot is < 100 ms.

## Failure modes

| Failure | Behavior |
|---|---|
| Broker process dies | Daemon supervisor restarts; sandboxes get connection-refused until broker is back; recap notes the gap |
| Policy file malformed | Daemon refuses to start the sandbox; surfaces a clear error to the user |
| Credential reference unresolved (`@profile.foo` not present) | Per-rule decision: `require = true` → deny request and log; `require = false` → forward without injection |
| Upstream host TLS error | Forward error to sandbox as plain HTTP 502 with header `X-Vibe-Broker-Error: tls-handshake` |
| Sandbox tries to reach a host with no rule | Deny with HTTP 451 + explanatory body; audit log records the attempt |
| User denies an approval-required host at the prompt | Same — 451 — and the rule remembers the denial for the rest of the session |
| Body-cap exceeded | Truncate response, return what was received plus header `X-Vibe-Broker-Truncated: true` |

## Recap integration

Egress events are aggregated into a per-session/per-job network-summary that feeds into the recap shape from `01-session.md` / `02-job.md`:

```jsonc
{
  "kind": "session",
  ...
  "artifacts": [
    ...
    { "kind": "network", "label": "egress summary",
      "locator": "audit:01HK.../egress",
      "summary": "27 requests · 4 hosts · 2 cloud-cred injections · 0 denied" }
  ]
}
```

The recap LLM prompt (when the user opts in to LLM recaps) gets a redacted summary of egress (hosts + counts, never bodies) so it can mention cloud activity in narrative form.

## Slicing plan

| Slice | What | Touches | Tests |
|---|---|---|---|
| **B1.1** | New crate `vibe-broker` skeleton: hyper accept loop on UDS, deny-all default, JSON audit emit | `vibe-broker` | Unit: deny-default, audit shape |
| **B1.2** | Policy DSL parsing + matching | `vibe-broker/policy.rs` | Unit: glob matching, method matching, path-prefix |
| **B1.3** | hickory-dns resolver + SSRF guard (port from `agent_executor.rs:21-56`) | `vibe-broker/dns.rs` + `ssrf.rs` | Unit: every banned range; live: dns-rebind defense |
| **B1.4** | rcgen-based per-broker root CA + ephemeral leaf cert minting | `vibe-broker/tls.rs` | Unit: cert chain validity; integration: sandbox curl handshakes |
| **B2.1** | Bearer + Basic injection | `vibe-broker/inject.rs` | Unit + live (httpbin.org) |
| **B2.2** | AWS SigV4 injection via `aws-sigv4` | `vibe-broker/inject.rs` | Live: real AWS S3 list-objects |
| **B2.3** | GCP IAM + Azure MSI injection | `vibe-broker/inject.rs` | Live: GCP storage list, Azure blob list |
| **B3** | IMDS faker on `169.254.169.254` (and GCP/Azure equivalents) | `vibe-broker/imds.rs` | Live: boto3 + gcloud + az inside a Tier-0 sandbox |
| **B4** | Tier-0 wiring: bind UDS into bwrap / .sb / AppContainer | per-platform native impls | E2E: curl from sandbox lands in audit log with correct policy_id |
| **B5** | Approval prompt for `require_user_consent` rules | `vibe-broker` + `agent.rs` ApprovalPolicy | UI integration in vibeui + REPL prompt |
| **B6** | Recap audit-summary integration | `vibe-broker` + recap | RTL: job recap shows network summary |

Each slice is independent; B1.* and B2.* can ship in parallel.

## Open questions

1. **Should the broker's CA be machine-rotating or per-launch?** Per-launch is safer; machine-rotating is more ergonomic. Recommend machine-rotating (one CA per host) with a 90-day rotation; per-launch is a follow-on hardening.
2. **Bidirectional mTLS for outbound?** Some enterprise APIs need a client cert. Supported via `Inject::MtlsClient`; v1 ships the type, v2 wires real SecretRef resolution for x509 keys (which want a different store than `ProfileStore`).
3. **WebSocket support.** Broker handles HTTP CONNECT for tunnels; WS upgrades work but escape per-message inspection. Document the limitation; recap notes WS sessions opaquely.
4. **gRPC.** HTTP/2 + protobuf — broker handles the transport; per-message policy is out of scope for v1.
5. **DNS-over-HTTPS / DoH.** If a tool resolves DNS over HTTPS bypassing the broker resolver, it'd hit the broker as a regular HTTPS request to `cloudflare-dns.com` etc. Allow-list per policy; default deny.
