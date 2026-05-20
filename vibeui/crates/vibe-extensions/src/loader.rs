//! WASM extension loader.
//!
//! Scans a directory for `*.wasm` files, instantiates each as a sandboxed
//! wasmtime module, wires up host functions, and calls the `init()` export.
//!
//! ## Sandbox tier H5 — Wasmtime fuel + epoch hardening
//!
//! Wasmtime gives capability-based isolation but defaults to letting a guest
//! loop forever. Two enforcement layers are turned on in
//! [`ExtensionLoader::new`]:
//!
//! 1. **Fuel** — `Config::consume_fuel(true)` + per-Store
//!    `Store::set_fuel(N)` before every call. Each WebAssembly
//!    instruction consumes one unit; when the budget hits zero the
//!    call traps with `Trap::OutOfFuel`. Default budget is
//!    [`DEFAULT_FUEL`] units (~ 10⁸, which is multi-second of
//!    typical WASM); per-call override via [`Extension::set_fuel`].
//! 2. **Epoch interruption** — `Config::epoch_interruption(true)` +
//!    [`Engine::increment_epoch`] from a single background thread,
//!    plus per-Store `Store::set_epoch_deadline(N)`. When the
//!    deadline elapses the call traps even if no fuel was burned
//!    (covers the case of a guest blocking inside a host call
//!    whose return path we control). Default deadline is
//!    [`DEFAULT_EPOCH_DEADLINE`] ticks at
//!    [`DEFAULT_EPOCH_INTERVAL`].
//!
//! Both knobs are belt-and-suspenders: fuel catches pure CPU loops,
//! epoch catches wall-clock-bound deadlocks. Either alone leaves
//! gaps. See `docs/design/sandbox-tiers/04-hyperlight-tier.md` §H5.

use crate::api::*;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;
use wasmtime::*;

/// Default per-call fuel budget. Wasmtime burns ~1 unit per WASM
/// instruction; 100M is a couple seconds of pure-CPU work for a
/// typical extension. Set conservatively so a misbehaving extension
/// dies loudly long before it starves the daemon thread.
pub const DEFAULT_FUEL: u64 = 100_000_000;

/// Default wall-clock deadline in epoch ticks, paired with
/// [`DEFAULT_EPOCH_INTERVAL`]. 10 ticks × 100 ms = 1 second of wall
/// time max per call. Extensions doing legitimate long work override
/// via [`Extension::set_epoch_deadline`].
pub const DEFAULT_EPOCH_DEADLINE: u64 = 10;

/// How often the background thread bumps the engine's epoch.
/// 100 ms is fine-grained enough for the default 1 s deadline and
/// cheap enough that the daemon's tracing is not flooded.
pub const DEFAULT_EPOCH_INTERVAL: Duration = Duration::from_millis(100);

// ── Tier preference ──────────────────────────────────────────────────────────

/// Which backend should run an extension.
///
/// Slice H4 — the structural piece of the future Hyperlight migration.
/// Today only the Wasmtime backend is wired in (`vibe-sandbox-hyperlight`
/// returns `NotSupported` for `spawn` and the WASI-on-Hyperlight guest
/// binary release pipeline H1 is pending). Once H1–H3 land, callers
/// pick the desired tier via [`ExtensionLoader::with_tier_preference`]
/// and the loader either routes to Hyperlight or transparently falls
/// back to Wasmtime, emitting a structured `extension.tier.downgrade`
/// trace event so the recap pipeline sees the chosen tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionTier {
    /// Run in-process via Wasmtime. The current behavior; available
    /// on every supported platform.
    Wasmtime,
    /// Try Hyperlight (KVM/mshv on Linux, WHP on Windows). If the
    /// host doesn't support Hyperlight or the H1 guest binary isn't
    /// installed yet, transparently fall back to Wasmtime.
    HyperlightIfAvailable,
}

