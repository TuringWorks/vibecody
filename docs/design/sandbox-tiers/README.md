# Sandbox Tiers — Design Index

**Status:** Draft · 2026-04-26
**Scope:** vibecli daemon (Rust) — used by vibecoder, vibemobile, vibewatch, vscode-extension, jetbrains-plugin, neovim-plugin, agent-sdk
**Owner:** TBD

---

## What this is

A unified sandboxing architecture for VibeCody's daemon that lets the user point at a folder and run **read / modify / write / shell commands and scripts inside it with kernel-enforced isolation, on all three desktop OSes, plus full network access mediated by a policy-driven egress broker** — without GB-scale images or second-scale boots in the default path. Two opt-in upgrade tiers (Firecracker microVM, Hyperlight hypervisor partition) cover the niches where the lightweight default is not strong enough.

## The four-tier model

| Tier | Backend | Host platforms | Cold-start | Per-instance overhead | Best for |
|---|---|---|---|---|---|
| **Tier-0: Native** (default) | bwrap+Landlock+seccomp / sandbox-exec / AppContainer | Linux, macOS, Windows | < 50 ms | KB | The user's stated goal — folder + bash + brokered network on every dev workstation |
| **Tier-1: WASI** (existing, hardened) | Wasmtime + WASI 0.2 preopens | All | µs | < 1 MB | Tools/skills implemented as WASM modules; cross-platform without per-OS code |
| **Tier-2: Hyperlight** | Hypervisor partition (KVM / mshv / WHP) | Linux + Windows | < 1 ms | tens of KB | Hardware-boundary upgrade for Tier-1 (WASM extensions) where Wasmtime alone is not enough |
| **Tier-3: Firecracker** | KVM microVM, full Linux guest | Linux | ~ 125 ms | 5 MB + tiny rootfs (~10 MB BusyBox) | Strongest isolation for opaque/untrusted workloads — fills the empty `CloudSandbox` slot, plus an opt-in "paranoid mode" on Linux dev workstations |

**Egress broker** is *orthogonal* to all four tiers — every tier connects to the same broker and gets the same network policy, credential injection, and audit log surface.

The default path for every interactive workload is Tier-0 + broker. The other tiers are opt-in upgrades, selected per-job by policy or by an explicit user request.

## Goals

1. **One API across tiers.** Picking a stronger boundary is a config knob, not a code rewrite.
2. **No GB images or second-scale boots in the default.** Tier-0 hits the bar; Tier-3 is opt-in only.
3. **Network is a first-class capability,** not a binary on/off. Every byte of egress goes through the broker, every cloud SDK keeps working without the sandbox seeing root credentials.
4. **Cross-platform parity in posture, not in implementation.** Same `Sandbox` trait, three native impls; user sees the same guarantees on macOS as Linux.
5. **Audit + recap integration.** Every sandbox session emits a structured log of FS access, exec'd commands, and network egress that feeds into the recap system designed in `docs/design/recap-resume/`.

## Non-goals

- A single cross-platform sandbox primitive. There isn't one that meets the lightness bar; we wrap three native primitives behind one trait.
- Replacing Docker / Podman as a packaging mechanism. Tier-3 (Firecracker) is for *isolation*, not for shipping container images. Existing `DockerRuntime` stays for "give me a container with this image" workflows.
- Reinventing `vibe-extensions`. Wasmtime stays as Tier-1; Hyperlight is a *backend swap* for the same crate, not a rewrite.

## What exists today (grounded in `b1e28ad1`)

