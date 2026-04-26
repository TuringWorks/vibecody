# 04 — Tier-2 Hyperlight Backend

**Scope:** Hyperlight hypervisor partition as a `Sandbox` impl — hardware-boundary upgrade for WASM extensions on Linux + Windows.
**Parent:** [`README.md`](./README.md)
**Status:** Draft · 2026-04-26

---

## Why this tier exists

Tier-1 (Wasmtime + WASI 0.2 preopens) gives capability-based isolation with µs cold-start, but the boundary is software — a Wasmtime VM exploit gets you the host process's privileges. The current `vibe-extensions` crate uses Wasmtime 43 with no fuel, no timeout, no resource metering — every extension shares the daemon's address space.

Hyperlight wraps the **same Wasmtime guest** in a hypervisor partition (KVM / mshv on Linux, WHP on Windows). Cold-start stays in the µs range, per-instance memory is tens of KB, and a guest exploit hits the partition boundary instead of the host. It's Microsoft's pattern for Azure serverless function isolation.

This tier exists for one specific job: **hardening `vibe-extensions` without changing its API surface.** Extension authors keep writing WASM modules; the daemon swaps the runtime backend from raw Wasmtime to Wasmtime-in-Hyperlight on Linux + Windows hosts.

It's an **opt-in upgrade**, not the default. Cross-platform — falls back transparently to Tier-1 on macOS (Hypervisor.framework not supported by Hyperlight upstream).

## Goals

1. Hyperlight is a `Sandbox` impl behind the trait — same API surface as the other tiers.
2. The existing `vibe-extensions` extension API (Wasmtime-style host functions, `Store<HostState>`) keeps working — extensions don't recompile, they just run inside a hypervisor partition.
3. Cold-start ≤ 5 ms, per-instance memory ≤ 100 KB. Acceptable for *every* extension call, not just session-startup.
4. Network egress for extensions still goes through the broker (`02-egress-broker.md`) via host-function call, not via partition-internal sockets.
5. Falls back to Tier-1 (raw Wasmtime) on unsupported hosts with an audited downgrade event.

## Non-goals

- Running `bash` or arbitrary native binaries inside Hyperlight. The guest must be a purpose-built binary against `hyperlight_guest` — this is exactly what the wasmtime-on-hyperlight pattern provides; we don't go below it.
- Replacing Tier-3 Firecracker for opaque untrusted code. Firecracker hosts a full Linux guest; Hyperlight hosts one specific guest binary. Different shapes, different jobs.
- Per-extension hardware boundary on macOS. Hyperlight upstream doesn't target Hypervisor.framework. Fall back gracefully.

## What's there today

- `vibeui/crates/vibe-extensions/Cargo.toml:18` — Wasmtime 43.
- `vibe-extensions/src/loader.rs` — loads `.wasm` modules into per-extension `Store<HostState>` instances. Linker registers host functions. **No fuel, no timeout, no resource metering.**
- `TuringWorks/hyperlight` — vanilla mirror of `hyperlight-dev/hyperlight`. We can use upstream directly via the `hyperlight_host` and `hyperlight_guest` crates.

## Architecture

