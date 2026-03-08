//! VibeUI Extensions - Extension system and VSCode API compatibility

pub mod api;
pub mod loader;
pub mod manifest;

pub use loader::{ExtensionAPI, ExtensionLoader};
pub use manifest::{ExtensionManifest, ExtensionRegistry, Permission, VersionReq};