| Surface | File | State |
|---|---|---|
| `BwrapProfile` (Linux config builder) | `vibecli/vibecli-cli/src/sandbox_bwrap.rs` | Builder only; **`#[allow(dead_code)]` in `main.rs:2355` — never invoked** |
| `WindowsSandboxConfig` (Windows policy struct) | `vibecli/vibecli-cli/src/sandbox_windows.rs` | Allow/deny path policy, **no kernel enforcement** |
| `tool_executor.rs::with_no_network()` | `vibecli/vibecli-cli/src/tool_executor.rs:151,371-373` | Wraps shell with `sandbox-exec -n no-network` (macOS) / `unshare --net` (Linux) — **network only, no FS isolation** |
| Regex command blocklist | `tool_executor.rs:16-67` | Rejects `rm -rf /`, fork bombs, `curl|sh` etc. — defense-in-depth, not isolation |
| `agent_executor.rs` SSRF guard | `vibecoder/src-tauri/src/agent_executor.rs:21-56` | Per-call URL validation for `fetch_url` — **explicitly notes (line 4) that the Tauri path skips bwrap/sandbox-exec entirely** |
| `DockerRuntime` + `ContainerRuntime` trait | `vibecli/vibecli-cli/src/docker_runtime.rs`, `container_runtime.rs` | Real Docker integration with iptables egress allow-list (`docker_runtime.rs:99-136`) — **the prior art for the broker pattern** |
| `OpenSandboxClient` | `opensandbox_client.rs` (1011 lines) | HTTP client for an external "OpenSandbox" service; not a local runtime |
| `CloudSandboxManager` | `cloud_sandbox.rs` (382 lines) | Pure in-memory data model — instance lifecycle but **no backend wired** |
| `Wasmtime 43.0` for `vibe-extensions` | `vibecoder/crates/vibe-extensions/Cargo.toml:18`, `loader.rs` | Loaded into separate `Store<HostState>`s; **no fuel/timeout/resource metering** |
| `ApprovalPolicy` enum | `vibecoder/crates/vibe-ai/src/agent.rs` | `ChatOnly` / `ReadOnly` / `Suggest` / `AutoEdit` / `FullAuto` — **policy gate, not isolation** |
| `config.safety.sandbox: bool` | `vibecli/vibecli-cli/src/config.rs:1268` | TOML toggle; on Linux/mac it would invoke bwrap/sandbox-exec — **the toggle exists but the wrap is partial (network only)** |

So most of the *parts* are present. The work is wiring them, replacing the Linux/Windows stubs with real enforcement, adding the egress broker, and registering Firecracker + Hyperlight as additional `Sandbox` backends.

## The unified `Sandbox` trait

A new crate `vibecli/crates/vibe-sandbox/` (workspace member) with one trait, four implementations:

```rust
// vibe-sandbox/src/lib.rs
pub trait Sandbox: Send + Sync {
    /// Bind a host folder into the sandbox, read-write.
    fn bind_rw(&mut self, host: &Path, guest: &Path) -> Result<()>;

    /// Bind a host folder into the sandbox, read-only.
    fn bind_ro(&mut self, host: &Path, guest: &Path) -> Result<()>;

    /// Set environment policy. Default: clear all, then add explicit vars.
    fn env(&mut self, policy: EnvPolicy);

    /// Resource limits.
    fn limits(&mut self, limits: ResourceLimits);

    /// Network policy. Default: deny all. `Brokered` connects to vibe-broker.
    fn network(&mut self, policy: NetPolicy);

    /// Spawn a process inside the sandbox.
    fn spawn(&self, cmd: &OsStr, args: &[OsStr]) -> Result<Child>;

    /// Tier identifier for telemetry / recap.
    fn tier(&self) -> SandboxTier;

    /// Best-effort cleanup. Drop also cleans up.
    fn shutdown(self: Box<Self>) -> Result<()>;
}

pub enum SandboxTier { Native, Wasi, Hyperlight, Firecracker }

pub enum NetPolicy {
    None,
    Brokered { socket: BrokerSocket, policy_id: PolicyId },
    Direct,                             // Tier-0 / Tier-3 only; not recommended
}

pub enum EnvPolicy {
    Clear,                              // default
    Pass(Vec<String>),                  // explicit allow-list
    Inherit { strip_secrets: bool },    // dev convenience; strip_secrets is on by default
}

pub struct ResourceLimits {
    pub cpu_quota_ms_per_sec: Option<u32>,
    pub memory_bytes: Option<u64>,
    pub pids: Option<u32>,
    pub wall_clock: Option<Duration>,
    pub max_open_files: Option<u32>,
}

pub fn select(tier: SandboxTier) -> Result<Box<dyn Sandbox>>;
pub fn native() -> Result<Box<dyn Sandbox>>;   // OS-appropriate Tier-0
```

Implementations live in sub-crates so optional dependencies stay opt-in:

```
vibecli/crates/
├── vibe-sandbox/                    # trait + tier selection + ResourceLimits + EnvPolicy types
├── vibe-sandbox-native/             # Tier-0
│   ├── linux.rs   → bwrap + landlock-rs + seccompiler
│   ├── macos.rs   → sandbox-exec + .sb profile generator
│   └── windows.rs → AppContainer via windows-rs + Restricted Token + Job Object
├── vibe-sandbox-wasi/               # Tier-1 (extends existing vibe-extensions)
├── vibe-sandbox-hyperlight/         # Tier-2 (Linux + Windows only; cfg-gated)
├── vibe-sandbox-firecracker/        # Tier-3 (Linux only; cfg-gated)
└── vibe-broker/                     # the egress broker (next section)
```

Tier crates that aren't built on a given platform compile to empty stubs that error politely (`SandboxError::TierUnsupported(tier, platform)`), so the daemon binary is the same on every OS.

## Egress broker overview

(Full design: [`02-egress-broker.md`](./02-egress-broker.md).)

```
sandbox process ─── (Unix socket | named pipe | virtio-vsock) ───▶ vibe-broker ───▶ internet
                                                                       │
                                                                       ├── reads ProfileStore + WorkspaceStore for credentials
                                                                       ├── per-tool policy (allow-listed hosts, methods, body caps)
                                                                       ├── DNS resolution (sandbox has none)
                                                                       ├── HTTPS interception with per-broker root CA
                                                                       ├── credential injection (SigV4 / Bearer / OAuth)
                                                                       ├── IMDS faking for cloud SDKs
                                                                       └── per-request audit log → recap stream
```

Every Tier connects via a different transport:
- Tier-0 Linux: Unix domain socket bind-mounted into the namespace
- Tier-0 macOS: Unix domain socket allow-listed in the `.sb` profile
- Tier-0 Windows: named pipe granted to the AppContainer
- Tier-1 WASI: `wasi:http/outgoing-handler` host-implemented — the broker *is* the implementation
- Tier-2 Hyperlight: host-function call into the embedder; embedder forwards to broker
- Tier-3 Firecracker: virtio-vsock from guest → broker on host (or in-cluster broker for cloud sandboxes)

The sandboxed process always sees `HTTP_PROXY=…` / `HTTPS_PROXY=…` / `NODE_EXTRA_CA_CERTS=…` / equivalents, and the broker terminates TLS with its own short-lived CA installed only inside the sandbox. From a tool's perspective, network "just works" — it just goes through policy.

## Tier-selection policy

Default policy lives in `config.toml`:

```toml
[sandbox]
default_tier = "native"          # Tier-0 for all interactive workloads

[sandbox.upgrade_rules]
# Workloads that auto-upgrade to a stronger tier
untrusted_skill = "firecracker"  # Linux fallback to native if Firecracker absent
wasm_extension  = "hyperlight"   # Linux/Windows fallback to wasi if Hyperlight absent
cloud_sandbox   = "firecracker"  # the CloudSandbox slot

[sandbox.broker]
enabled = true
listen  = "unix:/run/vibe-broker.sock"
ca_path = "~/.vibecli/sandbox-ca/"
```

Per-skill overrides ride on the existing `Skill` definition (`vibe-ai/src/skills.rs`):

```toml
# vibecli/skills/foo/skill.toml
[sandbox]
tier = "firecracker"             # opt this skill into a stronger boundary
network = "brokered"
egress_policy = "egress.toml"    # see 02-egress-broker.md for shape
```

## Default workflow — what it looks like end-to-end

User says "sandbox this repo and run my npm test":

```
1. daemon picks Tier-0 (default), builds a Sandbox via vibe-sandbox::native()
2. sandbox.bind_rw("/Users/me/myrepo", "/work")
3. sandbox.bind_ro("/usr", "/usr")  (toolchain on Linux/mac; not needed on Windows AppContainer)
4. sandbox.env(EnvPolicy::Pass(vec!["PATH","HOME","LANG"]))
5. sandbox.network(NetPolicy::Brokered { socket: broker_uds, policy_id: "skill:npm-test" })
6. sandbox.limits(ResourceLimits { cpu_quota_ms_per_sec: Some(2000), memory_bytes: Some(4 << 30), wall_clock: Some(Duration::from_secs(600)), .. })
7. sandbox.spawn("/bin/bash", &["-c", "cd /work && npm test"])
8. broker logs every npm-registry fetch, every node OAuth call, etc.
9. on exit, broker emits an audit summary; recap subsystem ingests it
10. sandbox.shutdown() — namespace torn down, mounts unmounted
```