```
┌──── vibe-extensions (existing) ─────────────────────────────────────────-────┐
│                                                                              │
│   ExtensionRegistry                                                          │
│       └── load("path/to/ext.wasm")                                           │
│                                                                              │
│   ┌─ Tier-1 backend (default) ─-┐    ┌─ Tier-2 backend (opt-in) ────────--──┐│
│   │ wasmtime::Engine            │    │ hyperlight_host::UninitializedSandbox││
│   │ wasmtime::Store<HostState>  │    │   └── wasmtime guest binary          ││
│   │ wasmtime::Instance          │    │       (purpose-built; baked into     ││
│   │                             │    │        the daemon release)           ││
│   │ host_fn registration        │    │   └── host_fn calls cross the        ││
│   │   (in-process call)         │    │       partition boundary via         ││
│   │                             │    │       hyperlight's marshalling       ││
│   └─────────────────────────────┘    └──────────────────────────────────────┘│
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

The "wasmtime guest binary" inside the partition is a thin shim that:
1. Embeds Wasmtime in `no_std` mode
2. Receives the user's `.wasm` module bytes via host function on first call
3. Instantiates the module against a Wasmtime store inside the partition
4. Forwards host-function calls back to the embedder via Hyperlight's marshalling

This is the existing wasmtime-on-hyperlight pattern from upstream Hyperlight's examples — we don't invent it, we adopt it.

## Implementation sketch

```rust
// vibe-sandbox-hyperlight/src/lib.rs (Linux + Windows; cfg-gated)
pub struct HyperlightSandbox {
    // The partition + a wasmtime instance inside it
    sandbox: hyperlight_host::MultiUseSandbox,
    bind_paths: Vec<(PathBuf, PathBuf, BindMode)>,
    broker_socket: PathBuf,
    limits: ResourceLimits,
}

impl Sandbox for HyperlightSandbox {
    fn bind_rw(&mut self, host: &Path, guest: &Path) -> Result<()> {
        // Hyperlight has no FS — we expose host-function-based file access.
        // The guest WASM module sees a WASI preopen; the host-side handler
        // checks the path is inside `host` before serving the read/write.
        self.bind_paths.push((host.into(), guest.into(), BindMode::Rw));
        Ok(())
    }
    fn bind_ro(&mut self, host: &Path, guest: &Path) -> Result<()> {
        self.bind_paths.push((host.into(), guest.into(), BindMode::Ro));
        Ok(())
    }
    fn network(&mut self, policy: NetPolicy) {
        if let NetPolicy::Brokered { socket, .. } = policy {
            self.broker_socket = socket;
            // Register host function `vibe_egress_request` that forwards to broker
        }
    }
    fn spawn(&self, cmd: &OsStr, args: &[OsStr]) -> Result<Child> {
        // Hyperlight doesn't spawn processes — it calls functions.
        // For the extension-hosting use case, "spawn" means "call the entry function."
        // The trait's `spawn` is awkward here; we expose a richer API on top.
        anyhow::bail!("Tier-2 (Hyperlight) does not support process spawn — use call_function() for extensions");
    }
    fn tier(&self) -> SandboxTier { SandboxTier::Hyperlight }
    fn shutdown(self: Box<Self>) -> Result<()> {
        // MultiUseSandbox drop tears down the partition
        Ok(())
    }
}

// Extension-specific API — the part the trait can't capture cleanly
impl HyperlightSandbox {
    pub fn call_extension<I: Serialize, O: DeserializeOwned>(
        &mut self,
        function: &str,
        input: I,
    ) -> Result<O> {
        let bytes = serde_cbor::to_vec(&input)?;
        let result: Vec<u8> = self.sandbox.call(function, bytes)?;
        Ok(serde_cbor::from_slice(&result)?)
    }
}
```

The `spawn` mismatch is real: Hyperlight's whole shape is "host calls function in guest, gets result back." It's not a process model. The trait satisfies it for tiering uniformity, but the *real* extension API is `call_extension`, exposed on the concrete type and used directly by `vibe-extensions`. This is fine — extensions never needed `spawn` anyway.

## Filesystem model — host-mediated, not preopen

WASI preopens (Tier-1) work because Wasmtime intercepts WASI syscalls and routes them to the host's real FS, gated by the preopen list. Hyperlight has no FS — there's no kernel inside the partition. So we do the equivalent at a higher level:

1. The wasmtime-on-hyperlight guest exposes a WASI preopen at `/work` from the *guest's* point of view.
2. WASI fs syscalls in the guest are caught by Wasmtime-inside-partition, which calls a Hyperlight host function (`vibe_fs_read`, `vibe_fs_write`, `vibe_fs_open`, `vibe_fs_readdir`).
3. The host-function handler on the embedder side (the daemon) checks the path is inside a `bind_rw`/`bind_ro` and serves it from real disk.

Performance: each FS call is an extra hypercall. For the typical extension workload (read a config file once, do work in memory), this is negligible — single-digit microseconds per call.

## Network model — host-function call to broker

```
extension.wasm
    │  (calls wasi_http_outgoing_handler::handle())
    ▼
