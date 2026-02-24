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
        let len = bytes.len() as i32;

        // alloc(len) → ptr (i32)
        let mut results = [Val::I32(0)];
        alloc.call(&mut self.store, &[Val::I32(len)], &mut results)?;
        let ptr = results[0].unwrap_i32() as usize;

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
            .with_context(|| format!("compile {}", path.display()))?;

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
            .with_context(|| format!("instantiate {}", path.display()))?;

        // Call init() if present.
        if let Some(init_fn) = instance.get_func(&mut store, EXPORT_INIT) {
            let mut results = [Val::I32(0)];
            init_fn
                .call(&mut store, &[], &mut results)
                .with_context(|| format!("init() in {}", name))?;
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