Cold path total: < 50 ms on Linux/macOS, < 80 ms on Windows. No images. No GB. The user's bash sees `/work` and that's it.

## Sequencing

| Phase | What | Crate(s) | Blocker for | Status |
|---|---|---|---|---|
| **S0** | Workspace skeleton: `vibe-sandbox` trait + `SandboxTier`/`NetPolicy`/`EnvPolicy`/`ResourceLimits` types + `select()` + platform stubs | `vibe-sandbox` | everything | ✅ shipped |
| **S1.1** | Tier-0 Linux: bwrap invocation + Landlock layer + seccomp filter | `vibe-sandbox-native/linux` | S2 | ✅ shipped (bwrap); Landlock + seccomp pending |
| **S1.2** | Tier-0 macOS: `.sb` profile generator + sandbox-exec invocation | `vibe-sandbox-native/macos` | S2 | ✅ shipped |
| **S1.3** | Tier-0 Windows: AppContainer + Restricted Token + Job Object via `windows-rs` | `vibe-sandbox-native/windows` | S2 | ✅ shipped (AppContainer + Job Object); Restricted Token + WFP pending |
| **S2** | Egress broker: `hyper` + `rustls` + `rcgen` + `hickory-dns` + policy DSL | `vibe-broker` | S3 | ✅ shipped |
| **S3** | Wire tool_executor → vibe-sandbox; remove the dead-code attribute on `sandbox_bwrap` | `vibecli-cli` integration | S4+ | ✅ shipped 2026-05-18 — `run_in_native_sandbox` routes shell tool through `Box<dyn Sandbox>`; legacy `sandbox_bwrap` kept (has BDD tests) but no longer the production path |
| **S4** | Audit log → recap integration (cross-references `docs/design/recap-resume/02-job.md`) | `vibe-broker` + recap | release | pending |
| **T2.1 / H0** | Tier-2 Hyperlight crate scaffold + state-tracking impl, behind cfg | `vibe-sandbox-hyperlight` | parallel with S2-S4 | ✅ shipped 2026-05-18 |
| **T2.5 / H5** | Tier-1 hardening — Wasmtime fuel + epoch on `vibe-extensions` | `vibe-extensions` | independent | ✅ shipped 2026-05-18 |
| **T2 / H6 preview** | `make sandbox-doctor` standalone host probe — Tier-0/1/2/3 availability + Firecracker rootfs presence + JSON contract | `scripts/check-sandbox-tiers.sh`, `vibe-sandbox/tests/sandbox_doctor_probe.rs` | independent | ✅ shipped 2026-05-19 |
| **T3.1 / F0** | Tier-3 Firecracker crate scaffold + state-tracking impl | `vibe-sandbox-firecracker` | parallel with S2-S4 | ✅ shipped 2026-05-18 |
| **T3.1 / F1** | Minimal BusyBox+bash rootfs builder + Makefile target + CI publish + cosign keyless attestation + daemon-side `RootfsManager` (verify + content-addressed cache + cosign passthrough) | `scripts/`, `Makefile`, `release.yml`, `vibe-sandbox-firecracker::rootfs` | T3.1.B | ✅ shipped 2026-05-19 (incl. F1.5 daemon-side) |
| **T3.1 / F2.1** | Firecracker REST API request shapes (`/boot-source`, `/machine-config`, `/drives`, `/vsock`, `/actions`) + ordered `boot_sequence()` | `vibe-sandbox-firecracker::api` | T3.1.B | ✅ shipped 2026-05-19 |
| **T3.1 / F2.2-A** | HTTP-over-UDS client for Firecracker API socket (sync, std-only, no hyper) | `vibe-sandbox-firecracker::api_client` | T3.1.B | ✅ shipped 2026-05-19 |
| **T3.1 / F2.2-B** | firecracker + jailer argv builders (vm-id validation, log-level, cgroup syntax) | `vibe-sandbox-firecracker::process` | T3.1.B | ✅ shipped 2026-05-19 |
| **T3.1 / F3.1** | virtio-fs share + virtiofsd argv builder (deny-list re-validation at construction) | `vibe-sandbox-firecracker::virtiofs` | T3.1.B | ✅ shipped 2026-05-19 |
| **T3.1 / F4.1** | vsock broker bridge config + daemon↔broker handshake (`PolicyHandshake` / `BridgeAttachResponse`) + kernel env-vars + cmdline fragment | `vibe-sandbox-firecracker::bridge` | T3.1.B | ✅ shipped 2026-05-19 |
| **T3.1 / F8** | Per-skill `SkillSandboxPolicy` schema (composes F2.1 + F3.1 + F4.1 + env + boot args + wall-clock timeout, validates at load time, composes `VmConfig`) | `vibe-sandbox-firecracker::skill_policy` | skill-manifest integration | ✅ shipped 2026-05-19 |
| **T3.1.B / F2.2-C, F3.2, F4.2, F5, F6, F7** | microVM process spawn orchestration, virtiofsd process spawn, vsock listener + in-guest shim, VM warm-pool, toolchain layers, `CloudSandbox` wiring | `vibe-sandbox-firecracker` (Linux-only) | depends on Linux+KVM hardware in CI runner + firecracker/virtiofsd binaries | pending |
| **T2 / H1–H4** | Wasmtime-on-Hyperlight guest binary release pipeline, FS + broker host functions, `vibe-extensions` Tier selector | `vibe-sandbox-hyperlight`, `vibe-extensions` | depends on Linux+KVM/mshv or Windows+WHP CI runner | pending |
| **T2 / H6 daemon-side** | `vibecli doctor` integration — port the sandbox-doctor probe into `main.rs` so the daemon's own health endpoint reports tier availability | `vibecli-cli/main.rs` | currently blocked by the local Metal-toolchain `cargo check -p vibecli` failure (pre-existing dev env issue) | pending |
| **R** | Release: docs in `docs/release.md` + `docs/CHANGELOG.md` + `docs/security.md` posture update | release | — | ongoing |

