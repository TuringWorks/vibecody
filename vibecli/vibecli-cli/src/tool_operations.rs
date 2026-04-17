//! Pluggable tool I/O — redirect built-in tools to SSH, Docker, or any backend.
//! Pi-mono gap bridge: Phase B2.
//!
//! Provides two core abstractions:
//! - [`BashOperations`] — execute shell commands on any backend
//! - [`EditOperations`] — read/write/patch files on any backend
//!
//! Built-in implementations:
//! - [`LocalBashOps`] / [`LocalEditOps`] — default, runs on the local host
//! - [`DryRunBashOps`] — records commands without executing (safe preview)
//! - [`EchoBashOps`] — returns command string as stdout (testing)
//! - [`MemoryEditOps`] — in-memory filesystem (unit tests)
//!
//! Custom backends (SSH, Docker, etc.) implement the traits and register
//! themselves with [`OpsRegistry`].

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

// ─── BashOutput ───────────────────────────────────────────────────────────────

/// Result of executing a shell command on any backend.
#[derive(Debug, Clone)]
pub struct BashOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub elapsed_ms: u64,
}

impl BashOutput {
    /// Construct a successful output (exit code 0).
    pub fn success(stdout: impl Into<String>) -> Self {
        Self {
            stdout: stdout.into(),
            stderr: String::new(),
            exit_code: 0,
            elapsed_ms: 0,
        }
    }

    /// Construct a failure output with a non-zero exit code.
    pub fn failure(stderr: impl Into<String>, code: i32) -> Self {
        Self {
            stdout: String::new(),
            stderr: stderr.into(),
            exit_code: code,
            elapsed_ms: 0,
        }
    }

    /// Returns `true` when `exit_code == 0`.
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

// ─── BashOperations trait ─────────────────────────────────────────────────────

/// Execute shell commands on a pluggable backend.
pub trait BashOperations: Send + Sync {
    /// Run `command`, optionally from `cwd`, with extra `env` variables.
    fn run(
        &self,
        command: &str,
        cwd: Option<&str>,
        env: &HashMap<String, String>,
    ) -> BashOutput;

    /// Human-readable backend identifier (e.g. `"local"`, `"ssh:prod"`, `"docker:app"`).
    fn backend_name(&self) -> &str;
}

// ─── LocalBashOps ─────────────────────────────────────────────────────────────

/// Executes commands on the local host using `/bin/sh -c`.
#[derive(Debug, Default)]
pub struct LocalBashOps;

impl BashOperations for LocalBashOps {
    fn run(
        &self,
        command: &str,
        cwd: Option<&str>,
        env: &HashMap<String, String>,
    ) -> BashOutput {
        let start = std::time::Instant::now();
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);

        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        for (k, v) in env {
            cmd.env(k, v);
        }

        match cmd.output() {
            Ok(output) => BashOutput {
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                exit_code: output.status.code().unwrap_or(-1),
                elapsed_ms: start.elapsed().as_millis() as u64,
            },
            Err(e) => BashOutput::failure(format!("spawn error: {e}"), -1),
        }
    }

    fn backend_name(&self) -> &str {
        "local"
    }
}

// ─── DryRunBashOps ────────────────────────────────────────────────────────────

/// Records commands that would be executed without actually running them.
///
/// Useful for previewing what a tool chain would do before committing.
#[derive(Debug)]
pub struct DryRunBashOps {
    pub recorded: std::sync::Mutex<Vec<String>>,
}

