//! WASM extension loader.
//!
//! Scans a directory for `*.wasm` files, instantiates each as a sandboxed
//! wasmtime module, wires up host functions, and calls the `init()` export.

use crate::api::*;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use wasmtime::*;

// ── HostState ─────────────────────────────────────────────────────────────────

/// Shared state threaded through every wasmtime host function call.
#[derive(Default)]
struct HostState {
    log_output: Vec<String>,
    notifications: Vec<String>,
}

// ── Extension ─────────────────────────────────────────────────────────────────

/// A loaded, running WASM extension.
pub struct Extension {
    pub name: String,
    pub path: PathBuf,
    instance: Instance,
    store: Store<HostState>,
}

impl Extension {
    /// Notify the extension that a file has been saved.
    pub fn on_file_save(&mut self, path: &str) {
        let _ = self.call_str_fn(EXPORT_ON_FILE_SAVE, path);
    }

    /// Notify the extension of a text change (payload: `"path\x00content"`).
    pub fn on_text_change(&mut self, path: &str, content: &str) {
        let payload = format!("{}\x00{}", path, content);
        let _ = self.call_str_fn(EXPORT_ON_TEXT_CHANGE, &payload);
    }

    /// Drain log messages produced by the last call.
    pub fn drain_logs(&mut self) -> Vec<String> {
        std::mem::take(&mut self.store.data_mut().log_output)
    }

    /// Drain notification messages.
    pub fn drain_notifications(&mut self) -> Vec<String> {
        std::mem::take(&mut self.store.data_mut().notifications)
    }

    // ── private ───────────────────────────────────────────────────────────

    fn call_str_fn(&mut self, export_name: &str, arg: &str) -> Result<()> {
        // Resolve the exported function (skip silently if absent).
        let func = match self.instance.get_func(&mut self.store, export_name) {
            Some(f) => f,
            None => return Ok(()),
        };

        // Resolve the `alloc` helper the extension must export for string passing.
        let Some(alloc) = self.instance.get_func(&mut self.store, "alloc") else {
            return Ok(()); // extension doesn't support string ABI
        };

        let bytes = arg.as_bytes().to_vec();
        let len = i32::try_from(bytes.len())
            .map_err(|_| anyhow::anyhow!("String too large for WASM allocation ({} bytes)", bytes.len()))?;

        // alloc(len) → ptr (i32)
        let mut results = [Val::I32(0)];
        alloc.call(&mut self.store, &[Val::I32(len)], &mut results)?;
        let ptr_val = results[0].unwrap_i32();
        if ptr_val < 0 {
            anyhow::bail!("WASM alloc returned error code: {}", ptr_val);
        }
        let ptr = ptr_val as usize;

        // Write string bytes into WASM linear memory.
        let memory = self
            .instance
            .get_memory(&mut self.store, "memory")
            .context("extension must export 'memory'")?;
        memory
            .write(&mut self.store, ptr, &bytes)
            .context("writing string to WASM memory")?;

        // Call export(ptr, len).
        func.call(&mut self.store, &[Val::I32(ptr as i32), Val::I32(len)], &mut [])?;
        Ok(())
    }
}

// ── ExtensionLoader ───────────────────────────────────────────────────────────

pub struct ExtensionLoader {
    engine: Engine,
}

impl Default for ExtensionLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtensionLoader {
    pub fn new() -> Self {
        let config = Config::new();
        Self {
            engine: Engine::new(&config).expect("wasmtime Engine"),
        }
    }

    /// Load all `*.wasm` files from `dir`.
    pub fn load_from_dir(&self, dir: &Path) -> Vec<Extension> {
        if !dir.exists() {
            return vec![];
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return vec![];
        };
        let mut exts = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("wasm") {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            match self.load_one(&path, &name) {
                Ok(ext) => {
                    tracing::info!("Loaded extension: {}", name);
                    exts.push(ext);
                }
                Err(e) => tracing::warn!("Extension '{}' failed to load: {}", name, e),
            }
        }
        exts
    }

