/*!
 * sandbox_bwrap.rs — Linux bwrap (bubblewrap) sandbox profile builder.
 *
 * Generates the argv list for a `bwrap` invocation from a structured policy.
 * Pure Rust policy logic — no actual syscalls, fully testable on any OS.
 */

// ---------------------------------------------------------------------------
// UnshareFlag
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum UnshareFlag {
    Net,
    Pid,
    Ipc,
    Uts,
    User,
    Cgroup,
}

impl UnshareFlag {
    pub fn to_arg(&self) -> &'static str {
        match self {
            UnshareFlag::Net => "--unshare-net",
            UnshareFlag::Pid => "--unshare-pid",
            UnshareFlag::Ipc => "--unshare-ipc",
            UnshareFlag::Uts => "--unshare-uts",
            UnshareFlag::User => "--unshare-user",
            UnshareFlag::Cgroup => "--unshare-cgroup",
        }
    }
}

// ---------------------------------------------------------------------------
// MountSpec
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum MountSpec {
    RoBind { src: String, dst: String },
    RwBind { src: String, dst: String },
    DevBind { src: String, dst: String },
    Proc { dst: String },
    Dev { dst: String },
    Tmpfs { dst: String },
}

impl MountSpec {
    pub fn to_args(&self) -> Vec<String> {
        match self {
            MountSpec::RoBind { src, dst } => {
                vec!["--ro-bind".to_string(), src.clone(), dst.clone()]
            }
            MountSpec::RwBind { src, dst } => {
                vec!["--bind".to_string(), src.clone(), dst.clone()]
            }
            MountSpec::DevBind { src, dst } => {
                vec!["--dev-bind".to_string(), src.clone(), dst.clone()]
            }
            MountSpec::Proc { dst } => vec!["--proc".to_string(), dst.clone()],
            MountSpec::Dev { dst } => vec!["--dev".to_string(), dst.clone()],
            MountSpec::Tmpfs { dst } => vec!["--tmpfs".to_string(), dst.clone()],
        }
    }

    fn dst(&self) -> &str {
        match self {
            MountSpec::RoBind { dst, .. } => dst,
            MountSpec::RwBind { dst, .. } => dst,
            MountSpec::DevBind { dst, .. } => dst,
            MountSpec::Proc { dst } => dst,
            MountSpec::Dev { dst } => dst,
            MountSpec::Tmpfs { dst } => dst,
        }
    }

    fn is_ro_bind(&self) -> bool {
        matches!(self, MountSpec::RoBind { .. })
    }

    fn is_rw_bind(&self) -> bool {
        matches!(self, MountSpec::RwBind { .. })
    }
}

// ---------------------------------------------------------------------------
// BwrapValidationError
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct BwrapValidationError {
    pub message: String,
}

// ---------------------------------------------------------------------------
// BwrapProfile
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct BwrapProfile {
    pub mounts: Vec<MountSpec>,
    pub unshare: Vec<UnshareFlag>,
    pub die_with_parent: bool,
    pub new_session: bool,
}

impl BwrapProfile {
    /// Empty profile with no mounts, no unshare flags, no extra options.
    pub fn new() -> Self {
        Self {
            mounts: vec![],
            unshare: vec![],
            die_with_parent: false,
            new_session: false,
        }
    }

    /// Sensible minimal sandbox:
    ///   - /proc, /dev, /tmp
    ///   - Unshare: Net, Pid, Ipc
    ///   - die_with_parent = true
    pub fn minimal() -> Self {
        Self {
            mounts: vec![
                MountSpec::Proc {
                    dst: "/proc".to_string(),
                },
                MountSpec::Dev {
                    dst: "/dev".to_string(),
                },
                MountSpec::Tmpfs {
                    dst: "/tmp".to_string(),
                },
            ],
            unshare: vec![UnshareFlag::Net, UnshareFlag::Pid, UnshareFlag::Ipc],
            die_with_parent: true,
            new_session: false,
        }
    }

    /// Remove the Net unshare flag (allow outbound network).
    pub fn with_network(mut self) -> Self {
        self.unshare.retain(|f| *f != UnshareFlag::Net);
        self
    }

    /// Append a read-only bind mount.
    pub fn add_ro(mut self, src: impl Into<String>, dst: impl Into<String>) -> Self {
        self.mounts.push(MountSpec::RoBind {
            src: src.into(),
            dst: dst.into(),
        });
        self
    }