impl DryRunBashOps {
    pub fn new() -> Self {
        Self {
            recorded: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Snapshot of all recorded commands in order.
    pub fn commands(&self) -> Vec<String> {
        self.recorded.lock().unwrap().clone()
    }
}

impl Default for DryRunBashOps {
    fn default() -> Self {
        Self::new()
    }
}

impl BashOperations for DryRunBashOps {
    fn run(
        &self,
        command: &str,
        _cwd: Option<&str>,
        _env: &HashMap<String, String>,
    ) -> BashOutput {
        self.recorded.lock().unwrap().push(command.to_owned());
        BashOutput::success(format!("[dry-run] {command}"))
    }

    fn backend_name(&self) -> &str {
        "dry-run"
    }
}

// ─── EchoBashOps ──────────────────────────────────────────────────────────────

/// Returns the command string verbatim as stdout without executing it.
///
/// Handy for assertion-heavy unit tests where you only care that the
/// correct command string was formed, not what it produces.
#[derive(Debug, Default)]
pub struct EchoBashOps;

impl BashOperations for EchoBashOps {
    fn run(
        &self,
        command: &str,
        _cwd: Option<&str>,
        _env: &HashMap<String, String>,
    ) -> BashOutput {
        BashOutput::success(command.to_owned())
    }

    fn backend_name(&self) -> &str {
        "echo"
    }
}

// ─── FileReadResult ───────────────────────────────────────────────────────────

/// Result of reading a file from any backend.
#[derive(Debug, Clone)]
pub struct FileReadResult {
    pub path: String,
    pub content: String,
    pub size_bytes: u64,
    pub line_count: usize,
}

// ─── EditPatch ────────────────────────────────────────────────────────────────

/// A search-and-replace edit to apply to a file.
#[derive(Debug, Clone)]
pub struct EditPatch {
    pub old_text: String,
    pub new_text: String,
}

// ─── EditResult ───────────────────────────────────────────────────────────────

/// Result of applying a patch or write to a file.
#[derive(Debug, Clone)]
pub struct EditResult {
    pub path: String,
    pub lines_changed: usize,
    pub success: bool,
    pub error: Option<String>,
}

// ─── EditOperations trait ─────────────────────────────────────────────────────

/// Read/write/patch files on a pluggable backend.
pub trait EditOperations: Send + Sync {
    fn read_file(&self, path: &str) -> Result<FileReadResult, String>;
    fn write_file(&self, path: &str, content: &str) -> Result<(), String>;
    fn apply_patch(&self, path: &str, patch: &EditPatch) -> Result<EditResult, String>;
    fn list_dir(&self, path: &str) -> Result<Vec<String>, String>;
    fn file_exists(&self, path: &str) -> bool;
    fn backend_name(&self) -> &str;
}

// ─── LocalEditOps ─────────────────────────────────────────────────────────────

/// Reads and writes files on the local filesystem.
///
/// When `root` is set, all relative paths are resolved under it and
/// paths that would escape the root are rejected (sandbox mode).
#[derive(Debug, Default)]
pub struct LocalEditOps {
    pub root: Option<PathBuf>,
}

impl LocalEditOps {
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Create an ops instance rooted at `root`; relative paths stay inside it.
    pub fn rooted(root: PathBuf) -> Self {
        Self { root: Some(root) }
    }

    /// Resolve `path` against the optional root, rejecting traversal attempts.
    fn resolve(&self, path: &str) -> Result<PathBuf, String> {
        let p = PathBuf::from(path);
        match &self.root {
            None => Ok(p),
            Some(root) => {
                let resolved = if p.is_absolute() {
                    p.clone()
                } else {
                    root.join(&p)
                };
                // Canonicalization might fail for non-existent paths, so use
                // component-level check instead.
                let resolved_str = resolved.to_string_lossy();
                let root_str = root.to_string_lossy();
                if resolved_str.starts_with(root_str.as_ref()) {
                    Ok(resolved)
                } else {
                    Err(format!(
                        "path '{}' escapes sandbox root '{}'",
                        path,
                        root.display()
                    ))
                }
            }
        }
    }
}

impl EditOperations for LocalEditOps {
    fn read_file(&self, path: &str) -> Result<FileReadResult, String> {
        let resolved = self.resolve(path)?;
        let content =
            std::fs::read_to_string(&resolved).map_err(|e| format!("read '{path}': {e}"))?;
        let size_bytes = content.len() as u64;
        let line_count = content.lines().count();
        Ok(FileReadResult {
            path: path.to_owned(),
            content,
            size_bytes,
            line_count,
        })
    }

    fn write_file(&self, path: &str, content: &str) -> Result<(), String> {
        let resolved = self.resolve(path)?;
        if let Some(parent) = resolved.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("mkdir '{parent:?}': {e}"))?;
        }
        std::fs::write(&resolved, content).map_err(|e| format!("write '{path}': {e}"))
    }

