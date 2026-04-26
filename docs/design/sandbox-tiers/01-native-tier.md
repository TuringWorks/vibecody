# 01 — Tier-0 Native Sandbox

**Scope:** the default sandbox tier — kernel-enforced isolation on Linux, macOS, Windows, with KB-scale overhead and millisecond cold-start.
**Parent:** [`README.md`](./README.md)
**Status:** Draft · 2026-04-26

---

## What's there today

- `vibecli/vibecli-cli/src/sandbox_bwrap.rs` — `BwrapProfile` config builder with `unshare-net/pid/ipc/uts/user/cgroup`, `--ro-bind` / `--bind` / `--dev-bind` / `--proc` / `--tmpfs`, `--die-with-parent`, `--new-session`. **`#[allow(dead_code)]` in `main.rs:2355` — never invoked.** The builder is good; nobody calls it.
- `vibecli/vibecli-cli/src/sandbox_windows.rs` — `WindowsSandboxConfig` with allow/deny path policy. **No kernel enforcement** — `check_path()` is advisory.
- `vibecli/vibecli-cli/src/tool_executor.rs:371-373` — wraps shell with `sandbox-exec -n no-network sh -c '...'` on macOS / `unshare --net` on Linux. **Network only, no FS isolation.** This is the only enforced sandbox in the daemon today.
- `vibecli/vibecli-cli/src/main.rs:15065-15076` — `vibecli doctor` checks for `sandbox-exec` (macOS) and `bwrap` (Linux) presence; nothing else uses them.
- `config.safety.sandbox: bool` (`config.rs:1268`) — TOML toggle that *would* invoke bwrap/sandbox-exec, but the wrap is partial.

So the foundation is half-built and untouched on Windows.

## Goals

1. One `Sandbox` impl per platform behind the trait in `vibe-sandbox`.
2. **Real kernel enforcement** on every platform — not policy-as-code.
3. Folder bind: `bind_rw(host_dir, guest_dir)` does the right thing on every OS.
4. Network: always via the broker (`02-egress-broker.md`), never raw.
5. Defense-in-depth: combine namespace + filesystem + syscall layers where the OS gives them. Don't pick "either bwrap or seccomp" when both are available.
6. Cold-start < 50 ms on Linux / macOS, < 80 ms on Windows; per-instance memory < 200 KB.

## Non-goals

- Replacing `DockerRuntime`. It stays for "ship me a container with this image."
- Implementing custom seccomp policies per skill. v1 ships one curated allow-list; per-skill overrides are a follow-on.
- macOS Seatbelt SPI (private `Sandbox.framework` API). The public `sandbox-exec` path is enough; using SPI requires entitlements and shipping signed.

## Tier-0 Linux — bwrap + Landlock + seccomp

Three layers, composed:

1. **bwrap** sets up namespaces (mount/net/pid/ipc/uts/user/cgroup), bind-mounts the user's folder, and execs the target. Uses the existing `BwrapProfile` builder — finally invoked.
2. **Landlock** (kernel ≥ 5.13) layers a second-stage FS-access ruleset *inside* the new namespace. Even if a future bug in the bwrap setup leaks an FS path, Landlock denies. Uses the `landlock-rs` crate.
3. **seccomp-bpf** filters syscalls to a curated allow-list (same shape AWS Firecracker's jailer uses; in fact uses the same `seccompiler` crate they use). Blocks rare/dangerous calls (`ptrace`, `mount`, `umount2`, `pivot_root`, `unshare`, `clone3 with new namespaces`, `bpf`, `kexec_load`, `init_module`, `delete_module`, `keyctl`, `add_key`, `request_key`, `query_module`, `_sysctl`, `personality`, `iopl`, `ioperm`, `swapon`, `swapoff`, `reboot`, `acct`, `mount_setattr`, etc.).

### Implementation sketch

```rust
// vibe-sandbox-native/src/linux.rs
pub struct LinuxSandbox {
    profile: BwrapProfile,            // imported from existing sandbox_bwrap
    landlock: Option<LandlockRuleset>,
    seccomp: SeccompFilter,
    broker_socket: PathBuf,
    limits: ResourceLimits,
}

impl Sandbox for LinuxSandbox {
    fn bind_rw(&mut self, host: &Path, guest: &Path) -> Result<()> {
        self.profile.bind(host, guest);                // bwrap --bind
        if let Some(ll) = self.landlock.as_mut() {
            ll.add_path_beneath(host, AccessFs::ALL_FILE | AccessFs::ALL_DIR_RW);
        }
        Ok(())
    }
    fn bind_ro(&mut self, host: &Path, guest: &Path) -> Result<()> {
        self.profile.ro_bind(host, guest);             // bwrap --ro-bind
        if let Some(ll) = self.landlock.as_mut() {
            ll.add_path_beneath(host, AccessFs::ALL_FILE_RO | AccessFs::ALL_DIR_RO);
        }
        Ok(())
    }
    fn network(&mut self, policy: NetPolicy) {
        match policy {
            NetPolicy::None | NetPolicy::Brokered { .. } => {
                self.profile.unshare_net();
            }
            NetPolicy::Direct => { /* keep host net ns; emit warning */ }
        }
        if let NetPolicy::Brokered { socket, .. } = policy {
            self.profile.bind(&socket, Path::new("/run/vibe-broker.sock"));
        }
    }
    fn spawn(&self, cmd: &OsStr, args: &[OsStr]) -> Result<Child> {
        let argv = self.profile.to_args();             // existing builder
        let mut c = Command::new("bwrap");
        c.args(&argv).arg("--").arg(cmd).args(args);
        // Apply seccomp + landlock from a small entry shim that runs inside bwrap
        // (we ship the shim binary; it loads seccomp/landlock then execve's $@)
        prepend_entry_shim(&mut c, &self.seccomp, &self.landlock, &self.limits);
        c.spawn().map_err(Into::into)
    }
    fn tier(&self) -> SandboxTier { SandboxTier::Native }
    fn shutdown(self: Box<Self>) -> Result<()> { Ok(()) }   // bwrap --die-with-parent handles teardown
}
```

The "entry shim" is a small (~200 line) helper binary that runs as the first process inside the bwrap'd namespace. It applies Landlock and seccomp (which must be applied *after* you're inside the namespace and *before* you `execve` the target), then `execve`s the user's command. This is exactly the pattern Bazel and Flatpak use.

### Resource limits

- CPU / memory / pids: cgroup v2 (set via the bwrap parent before the unshare, or via the entry shim writing to the cgroup it joins)
- Wall clock: a tokio task that `kill`s the bwrap PID after the deadline
- Open files: `setrlimit(RLIMIT_NOFILE, …)` in the shim

### Dependencies (Linux-only crates)

- `landlock` — MIT/Apache-2 (kernel ≥ 5.13; downgrades cleanly on older kernels)
- `seccompiler` — Apache-2 (the AWS-published one — same as Firecracker)
- `nix` — MIT (already in the workspace) for cgroup writes / `setrlimit`
- bwrap binary as a runtime requirement (already detected by `vibecli doctor`)

## Tier-0 macOS — `sandbox-exec` + structured `.sb` profile

The honest reality: Apple deprecated `sandbox-exec` in 10.8 (2012). Fourteen years later, Chromium, WebKit, Xcode, and Apple's own apps still use it on macOS 15. The "deprecation" has been theoretical the entire time. Using it is the consensus position; using the *private* `Sandbox.framework` SPI requires entitlements and a notarized binary, which doesn't fit a CLI shipping outside the App Store.

### The `.sb` (Sandbox Profile Language) shape

`.sb` files are TinyScheme. The minimal profile we generate per sandbox:

```scheme
(version 1)
(deny default)

;; allow basic process operations
(allow process-fork)
(allow process-exec (literal "/bin/bash") (literal "/bin/sh") (literal "/usr/bin/zsh"))
(allow signal (target same-sandbox))

;; mach lookups required by bash + libc
(allow mach-lookup
  (global-name "com.apple.system.notification_center")
  (global-name "com.apple.SystemConfiguration.configd")
  (global-name "com.apple.system.logger")
  (global-name "com.apple.system.opendirectoryd.api"))

;; system-level reads required to spawn anything
(allow file-read*
  (subpath "/usr")
  (subpath "/System")
  (subpath "/private/var/db/timezone")
  (literal "/dev/null") (literal "/dev/random") (literal "/dev/urandom") (literal "/dev/tty"))

;; the user's bound folders — generated per call
(allow file-read* file-write*
  (subpath "/Users/me/myrepo"))    ;; from bind_rw

(allow file-read*
  (subpath "/Users/me/readonly-toolchain"))  ;; from bind_ro

;; broker socket (the only way out)
(allow network-outbound (literal "/private/var/run/vibe-broker.sock"))

;; deny everything else, loud
(deny network*)
(deny file-write* (subpath "/Users") (subpath "/Library"))
```