S0–S4 are the critical path for the user's stated goal. T2 and T3 can land in parallel without blocking.

## Cross-cutting invariants

- **All four tiers go through the same broker for network.** No tier gets a fast-path that skips it.
- **Sandbox env is sanitized by default.** `EnvPolicy::Inherit` requires `strip_secrets: true` to be the default; the user can opt out per-call.
- **No tier sees ProfileStore or WorkspaceStore secrets.** Credentials are injected at the broker, not in the sandbox env.
- **Default network policy is `Brokered`,** not `None` and not `Direct`. `None` is for offline workloads; `Direct` is for trusted tooling only and emits a daemon log warning.
- **Tier downgrade emits a structured warning event.** Asking for Firecracker on a non-Linux host transparently falls back to Tier-0 *and* fires a `sandbox.downgrade` event so the recap can record it.
- **Audit log is mandatory.** All four tiers route through the broker's audit emitter; there is no path to "just turn it off."

## Per-platform support matrix

| Tier | Linux x86_64 | Linux aarch64 | macOS x86_64 | macOS Apple Silicon | Windows x86_64 | Windows aarch64 |
|---|---|---|---|---|---|---|
| **Native** | ✅ bwrap+Landlock+seccomp | ✅ same | ✅ sandbox-exec | ✅ same | ✅ AppContainer | ✅ AppContainer |
| **WASI** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Hyperlight** | ✅ KVM/mshv | ✅ KVM | ❌ falls back to Tier-1 | ❌ falls back to Tier-1 | ✅ WHP | ⚠️ WHP support TBD upstream |
| **Firecracker** | ✅ KVM | ✅ KVM | ❌ falls back to Tier-0 | ❌ falls back to Tier-0 | ❌ falls back to Tier-0 | ❌ falls back to Tier-0 |

## Cross-references

- [`AGENTS.md`](../../../AGENTS.md) — daemon is single source of truth; sandbox crate is daemon-side
- [`CLAUDE.md`](../../../CLAUDE.md) — storage rules (broker reads `ProfileStore` + `WorkspaceStore` for credentials)
- [`docs/design/recap-resume/02-job.md`](../recap-resume/02-job.md) — audit-log → recap integration
- [`docs/connectivity.md`](../../connectivity.md) — mobile/watch transport stack (broker is reachable from inside the daemon, not exposed to mobile/watch)

## Status table

| Doc | State |
|---|---|
| `README.md` (this) | Draft |
| `01-native-tier.md` | Draft |
| `02-egress-broker.md` | Draft |
| `03-firecracker-tier.md` | Draft |
| `04-hyperlight-tier.md` | Draft |
