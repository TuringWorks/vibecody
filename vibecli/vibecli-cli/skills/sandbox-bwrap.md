# sandbox-bwrap

Linux bwrap (bubblewrap) sandbox profile builder. Generates the argv list for a `bwrap` invocation from a structured Rust policy. Pure logic — no actual syscalls — fully testable on any OS.

## Usage

```
/sandbox-bwrap [--minimal] [--with-network] [--ro src dst] [--rw src dst] [--tmpfs dst]
```

## Quick start

```rust
use vibecli_cli::sandbox_bwrap::BwrapProfile;

let args = BwrapProfile::minimal()
    .add_ro("/usr", "/usr")
    .add_ro("/lib", "/lib")
    .add_rw("/workspace", "/workspace")
    .to_args();

// Execute: std::process::Command::new("bwrap").args(&args).arg("--").arg("myapp").spawn()?;
```

## Profiles

### `BwrapProfile::minimal()`

| Mount | Type |
|---|---|
| `/proc` | proc |
| `/dev` | dev |
| `/tmp` | tmpfs |

Unshare flags: `--unshare-net`, `--unshare-pid`, `--unshare-ipc`  
Extra: `--die-with-parent`

### Builder methods

| Method | Effect |
|---|---|
| `.with_network()` | Remove `--unshare-net` (allow outbound network) |
| `.add_ro(src, dst)` | Add `--ro-bind src dst` |
| `.add_rw(src, dst)` | Add `--bind src dst` |
| `.add_tmpfs(dst)` | Add `--tmpfs dst` |

## Validation

```rust
let result = profile.validate();
// Err(BwrapValidationError { message: "Duplicate mount destination: '/usr'" })
```

Validation detects duplicate mount destinations and returns an error.

## API

```rust
pub struct BwrapProfile {
    pub mounts: Vec<MountSpec>,
    pub unshare: Vec<UnshareFlag>,
    pub die_with_parent: bool,
    pub new_session: bool,
}

impl BwrapProfile {
    pub fn new() -> Self
    pub fn minimal() -> Self
    pub fn with_network(self) -> Self
    pub fn add_ro(self, src: impl Into<String>, dst: impl Into<String>) -> Self
    pub fn add_rw(self, src: impl Into<String>, dst: impl Into<String>) -> Self
    pub fn add_tmpfs(self, dst: impl Into<String>) -> Self
    pub fn unshares_network(&self) -> bool
    pub fn unshares_pid(&self) -> bool
    pub fn ro_count(&self) -> usize
    pub fn rw_count(&self) -> usize
    pub fn mount_count(&self) -> usize
    pub fn to_args(&self) -> Vec<String>
    pub fn validate(&self) -> Result<(), BwrapValidationError>
}
```

## Unshare flags

| Variant | Arg |
|---|---|
| `Net` | `--unshare-net` |
| `Pid` | `--unshare-pid` |
| `Ipc` | `--unshare-ipc` |
| `Uts` | `--unshare-uts` |
| `User` | `--unshare-user` |
| `Cgroup` | `--unshare-cgroup` |

## Mount types

| Variant | Args |
|---|---|
| `RoBind { src, dst }` | `--ro-bind src dst` |
| `RwBind { src, dst }` | `--bind src dst` |
| `DevBind { src, dst }` | `--dev-bind src dst` |
| `Proc { dst }` | `--proc dst` |
| `Dev { dst }` | `--dev dst` |
| `Tmpfs { dst }` | `--tmpfs dst` |