wasmtime-in-hyperlight guest
    │  (intercepts wasi:http/outgoing-handler;
    │   serializes request to bytes;
    │   calls Hyperlight host function vibe_egress_request)
    ▼
HyperlightSandbox::vibe_egress_request handler (host)
    │  (forwards bytes to broker over UDS / named pipe)
    ▼
vibe-broker
    │  (policy match, cred inject, etc.)
    ▼
internet
```

Same broker contract as every other tier. The audit log records `tier=hyperlight`.

## Why not just use Wasmtime fuel + epoch?

Wasmtime supports fuel (deterministic gas) and epoch interruption (wall-clock-driven yield). Both are options for Tier-1 hardening that don't need Hyperlight at all. The argument for Hyperlight is:

- Hardware boundary closes a class of vulnerabilities Wasmtime can't (compiler bugs, JIT escapes).
- Memory accounting is enforced by the partition, not by a software heuristic.
- Per-call partition boundary fits the threat model where extensions might be hostile, not just buggy.

If you only worry about buggy-but-honest extensions, Tier-1 + fuel + epoch is enough. Tier-2 is for when you might run extensions from an open marketplace and need the marketplace to be untrusted.

**Recommendation:** Tier-1 ships with fuel + epoch defaults turned on (independent of Hyperlight work). Tier-2 is opt-in for hosts that can run it. Both are valid choices; users pick.

## Performance numbers (target)

| Operation | Target | Notes |
|---|---|---|
| Cold-start (first call to a fresh extension) | ≤ 5 ms | partition creation + wasmtime guest init + module instantiation |
| Warm call (subsequent calls to MultiUseSandbox) | ≤ 100 µs | one hypercall + wasmtime function call |
| Per-instance memory | ≤ 100 KB | tens of KB per partition + wasmtime store |
| Host-function hypercall overhead | ≤ 2 µs | one VMENTER/VMEXIT cycle |
| Broker hop (per egress request) | ≤ 2 ms | hypercall + broker policy match |

At these numbers, Tier-2 is fast enough that the daemon could put *every* extension call inside Hyperlight without users noticing — but we still ship it opt-in to be conservative on the threat-model trade-off.

## Per-platform fall-through

| Host | What happens when Tier-2 is requested |
|---|---|
| Linux x86_64 (KVM) | Hyperlight via KVM |
| Linux aarch64 (KVM) | Hyperlight via KVM |
| Linux (mshv on Azure) | Hyperlight via mshv |
| Windows x86_64 (WHP) | Hyperlight via WHP |
| Windows aarch64 | Per upstream — currently TBD; fall through to Tier-1 if unavailable |
| macOS (any) | Falls back to Tier-1 with a `sandbox.downgrade` event |
| WSL2 | Tier-2 works because WSL2 is Linux + KVM |

The downgrade is transparent to the extension code (same API), audited in the recap, and emits a daemon-log warning so operators can see when fall-through is happening.

## Resource limits inside Hyperlight

Hyperlight exposes:
- Partition memory cap — set at creation, hard limit
- Stack and heap caps for the guest — set at creation
- Host-function call gas (fuel-equivalent) — guest can't run forever spinning host calls
- No CPU rate-limiting at the partition level today (upstream gap)

For wall-clock limits, the embedder uses a tokio kill timer that signals the partition to abort.

## Existing Wasmtime extensions — do they need changes?

No code changes for the WASM module itself. The wasmtime-on-hyperlight guest exposes the same WASI 0.2 surface. Host-function bindings might need a thin wrapper layer if extensions rely on shared-memory tricks that don't cross the partition cleanly — but the standard `wit-bindgen` flow Just Works.

Existing extensions should test on Tier-2 in CI; we expect ≥ 95% to pass with no changes.

## How the egress broker connects

| Aspect | How |
|---|---|
| Sandbox → broker transport | Host function `vibe_egress_request` registered on the partition; embedder forwards to broker via existing UDS / named pipe |
| TLS root CA | injected as bytes via host function call on first request |
| IMDS faking | broker exposes the same fake-IMDS responses; extension's WASI HTTP client hits them via the broker |

## Slicing plan

| Slice | What | Touches | Tests |
|---|---|---|---|
| **H0** | New crate `vibe-sandbox-hyperlight` (Linux+Windows cfg-gated) + impl skeleton; `spawn` returns `TierUnsupportedOperation` | new crate | compile gates |
| **H1** | Wasmtime-on-hyperlight guest binary + release pipeline (signed, cached) | new build job + crate | integration: trivial extension call returns expected output |
| **H2** | FS host functions (read/write/open/readdir) bound to `bind_rw` / `bind_ro` paths | crate | integration: extension reads `/work/foo.txt` and host sees the path-checked read |
| **H3** | Broker host function `vibe_egress_request` | crate + `vibe-broker` | integration: extension HTTPS call lands in broker audit log |
| **H4** | `vibe-extensions` Tier selector: try Hyperlight if available, fall back to Wasmtime | `vibe-extensions/loader.rs` | E2E: load same module twice, once each tier; both produce same result |
| **H5** | Tier-1 hardening (Wasmtime fuel + epoch) — done independently of H1–H4 since some users will stay on Tier-1 | `vibe-extensions` | unit: infinite loop in extension is killed at fuel/epoch deadline |
| **H6** | `vibecli doctor` reports Tier-2 availability per platform | `main.rs:15065-...` | manual verification on Linux+Windows+macOS |

H0–H4 are the critical path. H5 is independent and a hardening win regardless of H1–H4 timing.

## Failure modes

| Failure | Behavior |
|---|---|
| KVM/mshv/WHP not available on host | Tier-2 unavailable; transparent fall-through to Tier-1; doctor flags it |
| Wasmtime guest binary signature invalid | Refuse to load Tier-2; fall through |
| Partition crashes during `call_extension` | Sandbox marked dead; daemon drops it; subsequent calls re-create. Audit logs `outcome: partition_crashed` |
| Host function returns an error | Propagated to the WASI guest as a syscall error; extension handles it normally |
| Hyperlight version skew between host crate and guest binary | Caught at sandbox init; daemon refuses to use mismatched binaries |

## Testing

- **Linux CI matrix:** Ubuntu 22.04 + 24.04 (KVM), Azure Linux (mshv if runner permits)
- **Windows CI:** Windows 11 22H2 + Server 2022 with WHP
- **macOS CI:** verifies fall-through emits `sandbox.downgrade` event and Tier-1 takes over
- **Cross-tier consistency test:** the same extension run under Tier-1 and Tier-2 must produce byte-identical results for a deterministic input

## Open questions

1. **Hot-pool of partitions, like Firecracker?** Cold-start is already 5 ms — pooling buys us getting to ≤ 1 ms. Probably not worth the complexity in v1; revisit if profiling shows pain.
2. **Allow extensions to spawn child partitions?** The pattern would be "extension wants to run untrusted user code from a config file" — recursive sandboxing. Out of scope for v1.
3. **GPU / hardware accel.** Hyperlight partitions don't expose accelerators today; upstream may add it. Out of scope until upstream lands it.
4. **macOS Hypervisor.framework support.** Upstream Hyperlight doesn't target it. We *could* contribute a backend, but that's a multi-month effort orthogonal to this design. v1 ships with macOS fall-through to Tier-1.
5. **TuringWorks fork divergence.** Same as Firecracker — fork tracks upstream. If we ever need a patch (e.g., contributing macOS support), the fork makes it easy. Today: untouched.
