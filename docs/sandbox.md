---
layout: page
title: Sandbox
permalink: /sandbox/
---

# Sandbox

> Every shell command an agent runs through VibeCody passes through a sandbox layer. Tier-0 (Native) is the only tier shipping today and delivers **network isolation everywhere + filesystem isolation on Linux + Windows**. macOS filesystem isolation, Landlock + seccomp on Linux, and Tiers 1-3 (WASI, Hyperlight, Firecracker) are explicitly deferred — that's a deliberate scope choice and this page is honest about what you do and don't get.

The full multi-tier design is in [`docs/design/sandbox-tiers/`](https://github.com/TuringWorks/vibecody/tree/main/docs/design/sandbox-tiers/). This page documents what *ships*.

---

## What you get today (Tier-0 Native)

| OS | Network isolation | Filesystem isolation | Landlock / seccomp | Notes |
|---|---|---|---|---|
| **Linux** | ✅ via `unshare --net` | ⚠️ requires `bwrap` on PATH | ❌ deferred (slices N1.2 / N1.3) | Set `network_disabled` to opt in |
| **macOS** | ✅ via `sandbox-exec -n no-network` | ❌ Seatbelt FS profile deferred | n/a | Built-in to all macOS installs |
| **Windows** | ✅ Job Object network restriction | ✅ AppContainer + Restricted Token | n/a | Lightly tested in production |

**Network isolation** means: subprocesses spawned through `tool_executor` cannot reach the internet. They get a fresh network namespace on Linux, Seatbelt's `no-network` Apple framework on macOS, or a Job Object with no network rights on Windows.

**Filesystem isolation** is the harder problem: the agent's working directory is exposed (otherwise edits don't take effect) but the rest of the system shouldn't be readable or writable. Today only Linux (via `bwrap` if installed) and Windows (via AppContainer) deliver this. macOS users get network isolation only — a malicious tool call could still read `~/.ssh` or write to `/etc`. **If that's not acceptable, run VibeCody under a dedicated macOS user account or in a VM.**

---

## What's NOT shipping (and where you can read what we mean)

The marketing surface around "sandbox tiers" is bigger than the code. To prevent that gap from misleading anyone:

| Item | Status | Where it's planned |
|---|---|---|
| **Landlock rules on Linux** | Deferred | Slice N1.2 — `docs/design/sandbox-tiers/01-native-tier.md` |
| **seccomp BPF filter** | Deferred | Slice N1.3 — same doc |
| **macOS Seatbelt FS profile** | Deferred | Same doc, "macOS coverage" section |
| **Egress broker** | Designed, not wired | `docs/design/sandbox-tiers/02-egress-broker.md` |
| **Tier-1 (WASI)** | Stub returning `TierUnsupported` | `docs/design/sandbox-tiers/04-hyperlight-tier.md` (related) |
| **Tier-2 (Hyperlight)** | Stub | `docs/design/sandbox-tiers/04-hyperlight-tier.md` |
| **Tier-3 (Firecracker)** | Stub | `docs/design/sandbox-tiers/03-firecracker-tier.md` |

If a customer or external doc claims "VibeCody runs every tool call in a hyperlight microVM," that claim is **wrong today**. Call it out and link them here.

---

## Readiness — `/health.sandbox`

The daemon's `/health` endpoint reports the active tier and per-OS capabilities honestly:

```bash
curl http://127.0.0.1:7878/health | jq '.sandbox'
```

```json
{
  "active_tier": "native",
  "tiers": {
    "native": {
      "available": true,
      "network_isolation": true,
      "filesystem_isolation": false,
      "landlock_active": false,
      "seccomp_active": false,
      "bwrap_binary_available": false
    },
    "wasi":        { "available": false, "status": "stub" },
    "hyperlight":  { "available": false, "status": "stub" },
    "firecracker": { "available": false, "status": "stub" }
  },
  "egress_broker": { "available": false, "status": "designed-not-wired" },
  "deferred": [
    "landlock-rules-on-linux",
    "seccomp-bpf-filter",
    "macos-seatbelt-fs-profile",
    "egress-broker-wiring",
    "wasi/hyperlight/firecracker-tiers"
  ]
}
```

`features.sandbox` is the short-form availability summary feature gates read:

```json
{ "available": true, "transport": "in-process", "tier": "native" }
```

The `tiers.native.bwrap_binary_available` field is the runtime probe — `true` only when `bwrap --version` exits 0 on the host. Linux users without bwrap installed get network isolation only.

---

## Enabling stronger isolation per-tool

Every tool the agent calls goes through `ToolExecutor` (in `vibecli/vibecli-cli/src/tool_executor.rs`). Two flags control isolation:

```rust
ToolExecutor::new()
    .with_network_disabled(true)   // adds sandbox-exec / unshare wrapper
    .with_sandbox(true)            // routes through CommandExecutor::execute_sandboxed
```

