# 03 — Tier-3 Firecracker Backend

**Scope:** Firecracker microVM as a `Sandbox` impl — strongest isolation, opt-in, Linux host only.
**Parent:** [`README.md`](./README.md)
**Status:** Draft · 2026-04-26

---

## Why this tier exists

Tier-0 (bwrap+Landlock+seccomp / sandbox-exec / AppContainer) is plenty strong for a developer running their own code on their own laptop. There are two scenarios where it isn't enough and a hardware boundary is the right answer:

1. **The empty `CloudSandbox` slot** (`vibecli/vibecli-cli/src/cloud_sandbox.rs`, 382 lines, no backend wired). When a user spawns an agent that runs *opaque tenant code* on a remote VibeCody host — e.g. running a stranger's PR build, a third-party skill the user installed five minutes ago, or auto-fix tooling against a repository the user doesn't fully trust — Tier-0's syscall-allow-list approach is too permissive. Container escapes happen; namespace escapes happen; LSM bypasses happen. A KVM boundary doesn't.
2. **An opt-in "paranoid" mode on Linux dev workstations.** Some users will want this even on their own laptop for high-stakes runs (production-credential-touching scripts, untrusted skills from the marketplace).

Firecracker is the right backend for both: full hardware virtualization, ~5 MB VMM overhead, ~125 ms cold-start, real Linux guest kernel inside, in-tree seccomp on the VMM (the *jailer*) for defense-in-depth on the *outside*.

It's an **opt-in upgrade**, never the default. Linux only. Falls back transparently to Tier-0 on macOS / Windows.

## Goals

1. Firecracker is a drop-in `Sandbox` implementation behind the trait — same API as Tier-0.
2. **Rootfs ≤ 20 MB**, not multi-GB. Custom BusyBox + bash + Python-minimal + a tiny init.
3. Cold-start ≤ 200 ms wall (microVM boot + folder mount + broker connect).
4. The user's folder appears inside the guest at the same logical path (the path-remap difficulty is solvable here because it's a real Linux guest).
5. Network egress goes through the broker (`02-egress-broker.md`) via virtio-vsock, not via tap+iptables.
6. `CloudSandbox` slot picks this backend by default on Linux hosts.

## Non-goals

- Shipping Docker images. Tier-3 is for isolation, not for "run this image" — that's still `DockerRuntime`.
- Supporting macOS/Windows hosts directly. Falls back to Tier-0; this is documented.
- Running unmodified Linux distributions inside. The rootfs is purpose-built — small, predictable, no apt-get inside.
- Per-call microVMs for high-frequency workloads. 125 ms cold-start is too slow for that — that's Hyperlight's job (`04-hyperlight-tier.md`).

## What's there today

- `cloud_sandbox.rs` (382 lines) — `SandboxState` / `SandboxConfig` / `SandboxInstance` / `SandboxTemplate` / `CloudSandboxManager`. Pure data model with `create_instance` / `start_instance` / `stop_instance` / `list_instances` / `expire_instance` / `sync_files`. **No backend.**
- `opensandbox_client.rs` (1011 lines) — HTTP client for an external "OpenSandbox" service. Different problem; not in scope here.
- The `TuringWorks/firecracker` fork is a vanilla mirror of `firecracker-microvm/firecracker`. We can use upstream Firecracker directly via the `firecracker` binary at runtime, no need to vendor.

## Architecture

```
┌──── vibe-sandbox-firecracker (in-daemon) ────────────────────────────┐
│                                                                       │
│   FirecrackerSandbox                                                  │
│       ├── jailer process            ◀── seccomp + cgroups + ns-drop  │
│       │      └── firecracker (VMM)                                   │
│       │            └── KVM microVM                                   │
│       │                ├── kernel (~3 MB minimal config)             │
│       │                ├── rootfs (~10 MB BusyBox + bash)            │
│       │                ├── virtio-fs of /work ──▶ host: bound folder │
│       │                ├── virtio-vsock CID:guest ─▶ host broker    │
│       │                └── stdout/stderr → host pipes                │
│       │                                                               │
│       └── shutdown drain: SIGTERM jailer → firecracker → KVM exit    │
│                                                                       │
└──────────────────────────────────────────────────────────────────────┘
```

## Rootfs strategy — under 20 MB

A purpose-built rootfs, baked once per VibeCody release, shipped in the daemon:

```
vibe-firecracker-rootfs.img.zst    (~7 MB compressed, ~12 MB extracted)
├── /init                          (custom; mounts virtio-fs, starts vsock-proxy, exec's payload)
├── /bin/                          (BusyBox single static binary symlinked: sh, ls, cat, ...)
├── /bin/bash                      (static, ~2 MB)
├── /usr/bin/python3-minimal       (optional; +5 MB; off by default — selectable per call)
├── /lib/                          (musl libc; static where possible)
├── /etc/passwd /etc/group         (single user "vibe")
├── /etc/ssl/certs/                (per-broker CA injected at boot)
├── /work                          (mountpoint for virtio-fs; empty until host binds)
└── /run/vibe-broker.sock          (symlink to vsock-proxy socket)
```

