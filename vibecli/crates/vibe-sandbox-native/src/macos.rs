//! macOS Tier-0 backend: structured `.sb` profile + `sandbox-exec`.
//!
//! Profile shape follows `docs/design/sandbox-tiers/01-native-tier.md`.
//! `.sb` files are TinyScheme; we build them via a typed `SbProfile` so the
//! generator stays deterministic and unit-testable across platforms.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};

use vibe_sandbox::{
    BindMode, EnvPolicy, NetPolicy, ResourceLimits, Result, Sandbox, SandboxError, SandboxTier,
};

#[derive(Debug, Default, Clone)]
pub struct SbProfile {
    rw_subpaths: Vec<PathBuf>,
    ro_subpaths: Vec<PathBuf>,
    deny_network: bool,
    allow_network: bool,
    broker_socket: Option<PathBuf>,
    /// Loopback (`127.0.0.1`) TCP ports the sandbox is allowed to reach.
    /// Used to expose the IMDS faker port without unblocking general
    /// network access. Each entry becomes one
    /// `(allow network-outbound (remote tcp "localhost:PORT"))` rule.
    loopback_ports: Vec<u16>,
}

impl SbProfile {
    pub fn new() -> Self {
        SbProfile {
            deny_network: true,
            ..Default::default()
        }
    }

    pub fn allow_rw_subpath(&mut self, host: &Path) -> Result<()> {
        validate_subpath(host)?;
        self.rw_subpaths.push(host.to_owned());
        Ok(())
    }

    pub fn allow_ro_subpath(&mut self, host: &Path) -> Result<()> {
        validate_subpath(host)?;
        self.ro_subpaths.push(host.to_owned());
        Ok(())
    }

    pub fn allow_outbound_socket(&mut self, sock: &Path) {
        self.broker_socket = Some(sock.to_owned());
    }

    /// Permit outbound TCP to `127.0.0.1:port`. Used so the daemon's
    /// loopback IMDS faker (and any other policy-bound local helper)
    /// can be reached from the sandbox.
    pub fn allow_loopback_tcp(&mut self, port: u16) {
        self.loopback_ports.push(port);
    }

    pub fn deny_all_network(&mut self) {
        self.deny_network = true;
        self.allow_network = false;
    }

    pub fn allow_all_network(&mut self) {
        self.allow_network = true;
        self.deny_network = false;
    }

    pub fn render(&self) -> String {
        let mut out = String::with_capacity(2048);
        out.push_str("(version 1)\n");
        out.push_str("(deny default)\n");
        out.push_str("(allow process-fork)\n");
        out.push_str("(allow process-exec*)\n");
        out.push_str("(allow signal (target same-sandbox))\n");
        out.push_str("(allow sysctl-read)\n");
        out.push_str("(allow mach-lookup\n");
        out.push_str("  (global-name \"com.apple.system.notification_center\")\n");
        out.push_str("  (global-name \"com.apple.SystemConfiguration.configd\")\n");
        out.push_str("  (global-name \"com.apple.system.logger\")\n");
        out.push_str("  (global-name \"com.apple.system.opendirectoryd.api\"))\n");
        out.push_str("(allow file-read*\n");
        out.push_str("  (literal \"/\")\n");
        out.push_str("  (subpath \"/usr\")\n");
        out.push_str("  (subpath \"/System\")\n");
        out.push_str("  (subpath \"/Library\")\n");
        out.push_str("  (subpath \"/bin\")\n");
        out.push_str("  (subpath \"/sbin\")\n");
        out.push_str("  (subpath \"/private/var/db\")\n");
        out.push_str("  (subpath \"/private/etc\")\n");
        out.push_str("  (subpath \"/private/var/folders\")\n");
        out.push_str("  (subpath \"/private/var/select\")\n");
        out.push_str("  (subpath \"/dev\")\n");
        out.push_str("  (literal \"/dev/null\") (literal \"/dev/random\") (literal \"/dev/urandom\")\n");
        out.push_str("  (literal \"/dev/tty\") (literal \"/dev/dtracehelper\")\n");
        out.push_str("  (literal \"/private/var\") (literal \"/private/tmp\")\n");
        out.push_str("  (literal \"/var\") (literal \"/tmp\") (literal \"/etc\"))\n");
        // Allow reading process info for the shell itself
        out.push_str("(allow process-info* (target self))\n");
        out.push_str("(allow file-read-metadata)\n");

        for p in &self.rw_subpaths {
            out.push_str(&format!(
                "(allow file-read* file-write* (subpath {}))\n",
                quote_scheme(p)
            ));
        }
        for p in &self.ro_subpaths {
            out.push_str(&format!(
                "(allow file-read* (subpath {}))\n",
                quote_scheme(p)
            ));
        }
        if let Some(sock) = &self.broker_socket {
            out.push_str(&format!(
                "(allow network-outbound (literal {}))\n",
                quote_scheme(sock)
            ));
        }
        for port in &self.loopback_ports {
            // Allow outbound TCP to localhost:PORT only.
            out.push_str(&format!(
                "(allow network-outbound (remote tcp \"localhost:{port}\"))\n"
            ));
            // Connecting clients also need DNS-style sysctl + AF_INET socket
            // creation; granted broadly via the next rule scoped to TCP.
        }
        if !self.loopback_ports.is_empty() {
            // sandbox-exec needs (allow system-socket) for the AF_INET
            // socket() call itself; without this rule, the connect() to
            // localhost is denied at socket creation time.
            out.push_str("(allow system-socket)\n");
        }
        if self.deny_network {
            out.push_str("(deny network*)\n");
        }
        if self.allow_network {
            out.push_str("(allow network*)\n");
        }
        out
    }
}

