//! Thin shim — delegates all panel-settings operations to
//! `vibecli_cli::profile_store::ProfileStore` which now owns the
//! unified `~/.vibecli/profile_settings.db` store.
//!
//! All existing call sites (commands.rs) continue to use `PanelStore::new()`
//! and the same method signatures without change.

pub use vibecli_cli::profile_store::ProfileStore as PanelStore;