**Build pipeline** (existing CI job pattern; adds one):

- Buildroot or Alpine `mkrootfs` produces `vibe-firecracker-rootfs.img`
- `zstd -22` to compress
- Detached signature with the existing release-signing key
- Daemon verifies signature on first use; cached at `~/.vibecli/firecracker/rootfs-{version}.img`

Kernel: a minimal 5.x or 6.x build, virtio drivers compiled in, no modules, no networking stack beyond what virtio-vsock needs (no TCP stack inside the guest — see "Network" below). ~3 MB compressed.

**Total disk footprint per release: ~10 MB** (kernel + rootfs). Not multi-GB.

## How the rootfs handles the "I want to run npm" case

The minimal rootfs has bash + busybox + a static Python — but no Node, no Rust, no Go. For richer toolchains, the design is:

1. **Toolchain layers:** the daemon ships a small set of optional layers (`node-lts.img`, `python3.12-full.img`, `rust-stable.img`, `golang.img`), each ~30–80 MB. Pulled lazily on first use; cached locally.
2. **User-mounted toolchains:** if the host already has the toolchain installed (most do), the daemon `bind_ro`s the toolchain dir into the guest. E.g., `/usr/local/bin/node` mounted from host. Same model as Tier-0.
3. **No image registry.** No Docker pulls. No multi-GB downloads. The biggest layer is ~80 MB (rust-stable), pulled once.

The user picks per-skill via `egress.toml`:

```toml
[sandbox.firecracker]
extra_layers = ["node-lts", "python3.12-full"]
```

If a layer isn't pulled yet, the daemon pulls + verifies signature before sandbox start. Subsequent calls hit cache.

## Implementation sketch

```rust
// vibe-sandbox-firecracker/src/lib.rs (Linux only — cfg-gated)
pub struct FirecrackerSandbox {
    vm_id: VmId,                      // ULID
    rootfs_path: PathBuf,             // shared base; virtio-blk read-only
    overlay_path: PathBuf,            // per-vm tmpfs overlay for writes outside /work
    bind_paths: Vec<(PathBuf, PathBuf, BindMode)>,
    broker_vsock_port: u32,
    limits: ResourceLimits,
    jailer_pid: Option<Pid>,
    socket: PathBuf,                  // firecracker API socket
}

impl Sandbox for FirecrackerSandbox {
    fn bind_rw(&mut self, host: &Path, guest: &Path) -> Result<()> {
        self.bind_paths.push((host.into(), guest.into(), BindMode::Rw));
        Ok(())
    }
    fn bind_ro(&mut self, host: &Path, guest: &Path) -> Result<()> {
        self.bind_paths.push((host.into(), guest.into(), BindMode::Ro));
        Ok(())
    }
    fn network(&mut self, policy: NetPolicy) {
        match policy {
            NetPolicy::Brokered { socket: _, policy_id } => {
                // No tap interface inside the guest.
                // /init starts a vsock-to-loopback proxy that exposes the broker
                // at 127.0.0.1:8080 inside the guest. Sandbox sets HTTP_PROXY=...
                self.broker_vsock_port = vsock_port_for(&policy_id);
            }
            NetPolicy::None => { /* no vsock, guest has no network at all */ }
            NetPolicy::Direct => {
                anyhow::bail!("Tier-3 does not support NetPolicy::Direct — broker is mandatory");
            }
        }
    }
    fn spawn(&self, cmd: &OsStr, args: &[OsStr]) -> Result<Child> {
        // 1) Boot the VM if not already booted (we can keep a warm pool — see "Pooling")
        if self.jailer_pid.is_none() {
            self.boot_vm()?;
        }
        // 2) Send execute payload over vsock to /init's command socket
        let req = ExecRequest { cmd: cmd.into(), args: args.into(), env: ... };
        let child = self.send_exec_via_vsock(req)?;
        Ok(child)
    }
    fn tier(&self) -> SandboxTier { SandboxTier::Firecracker }
    fn shutdown(self: Box<Self>) -> Result<()> {
        // SIGTERM jailer; jailer signals firecracker; KVM exits; tmpfs overlay freed
        Ok(())
    }
}
```

`/init` inside the guest is the orchestrator: mounts virtio-fs at `/work`, starts the vsock-loopback proxy (so HTTP_PROXY works against `127.0.0.1`), opens a vsock command channel for `spawn`, and waits.

## Folder bind via virtio-fs

