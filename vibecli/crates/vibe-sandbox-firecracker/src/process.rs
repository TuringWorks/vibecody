//! Firecracker + jailer process spawn argv builder.
//!
//! Slice F2.2-B — composes the command line F2.2-C will hand to
//! `std::process::Command` on Linux. Two layers:
//!
//! * [`FirecrackerSpawn`] — direct `firecracker --api-sock …` invocation.
//!   Suitable for trusted single-tenant hosts (dev workstations) where
//!   the daemon is already running as root.
//!
//! * [`JailerWrap`] — wraps [`FirecrackerSpawn`] in `jailer`, the
//!   upstream chroot + cgroup + uid/gid drop helper. Required in
//!   production: it confines the firecracker process itself in case
//!   the VMM (not the guest) gets compromised.
//!
//! ## Why a separate slice
//!
//! Firecracker's command line is small but every flag has a specific
//! meaning + a documented default. Mis-passing `--id` (jailer
//! requires a unique alphanumeric VM ID) or omitting `--api-sock`
//! (boot fails silently) regresses runtime. Pinning the argv shape
//! here with tests means F2.2-C's actual `Command::spawn` is the
//! same code on Linux as a `dry-run` on macOS.

use std::ffi::OsString;
use std::path::PathBuf;

use thiserror::Error;

/// Errors from spawn-config construction.
#[derive(Debug, Error)]
pub enum SpawnError {
    #[error("vm_id must be non-empty alphanumeric/dash/underscore (got {0:?})")]
    InvalidVmId(String),

    #[error("vm_id exceeds 64 characters: {0}")]
    VmIdTooLong(String),

    #[error("log level not recognized: {0:?} (expected one of: Error, Warn, Info, Debug, Trace)")]
    InvalidLogLevel(String),
}

/// Firecracker process spawn config.
///
/// At least `api_socket_path` + `vm_id` are required; everything
/// else has a documented default. Use the builder pattern (`.with_*`)
/// to override.
#[derive(Debug, Clone)]
pub struct FirecrackerSpawn {
    firecracker_binary: PathBuf,
    api_socket_path: PathBuf,
    vm_id: String,
    log_path: Option<PathBuf>,
    log_level: LogLevel,
    /// Optional alternative to the REST API — pass a static JSON
    /// config at startup. Useful for snapshot-restore (F5) but not
    /// for the dynamic boot path F2.2 uses today.
    config_file: Option<PathBuf>,
    /// Extra args appended at the end (for forward-compatibility with
    /// firecracker flags this builder doesn't surface natively).
    extra_args: Vec<OsString>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "Error",
            LogLevel::Warn => "Warn",
            LogLevel::Info => "Info",
            LogLevel::Debug => "Debug",
            LogLevel::Trace => "Trace",
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = SpawnError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Error" => Ok(LogLevel::Error),
            "Warn" => Ok(LogLevel::Warn),
            "Info" => Ok(LogLevel::Info),
            "Debug" => Ok(LogLevel::Debug),
            "Trace" => Ok(LogLevel::Trace),
            other => Err(SpawnError::InvalidLogLevel(other.to_string())),
        }
    }
}

impl FirecrackerSpawn {
    pub fn new(
        firecracker_binary: impl Into<PathBuf>,
        api_socket_path: impl Into<PathBuf>,
        vm_id: impl Into<String>,
    ) -> Result<Self, SpawnError> {
        let vm_id = vm_id.into();
        validate_vm_id(&vm_id)?;
        Ok(Self {
            firecracker_binary: firecracker_binary.into(),
            api_socket_path: api_socket_path.into(),
            vm_id,
            log_path: None,
            log_level: LogLevel::Info,
            config_file: None,
            extra_args: Vec::new(),
        })
    }

    pub fn with_log_path(mut self, p: impl Into<PathBuf>) -> Self {
        self.log_path = Some(p.into());
        self
    }
    pub fn with_log_level(mut self, l: LogLevel) -> Self {
        self.log_level = l;
        self
    }
    pub fn with_config_file(mut self, p: impl Into<PathBuf>) -> Self {
        self.config_file = Some(p.into());
        self
    }
    pub fn with_extra_arg(mut self, a: impl Into<OsString>) -> Self {
        self.extra_args.push(a.into());
        self
    }