The profile is generated as a string per call (small templating; no external file) and passed via `sandbox-exec -p`. Generation is deterministic and < 1 ms.

### Implementation sketch

```rust
// vibe-sandbox-native/src/macos.rs
pub struct MacosSandbox {
    profile: SbProfile,            // strongly typed builder; renders to the Scheme above
    broker_socket: PathBuf,
    limits: ResourceLimits,
}

impl Sandbox for MacosSandbox {
    fn bind_rw(&mut self, host: &Path, _guest: &Path) -> Result<()> {
        // macOS sandbox is path-based, not namespace-based; we don't remap paths
        self.profile.allow_rw_subpath(host);
        Ok(())
    }
    fn bind_ro(&mut self, host: &Path, _guest: &Path) -> Result<()> {
        self.profile.allow_ro_subpath(host);
        Ok(())
    }
    fn network(&mut self, policy: NetPolicy) {
        match policy {
            NetPolicy::None | NetPolicy::Brokered { .. } => self.profile.deny_all_network(),
            NetPolicy::Direct => self.profile.allow_all_network(),
        }
        if let NetPolicy::Brokered { socket, .. } = policy {
            self.profile.allow_outbound_socket(&socket);
        }
    }
    fn spawn(&self, cmd: &OsStr, args: &[OsStr]) -> Result<Child> {
        let p = self.profile.render();
        let mut c = Command::new("sandbox-exec");
        c.arg("-p").arg(&p).arg(cmd).args(args);
        apply_rlimits(&mut c, &self.limits);
        c.spawn().map_err(Into::into)
    }
    fn tier(&self) -> SandboxTier { SandboxTier::Native }
    fn shutdown(self: Box<Self>) -> Result<()> { Ok(()) }
}
```

### Caveat: macOS sandbox is path-based, not namespace-based

There is no "remap host path X to guest path Y" on macOS. `bind_rw(host, guest)` only honors the host portion — the sandboxed process sees the same path the host has. Programs that hard-code paths like `/work` will break on macOS unless we symlink. Two options:

1. **Symlink in the host:** at sandbox setup, `/tmp/vibe-sb-{ulid}/work` → host folder, then pass that as the bound dir. Sandbox profile allows the symlinked subpath. Works but litters `/tmp`.
2. **Document the platform difference:** on Linux/Windows the sandbox sees `/work`; on macOS it sees the original host path. Tools that need predictability use environment variable `$VIBE_SANDBOX_ROOT`.

v1 picks **option 2** (document, expose env). Simpler, no `/tmp` litter, no symlink-resolution surprises.

### Resource limits on macOS

- CPU / memory: `setrlimit(RLIMIT_CPU, RLIMIT_AS)` in the spawned process
- Wall clock: same as Linux — kill timer
- macOS doesn't expose cgroup-level CPU quotas to userspace; for stronger limits, fall back to `taskpolicy` or accept rlimit-only

### Dependencies (macOS-only)

- `sandbox-exec` binary — present in every macOS install
- No external Rust crates beyond `nix` for `setrlimit`

## Tier-0 Windows — AppContainer + Restricted Token + Job Object

Three layers, similar to Linux's three-layer composition:

1. **AppContainer** — capability-based sandbox; the process runs under a unique SID and can only access objects that explicitly grant the AppContainer SID. Default: zero capabilities. Grant just `lpacInstrumentation` if needed for tracing.
2. **Restricted Token** — additionally restrict the process token by removing privileges and groups (e.g., remove SeDebugPrivilege, SeBackupPrivilege, etc.).
3. **Job Object** — wraps the process tree, sets memory caps, CPU rate limits, kills child processes on parent death (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`), prevents desktop / clipboard access (`JOB_OBJECT_UILIMIT_*`), restricts process creation count.

### File system access

AppContainer FS access is granted by setting an ACL on each folder the AppContainer should read/write. The `bind_rw` implementation:

```rust
// pseudocode
fn grant_appcontainer_access(path: &Path, sid: &Sid, mode: AccessMode) -> Result<()> {
    let acl = current_acl_of(path)?;
    let new_ace = build_explicit_ace(sid, mode);
    let new_acl = acl.with_added(new_ace);
    set_acl(path, new_acl)?;
    Ok(())
}
```

We append an ACE for the AppContainer SID, run the workload, and remove the ACE on shutdown. The host folder remains readable to the user; nothing else changes.

### Network on Windows

- Default: deny `internetClient`, `internetClientServer`, `privateNetworkClientServer`, `enterpriseAuthentication` capabilities. AppContainer has no inet by default if you don't grant these.
- Backstop: a Windows Filtering Platform (WFP) provider registered by the daemon explicitly blocks all outbound from this AppContainer's SID (defense in depth in case capabilities are misconfigured).
- Broker: a named pipe `\\.\pipe\vibe-broker-{ulid}` granted to the AppContainer SID. The pipe is the only IPC channel out.

### Implementation sketch

```rust
// vibe-sandbox-native/src/windows.rs
pub struct WindowsAppContainerSandbox {
    sid: Sid,                          // generated per sandbox
    profile_path: PathBuf,             // %LOCALAPPDATA%\Packages\<sid>\
    rw_paths: Vec<(PathBuf, PathBuf)>, // (host, guest)
    ro_paths: Vec<(PathBuf, PathBuf)>,
    job: JobObject,
    broker_pipe: PathBuf,
    limits: ResourceLimits,
}

impl Sandbox for WindowsAppContainerSandbox {
    fn bind_rw(&mut self, host: &Path, guest: &Path) -> Result<()> {
        grant_appcontainer_access(host, &self.sid, AccessMode::ReadWrite)?;
        self.rw_paths.push((host.to_owned(), guest.to_owned()));
        Ok(())
    }
    fn bind_ro(&mut self, host: &Path, guest: &Path) -> Result<()> {
        grant_appcontainer_access(host, &self.sid, AccessMode::Read)?;
        self.ro_paths.push((host.to_owned(), guest.to_owned()));
        Ok(())
    }
    fn network(&mut self, policy: NetPolicy) {
        // capabilities default to none; brokered = grant pipe access only
        if let NetPolicy::Brokered { socket, .. } = &policy {
            self.broker_pipe = socket.clone();
            grant_pipe_access(&self.broker_pipe, &self.sid)?;
        }
    }
    fn spawn(&self, cmd: &OsStr, args: &[OsStr]) -> Result<Child> {
        let proc = create_process_in_appcontainer(&self.sid, cmd, args, &self.limits)?;
        self.job.assign(&proc)?;
        Ok(proc.into())
    }
    fn shutdown(self: Box<Self>) -> Result<()> {
        // Kill all in job, revoke ACEs, delete profile
        self.job.terminate(0)?;
        for (path, _) in &self.rw_paths { revoke_appcontainer_access(path, &self.sid)?; }
        for (path, _) in &self.ro_paths { revoke_appcontainer_access(path, &self.sid)?; }
        delete_appcontainer_profile(&self.profile_path)?;
        Ok(())
    }
}
```

### Caveat: Windows AppContainer + bash

Native Windows doesn't ship `bash`. Three usable shells:

| Shell | Where it comes from | Sandbox-friendly? |
|---|---|---|
| `cmd.exe` | Native | ✅ |
| `pwsh.exe` (PowerShell 7) | Optional install | ✅ |
| `bash.exe` (Git Bash) | Bundled with Git for Windows | ✅ — runs as a normal process, AppContainer applies |
| WSL `bash` | Windows Subsystem for Linux | ❌ WSL bypasses AppContainer; treat as a separate Linux sandbox |

The daemon detects available shells and the user picks (config: `[sandbox.windows] shell = "git-bash"`). For users on WSL2 who want Linux-shell semantics, the daemon spawns into WSL and uses the **Linux** Tier-0 implementation inside WSL — that path "just works" because WSL2 *is* a Linux kernel.

### Dependencies (Windows-only crates)

- `windows` (formerly `windows-rs`) — MIT, Microsoft-published, the canonical Win32 binding
- `windows-acl` — MIT, for the ACE manipulation helpers
- No external runtime dependencies; everything is in the OS

## Common: spawn/exec semantics

All three implementations expose `spawn(cmd, args) -> Child`, where `Child` is a tokio-async wrapper with:

- `stdout` / `stderr` streamed (used by `tool_executor` to feed events)
- `wait()` returns exit code or signal
- `kill()` for cancellation (wired to ctrl-c and `wall_clock` timeout)
- `pid()` for telemetry only (not for cross-process IPC; that's what the broker is for)

## Audit events emitted by every Tier-0 impl

```jsonc
{ "event": "sandbox.spawn",  "tier": "native", "platform": "linux",   "cmd": "/bin/bash", "args_redacted": "...", "policy_id": "skill:foo", "ts": "..." }
{ "event": "sandbox.bind",   "tier": "native", "host_path": "...", "guest_path": "...", "mode": "rw" }
{ "event": "sandbox.exit",   "tier": "native", "exit_code": 0, "wall_ms": 1234, "cpu_ms": 890, "max_rss_kb": 12000 }
```

All three platform impls emit identical event shapes. Recap subsystem (`docs/design/recap-resume/02-job.md`) ingests these and renders them in job recaps.

## Failure modes

| Failure | Linux | macOS | Windows |
|---|---|---|---|
| Required tool missing | `bwrap` not on PATH → daemon errors at startup with install hint | `sandbox-exec` always present | always present |
| Kernel too old | Landlock <5.13 → fall back to bwrap+seccomp only, log warning | n/a | n/a |
| Capability not granted | Most kernels allow user namespaces unprivileged; if not, `vibecli doctor` flags it | sandbox-exec works as the user | AppContainer creation can fail if SID quota reached; emit error |
| Broker socket already exists | Reuse if owned by daemon UID; reject otherwise | same | named pipe is per-instance ULID; no collision |
| File ACE leaks on Windows crash | — | — | startup scan removes orphaned AppContainer SIDs older than 24h |

## Testing

- **Linux:** integration tests use a real bwrap; CI matrix covers Ubuntu 22.04 (Landlock 5.13), Ubuntu 24.04 (Landlock 6.x), Alpine (musl, no Landlock — fallback path).
- **macOS:** integration tests on macOS 13/14/15; profile-rendering unit tests are deterministic and run on every platform.
- **Windows:** integration tests on Windows 11 22H2, Server 2022; the `windows-rs` API is mockable for unit tests.

CI smoke test: `bind_rw("/tmp/sb-test")` + `spawn("bash", "-c", "echo hello > /work/out && cat /etc/passwd")` — first command succeeds with the right output, second fails with permission denied.

## Slicing plan

| Slice | What | Touches | Tests |
|---|---|---|---|
| **N0.1** | `vibe-sandbox` workspace crate scaffold + trait + types + `select()` + cfg-stubs | new crate | unit: stub error paths |
| **N1.1** | Linux: bwrap invocation via existing `BwrapProfile`; remove `#[allow(dead_code)]` | `vibe-sandbox-native/linux.rs`, `main.rs:2355` | integration: `bind_rw` + `bind_ro` + path traversal denial |
| **N1.2** | Linux: Landlock layer | `vibe-sandbox-native/linux.rs` | integration: even with bwrap bypass, Landlock denies |
| **N1.3** | Linux: seccomp filter + entry shim | `vibe-sandbox-native/linux.rs` + new shim binary | integration: blocked syscalls fail with EPERM |
| **N2.1** | macOS: `.sb` profile generator + sandbox-exec invocation | `vibe-sandbox-native/macos.rs` | integration: read outside bound path → denied |
| **N3.1** | Windows: AppContainer creation + ACE grant/revoke | `vibe-sandbox-native/windows.rs` | integration: read outside bound path → AccessDenied |
| **N3.2** | Windows: Job Object + Restricted Token + WFP block rule | `vibe-sandbox-native/windows.rs` | integration: kill-on-parent-death, network → blocked |
| **N4** | Wire into `tool_executor` + `agent_executor`; replace `with_no_network()` with `with_sandbox(policy)` | `vibecli-cli`, `vibeui/src-tauri` | end-to-end: vibecli + vibeui both run a real bash inside a real sandbox |
| **N5** | Tauri command surface for vibeui to set sandbox policy per chat tab | `vibeui/src-tauri/src/commands.rs` | RTL: settings panel toggles enforce on next spawn |

Each slice is `cargo test --workspace` green, `cargo build --release -p vibecli` green, and the `vibecli doctor` output reflects which layers are active on the host.

## Open questions

1. **Should we ship Landlock policy as opt-in or default-on?** Default-on is safer; the fallback path for old kernels handles it. Pick default-on.
2. **macOS path-remap** (the symlink-vs-document trade-off above). Recommended: document, expose `$VIBE_SANDBOX_ROOT`, ship a small skill helper that resolves it. Confirmed in v1; revisit if user reports break a workflow.
3. **WSL detection on Windows.** Should Tier-0 *prefer* the Linux impl when run inside WSL, or always use the Windows impl? Recommended: prefer Linux inside WSL — it's already a real Linux kernel and bwrap works.
4. **Per-skill seccomp overrides.** v1 ships one curated allow-list; per-skill overrides (some skills need `unshare` for nested sandboxes, etc.) are deferred to v2 with a careful threat-model review.