`virtio-fs` (built into Firecracker since 1.0; uses `virtiofsd` on the host) gives us a real shared filesystem between host and guest with near-native performance and no GB image to build per call. Setup:

1. Daemon spawns `virtiofsd --shared-dir <host_folder> --socket <uds>` per bound dir
2. Firecracker config includes `vhost-user-fs` device pointing at that socket
3. Guest's `/init` mounts `virtio-fs` at the desired mountpoint

Performance: reads/writes go to host page cache directly. Effectively free for everything except `fsync`, which has the usual VM overhead.

This solves the path-remap problem cleanly — the guest sees `/work`, the host sees `/Users/me/myrepo`, and reads/writes are coherent.

## Network — vsock-only

The microVM has **no virtual network interface**. No tap, no virtio-net, no IP stack inside the guest. The only outside-world channel is virtio-vsock to the host broker.

Inside the guest, `/init` runs a tiny Rust binary (`vsock-proxy`, ~500 LoC) that:
- Listens on `127.0.0.1:8080` (TCP, loopback only — no NIC needed for loopback)
- Forwards each connection over vsock to host CID 2, port `broker_vsock_port`
- The host's broker accepts vsock just like UDS (hyper has vsock-listener support via the `tokio-vsock` crate)

The sandbox sees `HTTP_PROXY=http://127.0.0.1:8080` and "just works." The broker sees a regular HTTP CONNECT just like any other tier.

This eliminates an entire class of network attacks (no IP stack in the guest = no SSRF inside the VM, no DNS resolution inside the VM, no outbound TCP that bypasses the broker).

## VM pooling — amortizing the 125 ms boot

For interactive workloads, a 125 ms cold-start per command is irritating. The trick used by AWS Lambda and Fly.io: **pre-boot a pool of stub VMs**, each with the rootfs loaded but no payload running. On `spawn()`, attach the user's bound folders and signal `/init` to exec the command — that step is < 5 ms.

Pool config (in `~/.vibecli/firecracker/pool.toml`):

```toml
[pool]
warm_vms = 2          # always have N pre-booted, ready to attach folders
max_total = 8
idle_ttl  = "5m"      # idle pooled VMs killed after this
preload_layers = ["node-lts"]  # toolchains pre-mounted into the pool
```

Trade-off: pooled VMs eat memory while idle (~10 MB each). Tunable.

## Jailer + VMM-side hardening

Firecracker's jailer is a separate binary that wraps the firecracker VMM in:
- A new mount/PID/network/IPC/UTS namespace
- A per-VM cgroup with CPU + memory caps
- A seccomp filter allowing only the syscalls the VMM actually needs (small allow-list, in-tree)
- A privileges drop to a UID + GID dedicated to this VM

The daemon spawns `jailer firecracker` with config — never `firecracker` directly. This is in the upstream docs; we follow it verbatim.

## Resource limits

Firecracker's REST API exposes per-VM:
- vCPU count
- Memory size
- Disk rate-limiting (BW + IOPS)
- Network rate-limiting (n/a here — no NIC)

These map directly onto `ResourceLimits` from the trait:

```rust
fn limits(&mut self, l: ResourceLimits) {
    self.vm_config.vcpu_count = l.cpu_quota_ms_per_sec.map(|q| (q + 999) / 1000).unwrap_or(1);
    self.vm_config.mem_size_mib = l.memory_bytes.map(|b| b >> 20).unwrap_or(256) as u32;
    // wall_clock handled by tokio kill timer outside the VM
    // pids: ulimit RLIMIT_NPROC inside the guest set by /init before exec
}
```

## How the egress broker connects

| Aspect | How |
|---|---|
| Sandbox → broker transport | virtio-vsock host CID 2 |
| Broker listening | `tokio-vsock` listener on the host alongside the existing UDS listener |
| TLS root CA | injected into the rootfs at boot via `/etc/ssl/certs/vibe-sandbox-ca.pem` (ephemeral, per-VM if pooled per-session; per-pool if not) |
| IMDS faking | broker exposes `169.254.169.254` traffic via the same vsock channel; vsock-proxy in guest NAT-routes it |

Net result: identical broker contract for all four tiers. The recap audit log shows tier=`firecracker` instead of tier=`native`, but everything else is the same.

## Per-platform fall-through

| Host | What happens when user requests Tier-3 |
|---|---|
| Linux x86_64 | Firecracker; full feature set |
| Linux aarch64 | Firecracker; full feature set (Graviton, Apple Silicon Linux VMs) |
| macOS | Falls back to Tier-0 with a `sandbox.downgrade` event in the recap stream and a daemon-log warning |
| Windows | Same — Tier-0 fallback, downgrade event |

The downgrade is *transparent* but *audited*. The recap and the user can see "asked for Firecracker, got Tier-0" so it isn't silently weakened.

