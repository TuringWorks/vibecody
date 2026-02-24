//! Extension host API — host functions exposed to WASM extensions.
//!
//! Extensions are compiled to WASM and loaded from `~/.vibeui/extensions/*.wasm`.
//! Each extension may export:
//!   - `init() -> i32`               — called once on load
//!   - `on_file_save(ptr, len)`      — called when a file is saved
//!   - `on_text_change(ptr, len)`    — called on editor text changes
//!
//! The host exposes via WASM imports:
//!   - `host_log(ptr, len)`          — write a log message
//!   - `host_read_file(ptr, len, out_ptr, out_cap)` → i32 bytes written
//!   - `host_write_file(path_ptr, path_len, data_ptr, data_len)` → i32 (0=ok)

/// Name of the WASM module (import namespace) for host functions.
pub const HOST_MODULE: &str = "vibeui_host";

/// Exported extension lifecycle function names.
pub const EXPORT_INIT: &str = "init";
pub const EXPORT_ON_FILE_SAVE: &str = "on_file_save";
pub const EXPORT_ON_TEXT_CHANGE: &str = "on_text_change";

/// Host function names exposed in the `HOST_MODULE` namespace.
pub const HOST_LOG: &str = "log";
pub const HOST_READ_FILE: &str = "read_file";
pub const HOST_WRITE_FILE: &str = "write_file";
pub const HOST_NOTIFY: &str = "notify";

/// Maximum memory a single extension is allowed to allocate (64 MiB).
pub const MAX_MEMORY_BYTES: u64 = 64 * 1024 * 1024;