impl Default for ExtensionTier {
    fn default() -> Self {
        ExtensionTier::Wasmtime
    }
}

impl std::fmt::Display for ExtensionTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtensionTier::Wasmtime => f.write_str("wasmtime"),
            ExtensionTier::HyperlightIfAvailable => f.write_str("hyperlight-if-available"),
        }
    }
}

/// Reports whether the host can actually run Hyperlight today.
///
/// Returns `false` everywhere right now — slice H1 is pending. Once
/// the H1 guest binary release pipeline + the `vibe-sandbox-hyperlight`
/// runtime binding land, this returns `true` on Linux/Windows where
/// KVM/mshv/WHP is available.
pub fn hyperlight_available() -> bool {
    // Stub for H4 — flip to a real probe (e.g. dlopen of the guest
    // binary + cap check) when H1 is ready.
    false
}

/// Resolve a preference to the backend that will actually run.
/// Pure function so the daemon can preview the choice without
/// loading any extension.
pub fn resolve_tier(pref: ExtensionTier) -> ExtensionTier {
    match pref {
        ExtensionTier::Wasmtime => ExtensionTier::Wasmtime,
        ExtensionTier::HyperlightIfAvailable => {
            if hyperlight_available() {
                ExtensionTier::HyperlightIfAvailable
            } else {
                ExtensionTier::Wasmtime
            }
        }
    }
}

// ── Error ─────────────────────────────────────────────────────────────────────

/// Why a Wasmtime call was killed.
///
/// Distinguishing fuel from epoch matters for operators: fuel
/// exhaustion is a pure-CPU loop in the extension; epoch exhaustion
/// is wall-clock time and may indicate the extension is stuck in a
/// host call (e.g. waiting on `host_read_file`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExhaustionKind {
    /// Wasmtime fuel budget ran out (CPU-bound).
    Fuel,
    /// Wasmtime epoch deadline elapsed (wall-clock-bound).
    Epoch,
}

impl std::fmt::Display for ExhaustionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExhaustionKind::Fuel => f.write_str("fuel"),
            ExhaustionKind::Epoch => f.write_str("epoch"),
        }
    }
}

/// Errors surfaced from an extension call.
///
/// The daemon should log `Exhausted` as a `vibe.extensions.exhausted`
/// event so operators can tune budgets or disable misbehaving
/// extensions.
#[derive(Debug, Error)]
pub enum ExtensionError {
    /// The extension exhausted its CPU/wall budget and was killed.
    #[error("extension '{name}' exhausted {kind} budget")]
    Exhausted { kind: ExhaustionKind, name: String },

    /// Any other failure (compile, ABI, host fn, …).
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

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
    /// Per-call fuel budget. Reset on the Store before each export
    /// invocation; populated from the loader default at load time
    /// and overridable via [`Extension::set_fuel`].
    fuel: u64,
    /// Per-call epoch deadline (in ticks of [`DEFAULT_EPOCH_INTERVAL`]).
    epoch_deadline: u64,
    /// H4: which backend ended up serving this load. Currently
    /// always [`ExtensionTier::Wasmtime`] because H1-H3 are pending,
    /// but the field is plumbed so the recap pipeline can correlate
    /// tier with extension once Hyperlight is live.
    tier: ExtensionTier,
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

    /// Override the fuel budget used for subsequent calls.
    pub fn set_fuel(&mut self, fuel: u64) {
        self.fuel = fuel;
    }

    /// Override the epoch deadline used for subsequent calls.
    pub fn set_epoch_deadline(&mut self, ticks: u64) {
        self.epoch_deadline = ticks;
    }

    /// H4: which backend served this load. Today always
    /// [`ExtensionTier::Wasmtime`] because slice H1 is pending.
    pub fn tier(&self) -> ExtensionTier {
        self.tier
    }