    pub fn api_socket_path(&self) -> &std::path::Path {
        &self.api_socket_path
    }
    pub fn vm_id(&self) -> &str {
        &self.vm_id
    }

    /// Build the firecracker argv (without the binary itself — the
    /// binary is `program`, argv0 is the program name by convention).
    pub fn argv(&self) -> Vec<OsString> {
        let mut a = Vec::<OsString>::new();
        a.push(OsString::from("--api-sock"));
        a.push(self.api_socket_path.clone().into_os_string());
        a.push(OsString::from("--id"));
        a.push(OsString::from(self.vm_id.clone()));
        if let Some(p) = &self.log_path {
            a.push(OsString::from("--log-path"));
            a.push(p.clone().into_os_string());
            // Firecracker requires --level only when --log-path is set.
            a.push(OsString::from("--level"));
            a.push(OsString::from(self.log_level.as_str()));
        }
        if let Some(p) = &self.config_file {
            a.push(OsString::from("--config-file"));
            a.push(p.clone().into_os_string());
        }
        a.extend(self.extra_args.iter().cloned());
        a
    }

    pub fn program(&self) -> &std::path::Path {
        &self.firecracker_binary
    }
}

/// Wrap a [`FirecrackerSpawn`] in `jailer`. The jailer chroots the
/// firecracker process into `<chroot_base>/<vm_id>/`, drops to the
/// given UID/GID, applies cgroups, and re-execs firecracker with
/// the same argv.
///
/// Required for any production deployment — even though Firecracker
/// itself is tiny, a VMM bug that lets the guest escape would still
/// be confined to the chroot.
#[derive(Debug, Clone)]
pub struct JailerWrap {
    jailer_binary: PathBuf,
    inner: FirecrackerSpawn,
    uid: u32,
    gid: u32,
    chroot_base_dir: PathBuf,
    /// Optional resource limits (forwarded to jailer's cgroup flags).
    /// `None` means no jailer-imposed limit; firecracker still
    /// honors its own per-VM machine_config memory limit.
    cgroups: Vec<JailerCgroup>,
}

/// One `--cgroup key=value` pair, e.g. `memory.max=512M` or
/// `cpu.weight=100`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JailerCgroup {
    pub key: String,
    pub value: String,
}

impl JailerWrap {
    pub fn new(
        jailer_binary: impl Into<PathBuf>,
        inner: FirecrackerSpawn,
        uid: u32,
        gid: u32,
        chroot_base_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            jailer_binary: jailer_binary.into(),
            inner,
            uid,
            gid,
            chroot_base_dir: chroot_base_dir.into(),
            cgroups: Vec::new(),
        }
    }

    pub fn with_cgroup(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.cgroups.push(JailerCgroup {
            key: key.into(),
            value: value.into(),
        });
        self
    }

    pub fn program(&self) -> &std::path::Path {
        &self.jailer_binary
    }

    /// Full jailer argv.
    pub fn argv(&self) -> Vec<OsString> {
        let mut a = Vec::<OsString>::new();
        a.push(OsString::from("--id"));
        a.push(OsString::from(self.inner.vm_id.clone()));
        a.push(OsString::from("--exec-file"));
        a.push(self.inner.firecracker_binary.clone().into_os_string());
        a.push(OsString::from("--uid"));
        a.push(OsString::from(self.uid.to_string()));
        a.push(OsString::from("--gid"));
        a.push(OsString::from(self.gid.to_string()));
        a.push(OsString::from("--chroot-base-dir"));
        a.push(self.chroot_base_dir.clone().into_os_string());
        for cg in &self.cgroups {
            a.push(OsString::from("--cgroup"));
            a.push(OsString::from(format!("{}={}", cg.key, cg.value)));
        }
        // After the jailer args, separator `--` and then the
        // firecracker argv. jailer execs firecracker with these,
        // skipping the --id since jailer already passes it.
        a.push(OsString::from("--"));
        // Inner firecracker argv — but skip the --id (jailer
        // provides it) and skip --api-sock (jailer rewrites the
        // path inside the chroot).
        for chunk in self.inner.argv().windows(2).step_by(2) {
            match chunk[0].to_string_lossy().as_ref() {
                "--id" | "--api-sock" => continue,
                _ => {
                    a.push(chunk[0].clone());
                    a.push(chunk[1].clone());
                }
            }
        }
        a
    }
}