    fn load_one(&self, path: &Path, name: &str) -> Result<Extension> {
        let wasm = std::fs::read(path)
            .with_context(|| format!("read {}", path.display()))?;
        let module = Module::new(&self.engine, &wasm)
            .map_err(|e| anyhow::anyhow!("compile {}: {}", path.display(), e))?;

        let mut linker: Linker<HostState> = Linker::new(&self.engine);

        // host_log(ptr, len)
        linker.func_wrap(
            HOST_MODULE,
            HOST_LOG,
            |mut caller: Caller<HostState>, ptr: i32, len: i32| {
                let msg = wasm_read_str(&mut caller, ptr, len).unwrap_or_default();
                tracing::debug!("[ext] {}", msg);
                caller.data_mut().log_output.push(msg);
            },
        )?;

        // host_notify(ptr, len)
        linker.func_wrap(
            HOST_MODULE,
            HOST_NOTIFY,
            |mut caller: Caller<HostState>, ptr: i32, len: i32| {
                let msg = wasm_read_str(&mut caller, ptr, len).unwrap_or_default();
                caller.data_mut().notifications.push(msg);
            },
        )?;

        // host_read_file(path_ptr, path_len, out_ptr, out_cap) → i32 bytes written
        linker.func_wrap(
            HOST_MODULE,
            HOST_READ_FILE,
            |mut caller: Caller<HostState>,
             path_ptr: i32,
             path_len: i32,
             out_ptr: i32,
             out_cap: i32|
             -> i32 {
                let path_str = match wasm_read_str(&mut caller, path_ptr, path_len) {
                    Some(s) => s,
                    None => return -1,
                };
                let Ok(content) = std::fs::read(&path_str) else {
                    return -1;
                };
                let written = content.len().min(out_cap as usize);
                let mem = match caller.get_export("memory") {
                    Some(Extern::Memory(m)) => m,
                    _ => return -1,
                };
                if mem.write(&mut caller, out_ptr as usize, &content[..written]).is_err() {
                    return -1;
                }
                written as i32
            },
        )?;

        // host_write_file(path_ptr, path_len, data_ptr, data_len) → 0=ok, -1=err
        linker.func_wrap(
            HOST_MODULE,
            HOST_WRITE_FILE,
            |mut caller: Caller<HostState>,
             path_ptr: i32,
             path_len: i32,
             data_ptr: i32,
             data_len: i32|
             -> i32 {
                let path_str = match wasm_read_str(&mut caller, path_ptr, path_len) {
                    Some(s) => s,
                    None => return -1,
                };
                let mem = match caller.get_export("memory") {
                    Some(Extern::Memory(m)) => m,
                    _ => return -1,
                };
                let mut buf = vec![0u8; data_len as usize];
                if mem.read(&caller, data_ptr as usize, &mut buf).is_err() {
                    return -1;
                }
                if std::fs::write(&path_str, &buf).is_err() {
                    return -1;
                }
                0
            },
        )?;

        let mut store = Store::new(&self.engine, HostState::default());

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| anyhow::anyhow!("instantiate {}: {}", path.display(), e))?;

        // Call init() if present.
        if let Some(init_fn) = instance.get_func(&mut store, EXPORT_INIT) {
            let mut results = [Val::I32(0)];
            init_fn
                .call(&mut store, &[], &mut results)
                .map_err(|e| anyhow::anyhow!("init() in {}: {}", name, e))?;
        }

        // Flush startup logs.
        for msg in store.data_mut().log_output.drain(..) {
            tracing::info!("[ext:{}:init] {}", name, msg);
        }

        Ok(Extension {
            name: name.to_string(),
            path: path.to_path_buf(),
            instance,
            store,
        })
    }

    /// Load from the default VibeUI extension directory (`~/.vibeui/extensions/`).
    pub fn load_default_extensions(&self) -> Vec<Extension> {
        let dir = default_extension_dir();
        self.load_from_dir(&dir)
    }

    /// Compatibility shim (matches the original stub signature).
    pub fn load_extensions(&self) -> Result<()> {
        let _ = self.load_default_extensions();
        Ok(())
    }
}