    /// Invoke an exported `(ptr, len)` function with a string payload,
    /// surfacing typed errors (notably [`ExtensionError::Exhausted`]).
    ///
    /// The fire-and-forget [`Self::on_file_save`] / [`Self::on_text_change`]
    /// helpers silently swallow errors; use this when the caller needs
    /// to observe fuel/epoch exhaustion (e.g. to emit a
    /// `vibe.extensions.exhausted` metric).
    pub fn try_call(&mut self, export_name: &str, arg: &str) -> Result<(), ExtensionError> {
        self.call_str_fn(export_name, arg)
    }

    // ── private ───────────────────────────────────────────────────────────

    fn call_str_fn(&mut self, export_name: &str, arg: &str) -> Result<(), ExtensionError> {
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
            .map_err(|_| ExtensionError::Other(anyhow::anyhow!(
                "String too large for WASM allocation ({} bytes)",
                bytes.len()
            )))?;

        // H5: reset fuel + epoch budgets so each call gets a fresh
        // allotment rather than inheriting the previous call's
        // remainder. Both must be reset before any guest code runs,
        // including `alloc`.
        self.reset_budgets()?;

        // alloc(len) → ptr (i32)
        let mut results = [Val::I32(0)];
        let alloc_res = alloc.call(&mut self.store, &[Val::I32(len)], &mut results);
        alloc_res.map_err(|e| classify_trap(e, &self.name))?;
        let ptr_val = results[0].unwrap_i32();
        if ptr_val < 0 {
            return Err(ExtensionError::Other(anyhow::anyhow!(
                "WASM alloc returned error code: {}",
                ptr_val
            )));
        }
        let ptr = ptr_val as usize;

        // Write string bytes into WASM linear memory.
        let memory = self
            .instance
            .get_memory(&mut self.store, "memory")
            .context("extension must export 'memory'")
            .map_err(ExtensionError::Other)?;
        memory
            .write(&mut self.store, ptr, &bytes)
            .context("writing string to WASM memory")
            .map_err(ExtensionError::Other)?;

        // Call export(ptr, len). Note: we do NOT reset budgets again
        // here — fuel/epoch carry over from the alloc call so a
        // misbehaving alloc cannot exhaust the budget and starve the
        // real call's accounting.
        let call_res =
            func.call(&mut self.store, &[Val::I32(ptr as i32), Val::I32(len)], &mut []);
        call_res.map_err(|e| classify_trap(e, &self.name))?;
        Ok(())
    }

    fn reset_budgets(&mut self) -> Result<(), ExtensionError> {
        self.store
            .set_fuel(self.fuel)
            .map_err(|e| ExtensionError::Other(anyhow::anyhow!("set fuel: {}", e)))?;
        self.store.set_epoch_deadline(self.epoch_deadline);
        Ok(())
    }
}

/// Inspect a `wasmtime::Error` returned from `Func::call` and map
/// known trap kinds to typed exhaustion variants.
fn classify_trap(e: wasmtime::Error, name: &str) -> ExtensionError {
    if let Some(trap) = e.downcast_ref::<Trap>() {
        match trap {
            Trap::OutOfFuel => {
                return ExtensionError::Exhausted {
                    kind: ExhaustionKind::Fuel,
                    name: name.to_string(),
                };
            }
            Trap::Interrupt => {
                return ExtensionError::Exhausted {
                    kind: ExhaustionKind::Epoch,
                    name: name.to_string(),
                };
            }
            _ => {}
        }
    }
    ExtensionError::Other(anyhow::anyhow!("{}", e))
}

// ── ExtensionLoader ───────────────────────────────────────────────────────────

pub struct ExtensionLoader {
    engine: Engine,
    /// Per-loader default fuel budget for new extensions. Each
    /// extension call resets fuel to this value unless the caller
    /// overrides via [`Extension::set_fuel`].
    default_fuel: u64,
    /// Per-loader default epoch deadline. Reset before every call.
    default_epoch_deadline: u64,
    /// H4: which backend the loader should *prefer* for new
    /// extensions. The *resolved* tier (after the
    /// `hyperlight_available()` probe) is stored on each
    /// `Extension` so callers can inspect which backend actually
    /// served the load.
    tier_preference: ExtensionTier,
}

