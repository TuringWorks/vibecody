//! Tier-0 native sandbox implementations.
//!
//! - Linux: `bwrap` + Landlock + seccomp (slice N1.x)
//! - macOS: structured `.sb` profile + `sandbox-exec` (slice N2.1)
//! - Windows: AppContainer + Restricted Token + Job Object (slice N3.x)
//!
//! See `docs/design/sandbox-tiers/01-native-tier.md`.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
mod windows_impl;

use vibe_sandbox::{Result, Sandbox};
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use vibe_sandbox::{SandboxError, SandboxTier};

/// Construct the OS-appropriate native Tier-0 sandbox.
pub fn native() -> Result<Box<dyn Sandbox>> {
    #[cfg(target_os = "linux")]
    {
        return linux::LinuxSandbox::new().map(|s| Box::new(s) as Box<dyn Sandbox>);
    }
    #[cfg(target_os = "macos")]
    {
        return macos::MacosSandbox::new().map(|s| Box::new(s) as Box<dyn Sandbox>);
    }
    #[cfg(target_os = "windows")]
    {
        return windows_impl::WindowsSandbox::new().map(|s| Box::new(s) as Box<dyn Sandbox>);
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(SandboxError::TierUnsupported {
            tier: SandboxTier::Native,
        })
    }
}
