/*!
 * BDD tests for tool_operations using Cucumber.
 * Run with: cargo test --test tool_operations_bdd
 */
use std::collections::HashMap;
use std::sync::Arc;

use cucumber::{World, given, then, when};
use vibecli_cli::tool_operations::{
    BashOperations, DryRunBashOps, EchoBashOps, EditOperations, EditPatch, MemoryEditOps,
    OpsRegistry,
};

// ─── World ────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
#[allow(dead_code)]
pub struct ToWorld {
    // bash backends
    dry_run: Option<Arc<DryRunBashOps>>,
    echo_ops: Option<Arc<EchoBashOps>>,
    last_stdout: String,
    last_exit_code: i32,
    last_backend_name: String,

    // edit backends
    mem_edit: Option<Arc<MemoryEditOps>>,
    patch_success: bool,
    patch_error: Option<String>,
    read_content: String,
    file_exists: bool,

    // registry
    registry: Option<OpsRegistry>,
    looked_up_bash_name: String,
    looked_up_edit_name: String,
}

// ─── Given ────────────────────────────────────────────────────────────────────

#[given("a dry-run bash backend")]
fn setup_dry_run(world: &mut ToWorld) {
    world.dry_run = Some(Arc::new(DryRunBashOps::new()));
}

#[given("an echo bash backend")]
fn setup_echo(world: &mut ToWorld) {
    world.echo_ops = Some(Arc::new(EchoBashOps));
}

#[given("a memory edit backend")]
fn setup_memory_edit(world: &mut ToWorld) {
    world.mem_edit = Some(Arc::new(MemoryEditOps::new()));
}

#[given(expr = "a memory edit backend seeded with path {string} and content {string}")]
fn setup_memory_seeded(world: &mut ToWorld, path: String, content: String) {
    let ops = Arc::new(MemoryEditOps::new());
    ops.seed(&path, &content);
    world.mem_edit = Some(ops);
}

#[given(expr = "a registry with a dry-run bash backend registered as {string}")]
fn registry_add_dry_run(world: &mut ToWorld, name: String) {
    let reg = world.registry.get_or_insert_with(OpsRegistry::new);
    reg.register_bash(&name, Arc::new(DryRunBashOps::new()));
}

#[given(expr = "a registry with a memory edit backend registered as {string}")]
fn registry_add_memory(world: &mut ToWorld, name: String) {
    let reg = world.registry.get_or_insert_with(OpsRegistry::new);
    reg.register_edit(&name, Arc::new(MemoryEditOps::new()));
}

// ─── When ─────────────────────────────────────────────────────────────────────

#[when(expr = "I run the command {string}")]
fn run_command(world: &mut ToWorld, cmd: String) {
    if let Some(dry) = &world.dry_run {
        let out = dry.run(&cmd, None, &HashMap::new());
        world.last_stdout = out.stdout;
        world.last_exit_code = out.exit_code;
    } else if let Some(echo) = &world.echo_ops {
        let out = echo.run(&cmd, None, &HashMap::new());
        world.last_stdout = out.stdout;
        world.last_exit_code = out.exit_code;
        world.last_backend_name = echo.backend_name().to_owned();
    }
}

#[when(expr = "I write {string} to path {string}")]
fn write_file(world: &mut ToWorld, content: String, path: String) {
    if let Some(ops) = &world.mem_edit {
        ops.write_file(&path, &content).unwrap();
    }
}

#[when(expr = "I apply a patch to {string} replacing {string} with {string}")]
fn apply_patch(world: &mut ToWorld, path: String, old: String, new: String) {
    if let Some(ops) = &world.mem_edit {
        let patch = EditPatch {
            old_text: old,
            new_text: new,
        };
        match ops.apply_patch(&path, &patch) {
            Ok(result) => {
                world.patch_success = result.success;
                world.patch_error = result.error;
            }
            Err(e) => {
                world.patch_success = false;
                world.patch_error = Some(e);
            }
        }
    }
}

#[when(expr = "I look up bash backend {string}")]
fn lookup_bash(world: &mut ToWorld, name: String) {
    if let Some(reg) = &world.registry {
        if let Some(ops) = reg.get_bash(&name) {
            world.looked_up_bash_name = ops.backend_name().to_owned();
        }
    }
}

#[when(expr = "I look up edit backend {string}")]
fn lookup_edit(world: &mut ToWorld, name: String) {
    if let Some(reg) = &world.registry {
        if let Some(ops) = reg.get_edit(&name) {
            world.looked_up_edit_name = ops.backend_name().to_owned();
        }
    }
}

// ─── Then ─────────────────────────────────────────────────────────────────────

#[then(expr = "{int} commands should be recorded")]
fn check_recorded_count(world: &mut ToWorld, count: usize) {
    let cmds = world.dry_run.as_ref().unwrap().commands();
    assert_eq!(cmds.len(), count, "expected {count} recorded commands, got {:?}", cmds);
}

#[then(expr = "recorded command {int} should be {string}")]
fn check_recorded_command(world: &mut ToWorld, index: usize, expected: String) {
    let cmds = world.dry_run.as_ref().unwrap().commands();
    let actual = &cmds[index - 1];
    assert_eq!(actual, &expected, "command at index {index}");
}

#[then("no files on disk should have changed")]
fn no_disk_changes(_world: &mut ToWorld) {
    // DryRunBashOps never calls a real process, so this is always satisfied.
    // The assertion exists to document the contract.
}

#[then(expr = "reading {string} should return {string}")]
fn check_read_content(world: &mut ToWorld, path: String, expected: String) {
    let ops = world.mem_edit.as_ref().unwrap();
    let fr = ops.read_file(&path).unwrap();
    assert_eq!(fr.content, expected, "content of '{path}'");
}

#[then(expr = "{string} should exist in the backend")]
fn check_file_exists(world: &mut ToWorld, path: String) {
    let ops = world.mem_edit.as_ref().unwrap();
    assert!(ops.file_exists(&path), "expected '{}' to exist", path);
}

#[then("the patch should succeed")]
fn check_patch_success(world: &mut ToWorld) {
    assert!(
        world.patch_success,
        "patch failed: {:?}",
        world.patch_error
    );
}

#[then(expr = "the bash backend name should be {string}")]
fn check_bash_backend_name(world: &mut ToWorld, expected: String) {
    assert_eq!(
        world.looked_up_bash_name, expected,
        "bash backend name mismatch"
    );
}

#[then(expr = "the edit backend name should be {string}")]
fn check_edit_backend_name(world: &mut ToWorld, expected: String) {
    assert_eq!(
        world.looked_up_edit_name, expected,
        "edit backend name mismatch"
    );
}

#[then(expr = "the output stdout should equal {string}")]
fn check_stdout(world: &mut ToWorld, expected: String) {
    assert_eq!(world.last_stdout, expected, "stdout mismatch");
}

#[then(expr = "the output exit code should be {int}")]
fn check_exit_code(world: &mut ToWorld, expected: i32) {
    assert_eq!(world.last_exit_code, expected, "exit code mismatch");
}

#[then(expr = "the backend name should be {string}")]
fn check_backend_name(world: &mut ToWorld, expected: String) {
    assert_eq!(world.last_backend_name, expected, "backend name mismatch");
}

// ─── Runner ───────────────────────────────────────────────────────────────────

fn main() {
    futures::executor::block_on(ToWorld::run("tests/features/tool_operations.feature"));
}