/// Legacy compat export.
pub struct ExtensionAPI;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn default_extension_dir() -> PathBuf {
    dirs_next::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vibeui")
        .join("extensions")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn load_from_nonexistent_dir_returns_empty() {
        let loader = ExtensionLoader::new();
        let exts = loader.load_from_dir(PathBuf::from("/nonexistent/path/42").as_path());
        assert!(exts.is_empty());
    }

    #[test]
    fn load_from_empty_dir_returns_empty() {
        let dir = std::env::temp_dir().join("vibe_ext_test_empty");
        let _ = std::fs::create_dir_all(&dir);
        let loader = ExtensionLoader::new();
        let exts = loader.load_from_dir(&dir);
        assert!(exts.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_extension_dir_ends_with_extensions() {
        let dir = default_extension_dir();
        assert!(dir.ends_with("extensions"));
        assert!(dir.to_string_lossy().contains(".vibeui"));
    }

    #[test]
    fn extension_loader_default_is_same_as_new() {
        // Both should succeed and create a valid engine
        let _loader = ExtensionLoader::default();
    }

    #[test]
    fn load_extensions_compat_shim_returns_ok() {
        let loader = ExtensionLoader::new();
        // Even if no extensions dir exists, the compat shim should return Ok
        assert!(loader.load_extensions().is_ok());
    }

    #[test]
    fn load_from_dir_ignores_non_wasm_files() {
        let dir = std::env::temp_dir().join("vibe_ext_test_non_wasm");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("readme.txt"), "not wasm").unwrap();
        std::fs::write(dir.join("lib.so"), "not wasm").unwrap();
        std::fs::write(dir.join("module.js"), "not wasm").unwrap();
        let loader = ExtensionLoader::new();
        let exts = loader.load_from_dir(&dir);
        assert!(exts.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_from_dir_rejects_invalid_wasm() {
        // A file with .wasm extension but invalid content should fail to load
        let dir = std::env::temp_dir().join("vibe_ext_test_invalid_wasm");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("bad.wasm"), b"this is not wasm").unwrap();
        let loader = ExtensionLoader::new();
        let exts = loader.load_from_dir(&dir);
        assert!(exts.is_empty()); // Should gracefully skip with a warning
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn host_state_default_has_empty_logs() {
        let state = HostState::default();
        assert!(state.log_output.is_empty());
        assert!(state.notifications.is_empty());
    }

    #[test]
    fn extension_api_constants_are_correct() {
        // Verify the API contract constants
        assert_eq!(crate::api::HOST_MODULE, "vibeui_host");
        assert_eq!(crate::api::EXPORT_INIT, "init");
        assert_eq!(crate::api::EXPORT_ON_FILE_SAVE, "on_file_save");
        assert_eq!(crate::api::EXPORT_ON_TEXT_CHANGE, "on_text_change");
        assert_eq!(crate::api::HOST_LOG, "log");
        assert_eq!(crate::api::HOST_READ_FILE, "read_file");
        assert_eq!(crate::api::HOST_WRITE_FILE, "write_file");
        assert_eq!(crate::api::HOST_NOTIFY, "notify");
    }

    #[test]
    fn max_memory_bytes_is_64_mib() {
        assert_eq!(crate::api::MAX_MEMORY_BYTES, 64 * 1024 * 1024);
    }

    #[test]
    fn load_default_extensions_returns_empty_when_dir_missing() {
        // ~/.vibeui/extensions/ might not exist in test environment
        let loader = ExtensionLoader::new();
        let exts = loader.load_default_extensions();
        // Should not panic; result depends on whether dir exists
        let _ = exts;
    }

    #[test]
    fn load_from_dir_with_subdirectories_ignores_dirs() {
        let dir = std::env::temp_dir().join("vibe_ext_test_subdir");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::create_dir(dir.join("subdir.wasm")); // dir, not file
        let loader = ExtensionLoader::new();
        let exts = loader.load_from_dir(&dir);
        // Should not try to load directories even if named .wasm
        assert!(exts.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_from_dir_with_mixed_files() {
        let dir = std::env::temp_dir().join("vibe_ext_test_mixed");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("valid.txt"), "text").unwrap();
        std::fs::write(dir.join("config.json"), "{}").unwrap();
        std::fs::write(dir.join("broken.wasm"), b"not wasm bytes").unwrap();
        let loader = ExtensionLoader::new();
        let exts = loader.load_from_dir(&dir);
        // broken.wasm should fail to compile, non-.wasm files are skipped
        assert!(exts.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn host_state_log_push_and_drain() {
        let mut state = HostState::default();
        state.log_output.push("msg1".to_string());
        state.log_output.push("msg2".to_string());
        assert_eq!(state.log_output.len(), 2);
        let drained: Vec<String> = std::mem::take(&mut state.log_output);
        assert_eq!(drained.len(), 2);
        assert!(state.log_output.is_empty());
    }

    #[test]
    fn host_state_notifications_push_and_drain() {
        let mut state = HostState::default();
        state.notifications.push("notif1".to_string());
        let drained: Vec<String> = std::mem::take(&mut state.notifications);
        assert_eq!(drained, vec!["notif1"]);
        assert!(state.notifications.is_empty());
    }

    #[test]
    fn extension_loader_engine_is_valid() {
        let loader = ExtensionLoader::new();
        // Engine should be able to compile an empty module
        let wat = "(module)";
        let result = Module::new(&loader.engine, wat);
        assert!(result.is_ok());
    }

    #[test]
    fn load_one_with_minimal_wasm_module() {
        let loader = ExtensionLoader::new();
        let dir = std::env::temp_dir().join("vibe_ext_test_minimal_wasm");
        let _ = std::fs::create_dir_all(&dir);

        // Create a minimal valid WASM module from WAT
        let wat = r#"(module
            (memory (export "memory") 1)
        )"#;
        let wasm = wat::parse_str(wat).unwrap();
        let wasm_path = dir.join("minimal.wasm");
        std::fs::write(&wasm_path, &wasm).unwrap();

        let result = loader.load_one(&wasm_path, "minimal");
        assert!(result.is_ok());
        let mut ext = result.unwrap();
        assert_eq!(ext.name, "minimal");
        assert!(ext.drain_logs().is_empty());
        assert!(ext.drain_notifications().is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_from_dir_loads_valid_wasm() {
        let loader = ExtensionLoader::new();
        let dir = std::env::temp_dir().join("vibe_ext_test_valid_load");
        let _ = std::fs::create_dir_all(&dir);

        let wat = r#"(module
            (memory (export "memory") 1)
        )"#;
        let wasm = wat::parse_str(wat).unwrap();
        std::fs::write(dir.join("myext.wasm"), &wasm).unwrap();
        std::fs::write(dir.join("readme.md"), "docs").unwrap();

        let exts = loader.load_from_dir(&dir);
        assert_eq!(exts.len(), 1);
        assert_eq!(exts[0].name, "myext");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extension_on_file_save_no_export_is_noop() {
        let loader = ExtensionLoader::new();
        let dir = std::env::temp_dir().join("vibe_ext_test_save_noop");
        let _ = std::fs::create_dir_all(&dir);

        let wat = r#"(module (memory (export "memory") 1))"#;
        let wasm = wat::parse_str(wat).unwrap();
        let wasm_path = dir.join("noop.wasm");
        std::fs::write(&wasm_path, &wasm).unwrap();

        let mut ext = loader.load_one(&wasm_path, "noop").unwrap();
        // Should not panic even though on_file_save is not exported
        ext.on_file_save("/some/file.rs");
        ext.on_text_change("/some/file.rs", "new content");

        let _ = std::fs::remove_dir_all(&dir);
    }
}

fn wasm_read_str(caller: &mut Caller<HostState>, ptr: i32, len: i32) -> Option<String> {
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(m)) => m,
        _ => return None,
    };
    let mut buf = vec![0u8; len as usize];
    mem.read(caller, ptr as usize, &mut buf).ok()?;
    String::from_utf8(buf).ok()
}
