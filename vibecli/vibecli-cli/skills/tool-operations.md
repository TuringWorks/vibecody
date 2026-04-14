---
triggers: ["tool operations", "bash backend", "edit backend", "ops registry", "ssh backend", "docker backend", "dry-run ops", "memory edit", "redirect tool", "pluggable tool", "LocalBashOps", "DryRunBashOps", "MemoryEditOps", "OpsRegistry"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["cargo"]
category: rust
---

# Tool Operations — Pluggable I/O Backends

Rules for working with the `tool_operations` module (`vibecli-cli/src/tool_operations.rs`).

## Rule 1 — Use `LocalBashOps` / `LocalEditOps` only for trusted, local execution

`LocalBashOps` spawns real processes via `sh -c`.  
`LocalEditOps` reads/writes the real filesystem.  
Never use these inside sandboxed agents, Docker containers, or when the working directory is untrusted.  
For those scenarios, implement the `BashOperations` / `EditOperations` traits against the remote transport instead.

## Rule 2 — Register SSH and Docker backends in `OpsRegistry` by URI-style names

Adopt the convention `"ssh:<host>"` and `"docker:<container>"` as registry keys:

```rust
registry.register_bash("ssh:prod", Arc::new(SshBashOps::new("prod.example.com")));
registry.register_edit("docker:app", Arc::new(DockerEditOps::new("app_container")));
```

Call `registry.get_bash("ssh:prod")` at dispatch time; fall back to `registry.default_bash()` when the named backend is absent.

## Rule 3 — Use `DryRunBashOps` for safe previewing before committing side-effects

Any workflow that shows a "preview" of commands before execution should route through `DryRunBashOps`.  
After preview, swap in the real backend:

```rust
let preview: Arc<dyn BashOperations> = Arc::new(DryRunBashOps::new());
plan_phase(&preview);           // record what would run
let cmds = preview.downcast_ref::<DryRunBashOps>().unwrap().commands();
show_to_user(&cmds);
let real: Arc<dyn BashOperations> = Arc::new(LocalBashOps);
execute_phase(&real);           // actually run
```

## Rule 4 — Use `MemoryEditOps` for hermetic unit tests; never touch the real filesystem

All module-level unit tests and BDD step implementations that involve file I/O must use `MemoryEditOps`.  
Seed the virtual FS with `ops.seed(path, content)` and assert with `ops.get(path)` — no temp dirs required.

```rust
let ops = MemoryEditOps::new();
ops.seed("src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }");
let patch = EditPatch { old_text: "a + b".into(), new_text: "a.saturating_add(b)".into() };
let result = ops.apply_patch("src/lib.rs", &patch).unwrap();
assert!(result.success);
```

## Rule 5 — Root-path sandboxing: use `LocalEditOps::rooted` for workspace isolation

When an agent operates inside a workspace directory, construct the edit backend with an explicit root so relative paths stay sandboxed:

```rust
let ops = LocalEditOps::rooted(PathBuf::from("/home/user/project"));
// ops.read_file("../../etc/passwd") returns Err("escapes sandbox root")
```

Always use `rooted` in Tauri commands that accept user-supplied file paths.

## Rule 6 — Implement new backends by satisfying `Send + Sync`

Both traits require `Send + Sync`.  Use `Arc<Mutex<T>>` for any internal mutable state:

```rust
pub struct SshBashOps {
    host: String,
    client: Arc<Mutex<SshClient>>,
}
impl BashOperations for SshBashOps {
    fn run(&self, command: &str, cwd: Option<&str>, env: &HashMap<String, String>) -> BashOutput {
        let mut client = self.client.lock().unwrap();
        // ...
    }
    fn backend_name(&self) -> &str { &self.host }
}
```

## Rule 7 — Test patterns: always assert `backend_name()` to confirm dispatch

After looking up a backend from the registry, assert `.backend_name()` to confirm the correct implementation was selected, not just that `Option::Some` was returned:

```rust
let ops = registry.get_bash("dry").expect("dry backend missing");
assert_eq!(ops.backend_name(), "dry-run");
```

This protects against accidentally overwriting a key with the wrong type of backend.