fn validate_vm_id(id: &str) -> Result<(), SpawnError> {
    if id.is_empty() {
        return Err(SpawnError::InvalidVmId(id.to_string()));
    }
    if id.len() > 64 {
        return Err(SpawnError::VmIdTooLong(id.to_string()));
    }
    let ok = id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok {
        return Err(SpawnError::InvalidVmId(id.to_string()));
    }
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_spawn() -> FirecrackerSpawn {
        FirecrackerSpawn::new(
            "/usr/bin/firecracker",
            "/run/firecracker/api.sock",
            "vibe-vm-42",
        )
        .unwrap()
    }

    // ── vm_id validation ─────────────────────────────────────────────────

    #[test]
    fn empty_vm_id_rejected() {
        let e = FirecrackerSpawn::new("/usr/bin/firecracker", "/run/a.sock", "").unwrap_err();
        assert!(matches!(e, SpawnError::InvalidVmId(_)));
    }

    #[test]
    fn vm_id_with_slash_rejected() {
        let e = FirecrackerSpawn::new("/usr/bin/firecracker", "/run/a.sock", "bad/id")
            .unwrap_err();
        assert!(matches!(e, SpawnError::InvalidVmId(_)));
    }

    #[test]
    fn vm_id_with_dash_underscore_accepted() {
        let s =
            FirecrackerSpawn::new("/usr/bin/firecracker", "/run/a.sock", "vm-1_test").unwrap();
        assert_eq!(s.vm_id(), "vm-1_test");
    }

    #[test]
    fn vm_id_too_long_rejected() {
        let too_long = "a".repeat(65);
        let e = FirecrackerSpawn::new("/usr/bin/firecracker", "/run/a.sock", &too_long)
            .unwrap_err();
        assert!(matches!(e, SpawnError::VmIdTooLong(_)));
    }

    #[test]
    fn vm_id_max_64_accepted() {
        let exactly_64 = "a".repeat(64);
        let s =
            FirecrackerSpawn::new("/usr/bin/firecracker", "/run/a.sock", &exactly_64).unwrap();
        assert_eq!(s.vm_id().len(), 64);
    }

    // ── argv shape ───────────────────────────────────────────────────────

    fn argv_str(args: &[OsString]) -> String {
        args.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn argv_minimal_has_api_sock_and_id() {
        let s = ok_spawn();
        let argv = argv_str(&s.argv());
        assert!(argv.contains("--api-sock /run/firecracker/api.sock"));
        assert!(argv.contains("--id vibe-vm-42"));
        // No --log-path / --level / --config-file by default.
        assert!(!argv.contains("--log-path"));
        assert!(!argv.contains("--level"));
        assert!(!argv.contains("--config-file"));
    }

    #[test]
    fn argv_log_path_emits_level_too() {
        let s = ok_spawn()
            .with_log_path("/var/log/firecracker.log")
            .with_log_level(LogLevel::Debug);
        let argv = argv_str(&s.argv());
        assert!(argv.contains("--log-path /var/log/firecracker.log"));
        assert!(argv.contains("--level Debug"));
    }

    #[test]
    fn argv_log_level_omitted_when_no_log_path() {
        // Set log level but no log path → Firecracker rejects --level
        // without --log-path, so we must not emit it.
        let s = ok_spawn().with_log_level(LogLevel::Trace);
        let argv = argv_str(&s.argv());
        assert!(!argv.contains("--level"));
    }

    #[test]
    fn argv_config_file_emits_flag() {
        let s = ok_spawn().with_config_file("/etc/vibe/firecracker.json");
        let argv = argv_str(&s.argv());
        assert!(argv.contains("--config-file /etc/vibe/firecracker.json"));
    }

    #[test]
    fn argv_extra_args_appended_last() {
        let s = ok_spawn()
            .with_extra_arg("--no-seccomp")
            .with_extra_arg("--metrics-fifo");
        let argv = s.argv();
        let last_two: Vec<String> = argv.iter().rev().take(2).rev()
            .map(|o| o.to_string_lossy().into_owned()).collect();
        assert_eq!(last_two, vec!["--no-seccomp", "--metrics-fifo"]);
    }

    // ── LogLevel ─────────────────────────────────────────────────────────

    #[test]
    fn log_level_from_str_round_trip() {
        for l in [
            LogLevel::Error,
            LogLevel::Warn,
            LogLevel::Info,
            LogLevel::Debug,
            LogLevel::Trace,
        ] {
            let s = l.as_str();
            let parsed: LogLevel = s.parse().unwrap();
            assert_eq!(parsed, l);
        }
    }

    #[test]
    fn log_level_from_str_rejects_bogus() {
        let e: Result<LogLevel, _> = "yelling".parse();
        assert!(matches!(e, Err(SpawnError::InvalidLogLevel(_))));
    }

    // ── JailerWrap argv ──────────────────────────────────────────────────

    #[test]
    fn jailer_argv_has_required_flags() {
        let inner = ok_spawn();
        let jw = JailerWrap::new(
            "/usr/bin/jailer",
            inner,
            1234,
            5678,
            "/srv/jailer",
        );
        let argv_v = jw.argv();
        let argv = argv_str(&argv_v);
        assert!(argv.contains("--id vibe-vm-42"));
        assert!(argv.contains("--exec-file /usr/bin/firecracker"));
        assert!(argv.contains("--uid 1234"));
        assert!(argv.contains("--gid 5678"));
        assert!(argv.contains("--chroot-base-dir /srv/jailer"));
        // The `--` token must be present as a standalone arg, regardless
        // of whether anything follows it (minimal inner spawn produces
        // nothing after `--`, which is still a valid jailer invocation).
        assert!(
            argv_v.iter().any(|a| a == "--"),
            "expected standalone `--` separator in argv: {:?}",
            argv_v
        );
    }

    #[test]
    fn jailer_argv_strips_inner_id_and_api_sock_after_separator() {
        // jailer's `--` separator hands the remainder to firecracker.
        // jailer already passes --id; it also rewrites --api-sock to
        // a chroot-relative path. So our argv builder must NOT
        // re-emit those in the post-`--` portion.
        let inner = ok_spawn().with_log_path("/var/log/f.log");
        let jw = JailerWrap::new("/usr/bin/jailer", inner, 0, 0, "/srv/jailer");
        let argv = argv_str(&jw.argv());
        // Find the `--` separator and look at what follows.
        let after_sep = argv.split(" -- ").nth(1).unwrap();
        assert!(
            !after_sep.contains("--id"),
            "jailer already provides --id; post-separator must omit it"
        );
        assert!(
            !after_sep.contains("--api-sock"),
            "jailer rewrites --api-sock; post-separator must omit it"
        );
        // But --log-path SHOULD still be there.
        assert!(after_sep.contains("--log-path /var/log/f.log"));
        assert!(after_sep.contains("--level Info"));
    }

    #[test]
    fn jailer_with_cgroups_emits_them() {
        let inner = ok_spawn();
        let jw = JailerWrap::new("/usr/bin/jailer", inner, 0, 0, "/srv/jailer")
            .with_cgroup("memory.max", "512M")
            .with_cgroup("cpu.weight", "100");
        let argv = argv_str(&jw.argv());
        assert!(argv.contains("--cgroup memory.max=512M"));
        assert!(argv.contains("--cgroup cpu.weight=100"));
    }

    #[test]
    fn jailer_program_returns_binary_path() {
        let jw = JailerWrap::new("/usr/bin/jailer", ok_spawn(), 0, 0, "/srv/jailer");
        assert_eq!(jw.program(), std::path::Path::new("/usr/bin/jailer"));
    }
}