fn validate_subpath(p: &Path) -> Result<()> {
    if p.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(SandboxError::Setup(format!(
            "path traversal not allowed in sandbox subpath: {}",
            p.display()
        )));
    }
    if !p.is_absolute() {
        return Err(SandboxError::Setup(format!(
            "sandbox subpath must be absolute: {}",
            p.display()
        )));
    }
    Ok(())
}

fn quote_scheme(p: &Path) -> String {
    let s = p.to_string_lossy();
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

pub struct MacosSandbox {
    profile: SbProfile,
    _env: EnvPolicy,
    _limits: ResourceLimits,
    /// Extra (key, value) pairs appended verbatim to the spawned process
    /// env. Daemon callers populate this with broker-handoff variables
    /// like `AWS_EC2_METADATA_SERVICE_ENDPOINT`, `HTTPS_PROXY`,
    /// `SSL_CERT_FILE`. The values are not secrets — secrets stay in the
    /// SecretStore, never in the sandbox env.
    extra_env: Vec<(String, String)>,
}

impl MacosSandbox {
    pub fn new() -> Result<Self> {
        Ok(MacosSandbox {
            profile: SbProfile::new(),
            _env: EnvPolicy::default(),
            _limits: ResourceLimits::default(),
            extra_env: Vec::new(),
        })
    }

    /// Add (or overwrite) an env var the sandboxed process will see.
    pub fn set_env(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let k = key.into();
        let v = value.into();
        if let Some(slot) = self.extra_env.iter_mut().find(|(ek, _)| ek == &k) {
            slot.1 = v;
        } else {
            self.extra_env.push((k, v));
        }
    }

    /// Permit outbound TCP to `127.0.0.1:port` from inside the sandbox.
    /// Used to expose broker helpers like the IMDS faker.
    pub fn allow_loopback_tcp(&mut self, port: u16) {
        self.profile.allow_loopback_tcp(port);
    }

    /// Diagnostic helper: returns the rendered profile string. Used by
    /// integration tests when reporting spawn failures.
    pub fn rendered_profile(&self) -> String {
        self.profile.render()
    }

    /// Run a command inside the sandbox and capture stdout/stderr/exit.
    /// Used by integration tests; production callers go through `spawn`.
    pub fn run_capture(&self, cmd: &str, args: &[&str]) -> std::io::Result<Output> {
        let profile = self.profile.render();
        let mut c = Command::new("sandbox-exec");
        c.arg("-p").arg(&profile).arg(cmd).args(args);
        c.current_dir(self.default_cwd());
        for (k, v) in &self.extra_env {
            c.env(k, v);
        }
        c.stdout(Stdio::piped()).stderr(Stdio::piped());
        c.output()
    }

    /// Pick a cwd that the sandboxed process can read. Defaults to the first
    /// rw-bound path; falls back to `/tmp`. Sandbox enforcement covers the
    /// real boundary; this is just so `getcwd` and relative paths behave
    /// predictably from the moment the child starts.
    fn default_cwd(&self) -> PathBuf {
        if let Some(p) = self.profile.rw_subpaths.first() {
            return p.clone();
        }
        PathBuf::from("/tmp")
    }
}

impl Sandbox for MacosSandbox {
    fn bind_rw(&mut self, host: &Path, _guest: &Path) -> Result<()> {
        let canonical = canonicalize_for_macos(host)?;
        self.profile.allow_rw_subpath(&canonical)
    }

    fn bind_ro(&mut self, host: &Path, _guest: &Path) -> Result<()> {
        let canonical = canonicalize_for_macos(host)?;
        self.profile.allow_ro_subpath(&canonical)
    }

    fn env(&mut self, policy: EnvPolicy) {
        self._env = policy;
    }

    fn limits(&mut self, limits: ResourceLimits) {
        self._limits = limits;
    }

    fn network(&mut self, policy: NetPolicy) {
        match &policy {
            NetPolicy::None | NetPolicy::Brokered { .. } => self.profile.deny_all_network(),
            NetPolicy::Direct => self.profile.allow_all_network(),
        }
        if let NetPolicy::Brokered { socket, .. } = policy {
            self.profile.allow_outbound_socket(&socket);
        }
    }

    fn spawn(&self, cmd: &OsStr, args: &[&OsStr]) -> Result<Child> {
        let profile = self.profile.render();
        let mut c = Command::new("sandbox-exec");
        c.arg("-p").arg(&profile).arg(cmd).args(args);
        c.current_dir(self.default_cwd());
        for (k, v) in &self.extra_env {
            c.env(k, v);
        }
        c.stdout(Stdio::piped()).stderr(Stdio::piped());
        c.spawn().map_err(SandboxError::Io)
    }

    fn tier(&self) -> SandboxTier {
        SandboxTier::Native
    }

    fn shutdown(self: Box<Self>) -> Result<()> {
        Ok(())
    }
}

fn canonicalize_for_macos(p: &Path) -> Result<PathBuf> {
    std::fs::canonicalize(p).map_err(|e| {
        SandboxError::Setup(format!("could not canonicalize {}: {}", p.display(), e))
    })
}

/// Helper used by `bind_with_mode` callers in higher tiers. Kept here so the
/// type stays paired with the profile builder.
pub fn bind_with_mode(profile: &mut SbProfile, host: &Path, mode: BindMode) -> Result<()> {
    match mode {
        BindMode::Rw => profile.allow_rw_subpath(host),
        BindMode::Ro => profile.allow_ro_subpath(host),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_starts_with_version_and_deny_default() {
        let r = SbProfile::new().render();
        assert!(r.starts_with("(version 1)\n"));
        assert!(r.contains("(deny default)"));
        assert!(r.contains("(deny network*)"));
    }

    #[test]
    fn validate_subpath_rejects_traversal() {
        assert!(validate_subpath(Path::new("/tmp/../etc")).is_err());
    }

    #[test]
    fn validate_subpath_rejects_relative() {
        assert!(validate_subpath(Path::new("relative/dir")).is_err());
    }

    #[test]
    fn validate_subpath_accepts_absolute() {
        assert!(validate_subpath(Path::new("/tmp")).is_ok());
    }

    #[test]
    fn rw_subpath_renders() {
        let mut p = SbProfile::new();
        p.allow_rw_subpath(Path::new("/tmp/work")).unwrap();
        let r = p.render();
        assert!(r.contains("file-read* file-write* (subpath \"/tmp/work\")"));
    }

    #[test]
    fn brokered_network_grants_socket_only() {
        let mut p = SbProfile::new();
        p.allow_outbound_socket(Path::new("/private/var/run/vibe-broker.sock"));
        let r = p.render();
        assert!(r.contains("(allow network-outbound (literal \"/private/var/run/vibe-broker.sock\"))"));
        assert!(r.contains("(deny network*)"));
    }
}