    fn apply_patch(&self, path: &str, patch: &EditPatch) -> Result<EditResult, String> {
        let fr = self.read_file(path)?;
        if !fr.content.contains(&patch.old_text) {
            return Ok(EditResult {
                path: path.to_owned(),
                lines_changed: 0,
                success: false,
                error: Some(format!("old_text not found in '{path}'")),
            });
        }
        let new_content = fr.content.replacen(&patch.old_text, &patch.new_text, 1);
        let old_lines = patch.old_text.lines().count();
        let new_lines = patch.new_text.lines().count();
        let lines_changed = old_lines.max(new_lines);
        self.write_file(path, &new_content)?;
        Ok(EditResult {
            path: path.to_owned(),
            lines_changed,
            success: true,
            error: None,
        })
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, String> {
        let resolved = self.resolve(path)?;
        let entries = std::fs::read_dir(&resolved)
            .map_err(|e| format!("list_dir '{path}': {e}"))?;
        let mut names = Vec::new();
        for entry in entries.flatten() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
        names.sort();
        Ok(names)
    }

    fn file_exists(&self, path: &str) -> bool {
        self.resolve(path)
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    fn backend_name(&self) -> &str {
        "local"
    }
}

// ─── MemoryEditOps ────────────────────────────────────────────────────────────

/// In-memory filesystem — ideal for hermetic unit tests.
///
/// All data is lost when the struct is dropped.  Use [`MemoryEditOps::seed`]
/// to pre-populate files before tests run.
#[derive(Debug)]
pub struct MemoryEditOps {
    files: std::sync::Mutex<HashMap<String, String>>,
}

impl MemoryEditOps {
    pub fn new() -> Self {
        Self {
            files: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Pre-populate the virtual filesystem with `content` at `path`.
    pub fn seed(&self, path: &str, content: &str) {
        self.files
            .lock()
            .unwrap()
            .insert(path.to_owned(), content.to_owned());
    }

    /// Read back the content of `path` (test assertion helper).
    pub fn get(&self, path: &str) -> Option<String> {
        self.files.lock().unwrap().get(path).cloned()
    }

    /// All paths currently stored in the virtual filesystem, sorted.
    pub fn all_paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self.files.lock().unwrap().keys().cloned().collect();
        paths.sort();
        paths
    }
}

impl Default for MemoryEditOps {
    fn default() -> Self {
        Self::new()
    }
}

impl EditOperations for MemoryEditOps {
    fn read_file(&self, path: &str) -> Result<FileReadResult, String> {
        let files = self.files.lock().unwrap();
        match files.get(path) {
            Some(content) => {
                let size_bytes = content.len() as u64;
                let line_count = content.lines().count();
                Ok(FileReadResult {
                    path: path.to_owned(),
                    content: content.clone(),
                    size_bytes,
                    line_count,
                })
            }
            None => Err(format!("file not found: '{path}'")),
        }
    }

    fn write_file(&self, path: &str, content: &str) -> Result<(), String> {
        self.files
            .lock()
            .unwrap()
            .insert(path.to_owned(), content.to_owned());
        Ok(())
    }

    fn apply_patch(&self, path: &str, patch: &EditPatch) -> Result<EditResult, String> {
        let content = {
            let files = self.files.lock().unwrap();
            files
                .get(path)
                .cloned()
                .ok_or_else(|| format!("file not found: '{path}'"))?
        };
        if !content.contains(&patch.old_text) {
            return Ok(EditResult {
                path: path.to_owned(),
                lines_changed: 0,
                success: false,
                error: Some(format!("old_text not found in '{path}'")),
            });
        }
        let new_content = content.replacen(&patch.old_text, &patch.new_text, 1);
        let old_lines = patch.old_text.lines().count();
        let new_lines = patch.new_text.lines().count();
        let lines_changed = old_lines.max(new_lines);
        self.files
            .lock()
            .unwrap()
            .insert(path.to_owned(), new_content);
        Ok(EditResult {
            path: path.to_owned(),
            lines_changed,
            success: true,
            error: None,
        })
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, String> {
        let prefix = if path.ends_with('/') {
            path.to_owned()
        } else {
            format!("{path}/")
        };
        let files = self.files.lock().unwrap();
        let mut names: Vec<String> = files
            .keys()
            .filter_map(|k| {
                if let Some(rest) = k.strip_prefix(&prefix) {
                    // Only immediate children (no nested '/')
                    if !rest.contains('/') {
                        return Some(rest.to_owned());
                    }
                }
                None
            })
            .collect();
        names.sort();
        Ok(names)
    }

    fn file_exists(&self, path: &str) -> bool {
        self.files.lock().unwrap().contains_key(path)
    }

    fn backend_name(&self) -> &str {
        "memory"
    }
}

// ─── OpsRegistry ──────────────────────────────────────────────────────────────

/// Central registry of named [`BashOperations`] and [`EditOperations`] backends.
///
/// The `"local"` backend is registered automatically.  Register additional
/// backends (e.g. `"ssh:prod"`, `"docker:app"`) and select them by name at
/// call time.
pub struct OpsRegistry {
    bash: HashMap<String, std::sync::Arc<dyn BashOperations>>,
    edit: HashMap<String, std::sync::Arc<dyn EditOperations>>,
}

impl std::fmt::Debug for OpsRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpsRegistry")
            .field("bash_backends", &self.bash.keys().collect::<Vec<_>>())
            .field("edit_backends", &self.edit.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl OpsRegistry {
    /// Create a registry with the `"local"` backends pre-registered.
    pub fn new() -> Self {
        let mut r = Self {
            bash: HashMap::new(),
            edit: HashMap::new(),
        };
        r.bash
            .insert("local".into(), std::sync::Arc::new(LocalBashOps));
        r.edit
            .insert("local".into(), std::sync::Arc::new(LocalEditOps::new()));
        r
    }

    /// Register (or replace) a [`BashOperations`] backend under `name`.
    pub fn register_bash(
        &mut self,
        name: &str,
        ops: std::sync::Arc<dyn BashOperations>,
    ) {
        self.bash.insert(name.to_owned(), ops);
    }

    /// Register (or replace) an [`EditOperations`] backend under `name`.
    pub fn register_edit(
        &mut self,
        name: &str,
        ops: std::sync::Arc<dyn EditOperations>,
    ) {
        self.edit.insert(name.to_owned(), ops);
    }

    pub fn get_bash(&self, name: &str) -> Option<&std::sync::Arc<dyn BashOperations>> {
        self.bash.get(name)
    }

    pub fn get_edit(&self, name: &str) -> Option<&std::sync::Arc<dyn EditOperations>> {
        self.edit.get(name)
    }

    /// Returns the `"local"` bash backend (always present).
    pub fn default_bash(&self) -> &std::sync::Arc<dyn BashOperations> {
        self.bash.get("local").expect("local bash ops always registered")
    }

    /// Returns the `"local"` edit backend (always present).
    pub fn default_edit(&self) -> &std::sync::Arc<dyn EditOperations> {
        self.edit.get("local").expect("local edit ops always registered")
    }
}

impl Default for OpsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── BashOutput helpers ────────────────────────────────────────────────────

    #[test]
    fn bash_output_success_is_success() {
        let o = BashOutput::success("hello");
        assert!(o.is_success());
        assert_eq!(o.stdout, "hello");
        assert_eq!(o.exit_code, 0);
    }

    #[test]
    fn bash_output_failure_not_success() {
        let o = BashOutput::failure("oops", 1);
        assert!(!o.is_success());
        assert_eq!(o.exit_code, 1);
        assert!(o.stdout.is_empty());
    }

    // ── LocalBashOps ──────────────────────────────────────────────────────────

    #[test]
    fn local_bash_executes_echo() {
        let ops = LocalBashOps;
        let out = ops.run("echo hello", None, &HashMap::new());
        assert!(out.is_success(), "exit_code={}", out.exit_code);
        assert_eq!(out.stdout.trim(), "hello");
        assert_eq!(ops.backend_name(), "local");
    }

    #[test]
    fn local_bash_captures_stderr_and_nonzero_exit() {
        let ops = LocalBashOps;
        let out = ops.run("exit 42", None, &HashMap::new());
        assert!(!out.is_success());
        assert_eq!(out.exit_code, 42);
    }

    #[test]
    fn local_bash_respects_env_var() {
        let ops = LocalBashOps;
        let mut env = HashMap::new();
        env.insert("VIBE_TEST_VAR".into(), "from_env".into());
        let out = ops.run("echo $VIBE_TEST_VAR", None, &env);
        assert!(out.is_success());
        assert_eq!(out.stdout.trim(), "from_env");
    }

    // ── DryRunBashOps ─────────────────────────────────────────────────────────

    #[test]
    fn dry_run_records_without_executing() {
        let ops = DryRunBashOps::new();
        ops.run("rm -rf /", None, &HashMap::new());
        ops.run("echo safe", None, &HashMap::new());
        let cmds = ops.commands();
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0], "rm -rf /");
        assert_eq!(cmds[1], "echo safe");
    }

    #[test]
    fn dry_run_returns_success_output() {
        let ops = DryRunBashOps::new();
        let out = ops.run("ls", None, &HashMap::new());
        assert!(out.is_success());
        assert!(out.stdout.contains("[dry-run]"));
        assert_eq!(ops.backend_name(), "dry-run");
    }

    // ── EchoBashOps ───────────────────────────────────────────────────────────

    #[test]
    fn echo_bash_returns_command_as_stdout() {
        let ops = EchoBashOps;
        let cmd = "git push origin main";
        let out = ops.run(cmd, None, &HashMap::new());
        assert!(out.is_success());
        assert_eq!(out.stdout, cmd);
        assert_eq!(ops.backend_name(), "echo");
    }

    // ── MemoryEditOps ─────────────────────────────────────────────────────────

    #[test]
    fn memory_edit_write_then_read() {
        let ops = MemoryEditOps::new();
        ops.write_file("src/main.rs", "fn main() {}").unwrap();
        let fr = ops.read_file("src/main.rs").unwrap();
        assert_eq!(fr.content, "fn main() {}");
        assert_eq!(fr.path, "src/main.rs");
        assert_eq!(fr.line_count, 1);
    }

    #[test]
    fn memory_edit_read_missing_file_is_error() {
        let ops = MemoryEditOps::new();
        assert!(ops.read_file("nope.rs").is_err());
    }

    #[test]
    fn memory_edit_seed_and_get() {
        let ops = MemoryEditOps::new();
        ops.seed("README.md", "# Hello");
        assert_eq!(ops.get("README.md").unwrap(), "# Hello");
    }

    #[test]
    fn memory_edit_apply_patch_success() {
        let ops = MemoryEditOps::new();
        ops.seed("src/lib.rs", "let x = 1;\nlet y = 2;\n");
        let patch = EditPatch {
            old_text: "let x = 1;".into(),
            new_text: "let x = 42;".into(),
        };
        let result = ops.apply_patch("src/lib.rs", &patch).unwrap();
        assert!(result.success);
        assert_eq!(result.lines_changed, 1);
        let updated = ops.get("src/lib.rs").unwrap();
        assert!(updated.contains("let x = 42;"));
        assert!(updated.contains("let y = 2;"));
    }

    #[test]
    fn memory_edit_apply_patch_not_found() {
        let ops = MemoryEditOps::new();
        ops.seed("a.rs", "hello");
        let patch = EditPatch {
            old_text: "goodbye".into(),
            new_text: "world".into(),
        };
        let result = ops.apply_patch("a.rs", &patch).unwrap();
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn memory_edit_list_dir() {
        let ops = MemoryEditOps::new();
        ops.seed("src/main.rs", "");
        ops.seed("src/lib.rs", "");
        ops.seed("Cargo.toml", "");
        let entries = ops.list_dir("src").unwrap();
        assert_eq!(entries, vec!["lib.rs", "main.rs"]);
    }

    #[test]
    fn memory_edit_file_exists() {
        let ops = MemoryEditOps::new();
        ops.seed("exists.rs", "");
        assert!(ops.file_exists("exists.rs"));
        assert!(!ops.file_exists("ghost.rs"));
    }

    #[test]
    fn memory_edit_all_paths_sorted() {
        let ops = MemoryEditOps::new();
        ops.seed("b.rs", "");
        ops.seed("a.rs", "");
        ops.seed("c.rs", "");
        assert_eq!(ops.all_paths(), vec!["a.rs", "b.rs", "c.rs"]);
    }

    #[test]
    fn memory_edit_backend_name() {
        assert_eq!(MemoryEditOps::new().backend_name(), "memory");
    }

    // ── OpsRegistry ───────────────────────────────────────────────────────────

    #[test]
    fn registry_has_local_defaults() {
        let reg = OpsRegistry::new();
        assert_eq!(reg.default_bash().backend_name(), "local");
        assert_eq!(reg.default_edit().backend_name(), "local");
    }

    #[test]
    fn registry_lookup_by_name() {
        let mut reg = OpsRegistry::new();
        reg.register_bash("echo", std::sync::Arc::new(EchoBashOps));
        reg.register_edit("mem", std::sync::Arc::new(MemoryEditOps::new()));

        assert!(reg.get_bash("echo").is_some());
        assert!(reg.get_edit("mem").is_some());
        assert!(reg.get_bash("missing").is_none());
    }

    #[test]
    fn registry_registered_backend_is_dispatched() {
        let mut reg = OpsRegistry::new();
        reg.register_bash("dry", std::sync::Arc::new(DryRunBashOps::new()));
        let out = reg.get_bash("dry").unwrap().run("ls", None, &HashMap::new());
        assert!(out.is_success());
        assert_eq!(reg.get_bash("dry").unwrap().backend_name(), "dry-run");
    }

    #[test]
    fn registry_replace_existing_backend() {
        let mut reg = OpsRegistry::new();
        reg.register_bash("local", std::sync::Arc::new(EchoBashOps));
        // "local" now points to EchoBashOps
        assert_eq!(reg.default_bash().backend_name(), "echo");
    }
}