Network isolation is **off** by default for shell tools today (we found early users hit it as a footgun); it's on by default for the diffcomplete file write path. Per-tool config is being unified in slice N1.4.

---

## Installing bwrap (Linux)

Filesystem isolation on Linux requires `bwrap` on PATH:

```bash
# Debian / Ubuntu
sudo apt install bubblewrap

# Fedora
sudo dnf install bubblewrap

# Arch
sudo pacman -S bubblewrap

# Verify
bwrap --version
```

Once installed, restart the daemon and `/health.sandbox.tiers.native.bwrap_binary_available` flips to `true`.

---

## Observability

Every sandbox spawn emits a structured `tracing` event under the `vibecody::sandbox` target:

```bash
RUST_LOG=vibecody::sandbox=info vibecli serve
```

Examples:

```
INFO vibecody::sandbox: sandbox.spawn: Linux netns
  tier=native os=linux backend=unshare network_isolation=true cmd_len=42

INFO vibecody::sandbox: sandbox.spawn: macOS Seatbelt no-network
  tier=native os=macos backend=sandbox-exec network_isolation=true cmd_len=42
```

Command content is **never** logged — only the length. No telemetry leaves your machine.

---

## Threat model

**What Tier-0 protects against (today):**
- Tools accidentally exfiltrating data over the network — the netns / Seatbelt / Job Object cuts off internet access.
- On Linux + Windows, tools accidentally reading or writing outside the workspace (when bwrap is present / on Windows).

**What Tier-0 does NOT protect against (today):**
- A determined attacker with code execution inside a tool. Without seccomp / Landlock, a malicious binary can still call arbitrary syscalls within the namespace.
- Side-channel attacks (CPU caches, microarchitectural).
- macOS file-system access — full home directory is reachable.
- Privilege escalation if the daemon itself is compromised.

For higher-assurance use cases (regulated environments, untrusted tool ecosystems, public CI), wait for Tier-1 (WASI) or Tier-2 (Hyperlight) — or run VibeCody inside a dedicated VM today.

---

## Troubleshooting

### "bwrap: setting up uid map: Permission denied"

bwrap needs unprivileged user namespaces enabled. Most modern distros enable this by default; some hardened ones (RHEL, Alpine on minimal kernels) don't.

```bash
# Check
sysctl kernel.unprivileged_userns_clone
# Should report `kernel.unprivileged_userns_clone = 1`. If 0:
sudo sysctl -w kernel.unprivileged_userns_clone=1
echo "kernel.unprivileged_userns_clone=1" | sudo tee /etc/sysctl.d/99-vibecody.conf
```

### "sandbox-exec: failed to load profile" (macOS)

Almost always a malformed Seatbelt profile. The shipping path uses `-n no-network`, the system-bundled profile, so this should not occur. If you see it: file an issue with the failing command + macOS version.

### Network isolation isn't blocking outbound DNS on Linux

`unshare --net` creates an empty namespace — no `lo`, no DNS resolver. Tools that hardcode `127.0.0.1` still work; tools that resolve `api.example.com` fail with NXDOMAIN. That's the correct behavior. If you need DNS-only access, file an issue — the egress broker design (deferred) covers this.

### macOS Seatbelt allows file reads I expected to block

Correct — the shipping `-n no-network` profile only blocks network access. Filesystem-restrictive profiles need a custom `.sb` file (deferred to slice N1.4). For now, use a dedicated macOS user account.

---

## Cross-client scope

Sandbox enforcement happens **inside the daemon**. Every client that runs a tool through the daemon (`POST /tools/run` and friends) inherits the protection automatically — there is no per-client sandbox to enable. The daemon is the only process with the capability to spawn a tool subprocess.

| Client | Sandbox visibility |
|---|---|
| **VibeUI / VibeApp** | Tools run via daemon — automatic Tier-0 |
| **VibeCLI** | Same — automatic Tier-0 |
| **VibeMobile / VibeWatch** | No tool execution — n/a |
| **IDE plugins** | Tools run via daemon — automatic Tier-0 |
| **Agent SDK** | Tools run via daemon — automatic Tier-0 |

If a client runs tools out-of-band (without the daemon), it bypasses the sandbox. That's a misconfiguration; the daemon-mediated path is the only supported one.

---

## Related

- **Design docs:** [`docs/design/sandbox-tiers/`](https://github.com/TuringWorks/vibecody/tree/main/docs/design/sandbox-tiers/) — README + per-tier specs (native / egress-broker / firecracker / hyperlight).
- **Source:** `vibecli/vibecli-cli/src/sandbox_bwrap.rs` (BwrapProfile builder, currently dead code; wired in slice N1.4) · `vibecli/vibecli-cli/src/tool_executor.rs` (active spawn path).
- **Settings:** [`/settings/`](../settings/) — no per-user sandbox toggles today; per-tool gating in slice N1.4.