For users who *must* have Firecracker on macOS/Windows, the path is "use vibe-indexer or a Linux remote daemon." Out of scope to add macOS/Windows hypervisor support — Apple's Hypervisor.framework and Windows Hyper-V both want full guest images, which contradicts the rootfs-size goal.

## Performance numbers (target)

| Operation | Target | Notes |
|---|---|---|
| Cold-boot (no pool) | ≤ 200 ms wall | jailer + firecracker + kernel + /init + virtio-fs mount |
| Warm-attach (from pool) | ≤ 10 ms | folders bind via virtio-fs, /init signals payload |
| `spawn` after attach | ≤ 5 ms | vsock command + execve in guest |
| Per-VM RAM (idle) | ≤ 10 MB | minimal kernel + rootfs |
| Per-VM RAM (working) | depends on payload | your npm install, your problem |
| Egress request (broker hop) | ≤ 2 ms | one vsock hop + broker policy match |

## Failure modes

| Failure | Behavior |
|---|---|
| Host kernel KVM not present | Daemon detects at startup; Tier-3 unavailable; transparent fall-through to Tier-0 |
| `firecracker` / `jailer` binaries not present | Same as above; daemon prints install hint via `vibecli doctor` |
| Rootfs signature invalid | Refuse to boot; user told to re-pull a release |
| virtiofsd absent | Daemon ships a vendored binary alongside the rootfs |
| VM crashes mid-execution | `/init` not reachable; daemon kills jailer, drops VM, returns error to caller. Audit log marks `outcome: vm_crashed` |
| Guest tries to bypass broker (it can't — no NIC) | n/a — design-prevented |
| Pool exhaustion | New requests block on a `tokio::Semaphore`; if `max_total` reached and all busy, request errors with `SandboxError::PoolExhausted` |

## Testing

- **CI Linux x86_64:** real Firecracker boot, real vsock, real broker. Smoke test: bind a temp folder rw, run `bash -c "echo hi > /work/out"`, verify host file appears.
- **CI Linux aarch64:** GitHub-hosted ARM runner or Graviton self-hosted; same suite.
- **macOS / Windows CI:** verify fall-through emits the downgrade event and Tier-0 takes over.

## Slicing plan

| Slice | What | Touches | Tests |
|---|---|---|---|
| **F0** | New crate `vibe-sandbox-firecracker` (Linux-cfg-gated) + impl skeleton, panics in `spawn` | new crate | compile gates |
| **F1** | Rootfs builder CI job + signed-release artifact + version detection | `.github/workflows/release.yml`, `Makefile`, daemon download/verify path | release artifacts present, signature verifies |
| **F2** | Boot-once-per-call: jailer + firecracker + minimal kernel + rootfs + tmpfs overlay; no folder bind, no broker | crate | integration: `bash -c "echo hi"` returns "hi" via stdout |
| **F3** | virtio-fs bind via virtiofsd; folder rw works | crate | integration: write `/work/out`, host sees it |
| **F4** | virtio-vsock + broker bridge; `HTTP_PROXY` works | crate + `vibe-broker` vsock listener | integration: `curl https://example.com` from inside reaches broker, lands in audit log |
| **F5** | VM pool + warm-attach | crate | benchmark: warm-attach < 10 ms, cold < 200 ms |
| **F6** | Toolchain layers (node, python3-full, rust, go) | crate + release pipeline | integration: `npm --version` works in a node-layer sandbox |
| **F7** | `CloudSandbox` slot consumes `vibe-sandbox-firecracker` | `cloud_sandbox.rs` | integration: existing CloudSandboxManager API now spawns real VMs |
| **F8** | Per-skill `[sandbox.firecracker]` config + auto-upgrade rules from `README.md` policy | `vibe-ai/src/skills.rs`, daemon | E2E: marketplace skill auto-runs in Firecracker on Linux |

F0–F4 are the critical path; F5–F8 are post-MVP polish.

## Open questions

1. **Rootfs distribution.** Bake into the daemon binary (resources include macros) or download on first use? Recommend download-on-first-use with a 7 MB payload — keeps the daemon binary small.
2. **GPU passthrough.** Firecracker doesn't support GPU. For ML workloads, fall back to native or tell the user to use a separate runtime. Out of scope for v1.
3. **Snapshot/restore.** Firecracker supports VM snapshots; warm-pool from a snapshot can hit < 30 ms cold-start. Worth doing, but adds complexity. Defer to v2.
4. **Multi-tenancy on a shared host.** Daemon runs as a single user; jailer drops to per-VM UIDs. Adequate for single-user workstations; cluster-grade multi-tenancy (e.g. shared CI host) needs an additional outer layer not designed here.
5. **TuringWorks fork divergence.** If we ever need to patch Firecracker (e.g. for a missing feature), the fork makes that easy. Today it tracks upstream verbatim — keep it that way unless a specific need surfaces.