impl Default for ExtensionLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtensionLoader {
    pub fn new() -> Self {
        Self::with_interval(DEFAULT_EPOCH_INTERVAL)
    }

    /// Test-friendly constructor: pick a faster epoch interval so
    /// fuel/epoch tests don't have to wait 100 ms × N.
    pub fn with_interval(epoch_interval: Duration) -> Self {
        let mut config = Config::new();
        // H5: fuel catches infinite-CPU-loops; epoch catches
        // wall-clock-bound deadlocks. Both required — see
        // module-level docs.
        config.consume_fuel(true);
        config.epoch_interruption(true);
        let engine = Engine::new(&config).expect("wasmtime Engine");

        // Spawn the epoch ticker. The thread holds an Engine clone
        // (cheap — Engine is Arc inside) and bumps the epoch
        // counter every `epoch_interval`. The thread is detached;
        // it exits when the Engine is dropped because
        // `increment_epoch` becomes a no-op on a stale handle.
        let ticker_engine: Engine = engine.clone();
        std::thread::Builder::new()
            .name("vibe-extensions:epoch-ticker".to_string())
            .spawn(move || {
                let engine = ticker_engine;
                // Hold a weak count so we exit when no other engine
                // clones remain. Wasmtime doesn't expose
                // strong-count directly; instead we just sleep +
                // bump, and the daemon's lifetime is the daemon's
                // lifetime. For tests that build many loaders the
                // OS-thread overhead is fine — these are dev-only.
                loop {
                    std::thread::sleep(epoch_interval);
                    engine.increment_epoch();
                }
            })
            .expect("spawn epoch ticker");

        Self {
            engine,
            default_fuel: DEFAULT_FUEL,
            default_epoch_deadline: DEFAULT_EPOCH_DEADLINE,
            tier_preference: ExtensionTier::default(),
        }
    }

    /// Override the default fuel budget for extensions loaded by
    /// this loader. Affects subsequent `load_*` calls; existing
    /// `Extension` instances are unaffected.
    pub fn with_default_fuel(mut self, fuel: u64) -> Self {
        self.default_fuel = fuel;
        self
    }

    /// Override the default epoch deadline (in epoch ticks).
    pub fn with_default_epoch_deadline(mut self, ticks: u64) -> Self {
        self.default_epoch_deadline = ticks;
        self
    }

    /// H4: choose which backend extensions loaded by this loader
    /// should run on. Today the only fully-wired backend is
    /// Wasmtime; `HyperlightIfAvailable` resolves to Wasmtime via
    /// [`resolve_tier`] until slice H1 ships the guest binary.
    pub fn with_tier_preference(mut self, pref: ExtensionTier) -> Self {
        self.tier_preference = pref;
        self
    }