    /// Append a read-write bind mount.
    pub fn add_rw(mut self, src: impl Into<String>, dst: impl Into<String>) -> Self {
        self.mounts.push(MountSpec::RwBind {
            src: src.into(),
            dst: dst.into(),
        });
        self
    }

    /// Append a tmpfs at the given destination.
    pub fn add_tmpfs(mut self, dst: impl Into<String>) -> Self {
        self.mounts.push(MountSpec::Tmpfs { dst: dst.into() });
        self
    }

    pub fn unshares_network(&self) -> bool {
        self.unshare.contains(&UnshareFlag::Net)
    }

    pub fn unshares_pid(&self) -> bool {
        self.unshare.contains(&UnshareFlag::Pid)
    }

    pub fn ro_count(&self) -> usize {
        self.mounts.iter().filter(|m| m.is_ro_bind()).count()
    }

    pub fn rw_count(&self) -> usize {
        self.mounts.iter().filter(|m| m.is_rw_bind()).count()
    }

    pub fn mount_count(&self) -> usize {
        self.mounts.len()
    }

    /// Build the full bwrap argv (without the leading "bwrap" binary name).
    ///
    /// Order: mounts → unshare flags → --die-with-parent → --new-session
    pub fn to_args(&self) -> Vec<String> {
        let mut args: Vec<String> = vec![];

        // Mounts
        for mount in &self.mounts {
            args.extend(mount.to_args());
        }

        // Unshare flags
        for flag in &self.unshare {
            args.push(flag.to_arg().to_string());
        }

        // Lifecycle options
        if self.die_with_parent {
            args.push("--die-with-parent".to_string());
        }
        if self.new_session {
            args.push("--new-session".to_string());
        }

        args
    }

    /// Validate the profile: error if any destination path appears more than once.
    pub fn validate(&self) -> Result<(), BwrapValidationError> {
        let mut seen: Vec<&str> = vec![];
        for mount in &self.mounts {
            let dst = mount.dst();
            if seen.contains(&dst) {
                return Err(BwrapValidationError {
                    message: format!("Duplicate mount destination: '{}'", dst),
                });
            }
            seen.push(dst);
        }
        Ok(())
    }
}

impl Default for BwrapProfile {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// TDD unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_unshares_network() {
        let profile = BwrapProfile::minimal();
        assert!(profile.unshares_network());
    }

    #[test]
    fn test_minimal_unshares_pid() {
        let profile = BwrapProfile::minimal();
        assert!(profile.unshares_pid());
    }

    #[test]
    fn test_with_network_removes_net_unshare() {
        let profile = BwrapProfile::minimal().with_network();
        assert!(!profile.unshares_network());
        // Other flags should remain
        assert!(profile.unshares_pid());
    }

    #[test]
    fn test_add_ro_increases_ro_count() {
        let profile = BwrapProfile::minimal();
        let before = profile.ro_count();
        let profile = profile.add_ro("/usr", "/usr");
        assert_eq!(profile.ro_count(), before + 1);
    }

    #[test]
    fn test_add_rw_increases_rw_count() {
        let profile = BwrapProfile::minimal();
        let before = profile.rw_count();
        let profile = profile.add_rw("/workspace", "/workspace");
        assert_eq!(profile.rw_count(), before + 1);
    }

    #[test]
    fn test_to_args_contains_proc() {
        let args = BwrapProfile::minimal().to_args();
        let idx = args.iter().position(|a| a == "--proc");
        assert!(idx.is_some(), "--proc flag not found in {:?}", args);
        // Next arg should be the destination
        let dst = &args[idx.unwrap() + 1];
        assert_eq!(dst, "/proc");
    }

    #[test]
    fn test_to_args_contains_die_with_parent() {
        let args = BwrapProfile::minimal().to_args();
        assert!(
            args.contains(&"--die-with-parent".to_string()),
            "--die-with-parent not in {:?}",
            args
        );
    }

    #[test]
    fn test_validate_duplicate_dst_fails() {
        let profile = BwrapProfile::minimal()
            .add_ro("/usr", "/usr")
            .add_ro("/lib", "/usr"); // duplicate dst
        assert!(
            profile.validate().is_err(),
            "expected validation error for duplicate dst"
        );
    }

    #[test]
    fn test_validate_clean_profile_ok() {
        let profile = BwrapProfile::minimal()
            .add_ro("/usr", "/usr")
            .add_ro("/lib", "/lib");
        assert!(
            profile.validate().is_ok(),
            "expected clean validation, got error"
        );
    }
}