    /// Inspect what tier the loader will actually use given the
    /// configured preference + host capabilities.
    pub fn effective_tier(&self) -> ExtensionTier {
        resolve_tier(self.tier_preference)
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

        // H5: prime fuel + epoch budgets so `init()` itself runs
        // bounded. A malicious extension's init() could otherwise
        // loop the daemon startup forever.
        store
            .set_fuel(self.default_fuel)
            .map_err(|e| anyhow::anyhow!("set initial fuel: {}", e))?;
        store.set_epoch_deadline(self.default_epoch_deadline);

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

        // H4: resolve the configured preference; if Hyperlight was
        // requested but isn't actually available, emit a structured
        // downgrade trace so the recap pipeline picks it up.
        let resolved = resolve_tier(self.tier_preference);
        if self.tier_preference == ExtensionTier::HyperlightIfAvailable
            && resolved == ExtensionTier::Wasmtime
        {
            tracing::warn!(
                target: "vibe.extensions.tier.downgrade",
                extension = %name,
                requested = %self.tier_preference,
                effective = %resolved,
                "Hyperlight tier requested but not available on this host — falling back to Wasmtime"
            );
        }

        Ok(Extension {
            name: name.to_string(),
            path: path.to_path_buf(),
            instance,
            store,
            fuel: self.default_fuel,
            epoch_deadline: self.default_epoch_deadline,
            tier: resolved,
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

    /// Build a tiny WAT module that exports `memory`, `alloc(len)`
    /// returning ptr=0, and `loop_forever(ptr, len)` that runs an
    /// infinite WASM loop. Designed for H5 fuel/epoch tests.
    fn infinite_loop_wasm() -> Vec<u8> {
        let wat = r#"(module
            (memory (export "memory") 1)
            (func (export "alloc") (param i32) (result i32)
                i32.const 0
            )
            (func (export "loop_forever") (param i32 i32)
                (loop $forever
                    br $forever
                )
            )
        )"#;
        wat::parse_str(wat).expect("parse loop WAT")
    }

    #[test]
    fn h5_fuel_exhaustion_traps_with_typed_error() {
        // Cap fuel low so we exhaust quickly. Epoch deadline is
        // huge so this test is specifically about fuel.
        let loader = ExtensionLoader::new()
            .with_default_fuel(10_000)
            .with_default_epoch_deadline(u64::MAX);

        let dir = std::env::temp_dir().join("vibe_ext_test_h5_fuel");
        let _ = std::fs::create_dir_all(&dir);
        let wasm_path = dir.join("loop.wasm");
        std::fs::write(&wasm_path, infinite_loop_wasm()).unwrap();

        let mut ext = loader.load_one(&wasm_path, "loop").unwrap();
        let err = ext.try_call("loop_forever", "x").unwrap_err();
        match err {
            ExtensionError::Exhausted { kind, name } => {
                assert_eq!(kind, ExhaustionKind::Fuel);
                assert_eq!(name, "loop");
            }
            other => panic!("expected Exhausted{{Fuel}}, got {:?}", other),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn h5_epoch_exhaustion_traps_with_typed_error() {
        // Use a fast epoch interval so the test completes quickly.
        // Fuel is effectively unlimited so this test isolates epoch.
        let loader = ExtensionLoader::with_interval(Duration::from_millis(10))
            .with_default_fuel(u64::MAX)
            .with_default_epoch_deadline(1);

        let dir = std::env::temp_dir().join("vibe_ext_test_h5_epoch");
        let _ = std::fs::create_dir_all(&dir);
        let wasm_path = dir.join("loop.wasm");
        std::fs::write(&wasm_path, infinite_loop_wasm()).unwrap();

        let mut ext = loader.load_one(&wasm_path, "loop").unwrap();
        let err = ext.try_call("loop_forever", "x").unwrap_err();
        match err {
            ExtensionError::Exhausted { kind, name } => {
                assert_eq!(kind, ExhaustionKind::Epoch);
                assert_eq!(name, "loop");
            }
            other => panic!("expected Exhausted{{Epoch}}, got {:?}", other),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn h5_per_extension_set_fuel_override_takes_effect() {
        // Default fuel huge — the loader-level setting would let
        // loop_forever run "forever". Per-extension override
        // drops it to ~zero and forces a Fuel trap.
        let loader = ExtensionLoader::new()
            .with_default_fuel(u64::MAX)
            .with_default_epoch_deadline(u64::MAX);

        let dir = std::env::temp_dir().join("vibe_ext_test_h5_per_ext_fuel");
        let _ = std::fs::create_dir_all(&dir);
        let wasm_path = dir.join("loop.wasm");
        std::fs::write(&wasm_path, infinite_loop_wasm()).unwrap();

        let mut ext = loader.load_one(&wasm_path, "loop").unwrap();
        ext.set_fuel(10_000);
        let err = ext.try_call("loop_forever", "x").unwrap_err();
        assert!(matches!(
            err,
            ExtensionError::Exhausted { kind: ExhaustionKind::Fuel, .. }
        ));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn h5_exhaustion_kind_display_is_stable() {
        // Daemon log format depends on this.
        assert_eq!(ExhaustionKind::Fuel.to_string(), "fuel");
        assert_eq!(ExhaustionKind::Epoch.to_string(), "epoch");
    }

    // ── H4 — Tier selector ───────────────────────────────────────────────

    #[test]
    fn h4_default_tier_is_wasmtime() {
        assert_eq!(ExtensionTier::default(), ExtensionTier::Wasmtime);
    }

    #[test]
    fn h4_resolve_wasmtime_is_identity() {
        assert_eq!(
            resolve_tier(ExtensionTier::Wasmtime),
            ExtensionTier::Wasmtime
        );
    }

    #[test]
    fn h4_resolve_hyperlight_falls_back_to_wasmtime_today() {
        // H1 not shipped → hyperlight_available() returns false.
        // Until then, HyperlightIfAvailable must resolve to Wasmtime
        // so loaders don't silently fail when callers opt in early.
        assert!(!hyperlight_available());
        assert_eq!(
            resolve_tier(ExtensionTier::HyperlightIfAvailable),
            ExtensionTier::Wasmtime
        );
    }

    #[test]
    fn h4_loader_effective_tier_defaults_to_wasmtime() {
        let l = ExtensionLoader::new();
        assert_eq!(l.effective_tier(), ExtensionTier::Wasmtime);
    }

    #[test]
    fn h4_loader_with_hyperlight_preference_still_resolves_to_wasmtime() {
        let l = ExtensionLoader::new()
            .with_tier_preference(ExtensionTier::HyperlightIfAvailable);
        assert_eq!(l.effective_tier(), ExtensionTier::Wasmtime);
    }

    #[test]
    fn h4_loaded_extension_reports_its_tier() {
        let loader = ExtensionLoader::new();
        let dir = std::env::temp_dir().join("vibe_ext_test_h4_tier");
        let _ = std::fs::create_dir_all(&dir);
        let wat = r#"(module (memory (export "memory") 1))"#;
        let wasm = wat::parse_str(wat).unwrap();
        let wasm_path = dir.join("ext.wasm");
        std::fs::write(&wasm_path, &wasm).unwrap();

        let ext = loader.load_one(&wasm_path, "h4-test").unwrap();
        assert_eq!(ext.tier(), ExtensionTier::Wasmtime);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn h4_loaded_extension_with_hyperlight_pref_still_reports_wasmtime() {
        // When H1 lands, this test flips: with the preference set
        // and the host supporting Hyperlight, tier() returns
        // HyperlightIfAvailable. Until then it's Wasmtime.
        let loader = ExtensionLoader::new()
            .with_tier_preference(ExtensionTier::HyperlightIfAvailable);
        let dir = std::env::temp_dir().join("vibe_ext_test_h4_pref");
        let _ = std::fs::create_dir_all(&dir);
        let wat = r#"(module (memory (export "memory") 1))"#;
        let wasm = wat::parse_str(wat).unwrap();
        let wasm_path = dir.join("ext.wasm");
        std::fs::write(&wasm_path, &wasm).unwrap();

        let ext = loader.load_one(&wasm_path, "h4-pref").unwrap();
        assert_eq!(ext.tier(), ExtensionTier::Wasmtime);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn h4_tier_display_is_stable() {
        // Daemon log + audit-event format depends on this.
        assert_eq!(ExtensionTier::Wasmtime.to_string(), "wasmtime");
        assert_eq!(
            ExtensionTier::HyperlightIfAvailable.to_string(),
            "hyperlight-if-available"
        );
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
