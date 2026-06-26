/*!
 * BDD coverage for the MCP Tasks extension + stateless _meta (gap C3).
 * Run with: cargo test --test mcp_tasks_bdd
 */
use cucumber::{given, then, when, World};
use serde_json::json;
use vibecli_cli::mcp_tasks::{RequestMeta, TaskRegistry, TaskState, TASKS_EXTENSION_KEY};

#[derive(Default, World)]
#[allow(dead_code)]
pub struct TasksWorld {
    reg: TaskRegistry,
    task_id: Option<String>,
    last_err: Option<String>,
    meta: Option<RequestMeta>,
}

impl std::fmt::Debug for TasksWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TasksWorld")
            .field("task_id", &self.task_id)
            .field("tasks", &self.reg.len())
            .finish()
    }
}

// ── Given ─────────────────────────────────────────────────────────────────

#[given("a fresh task registry")]
fn fresh_registry(w: &mut TasksWorld) {
    w.reg = TaskRegistry::new();
}

#[given("a request whose _meta advertises the tasks extension")]
fn meta_with_tasks(w: &mut TasksWorld) {
    let body = json!({
        "method": "tools/call",
        "params": { "_meta": { "extensions": [TASKS_EXTENSION_KEY] } }
    });
    w.meta = Some(RequestMeta::from_request_json(&body));
}

// ── When ──────────────────────────────────────────────────────────────────

#[when(expr = "I create a task for tool {string}")]
fn create_task(w: &mut TasksWorld, tool: String) {
    w.task_id = Some(w.reg.create(&tool).id);
}

#[when(expr = "I update the task progress to {int}")]
fn update_progress(w: &mut TasksWorld, p: u8) {
    let id = w.task_id.clone().unwrap();
    w.reg.update(&id, Some(p), None, None, None).unwrap();
}

#[when("I complete the task with a result")]
fn complete_task(w: &mut TasksWorld) {
    let id = w.task_id.clone().unwrap();
    w.reg
        .update(
            &id,
            None,
            Some(TaskState::Completed),
            Some(json!({"ok": true})),
            None,
        )
        .unwrap();
}

#[when("I cancel the task")]
fn cancel_task(w: &mut TasksWorld) {
    let id = w.task_id.clone().unwrap();
    w.reg.cancel(&id).unwrap();
}

// ── Then ──────────────────────────────────────────────────────────────────

#[then(expr = "the task state is {string}")]
fn state_is(w: &mut TasksWorld, expected: String) {
    let id = w.task_id.clone().unwrap();
    let state = w.reg.get(&id).unwrap().state;
    let actual = match state {
        TaskState::Working => "working",
        TaskState::InputRequired => "input_required",
        TaskState::Completed => "completed",
        TaskState::Failed => "failed",
        TaskState::Cancelled => "cancelled",
    };
    assert_eq!(actual, expected);
}

#[then(expr = "the task progress is {int}")]
fn progress_is(w: &mut TasksWorld, expected: u8) {
    let id = w.task_id.clone().unwrap();
    assert_eq!(w.reg.get(&id).unwrap().progress, expected);
}

#[then("completing it without a result fails")]
fn complete_without_result_fails(w: &mut TasksWorld) {
    let id = w.task_id.clone().unwrap();
    assert!(w
        .reg
        .update(&id, None, Some(TaskState::Completed), None, None)
        .is_err());
}

#[then("updating it afterward fails")]
fn update_after_terminal_fails(w: &mut TasksWorld) {
    let id = w.task_id.clone().unwrap();
    assert!(w.reg.update(&id, Some(10), None, None, None).is_err());
}

#[then("the request supports tasks")]
fn supports_tasks(w: &mut TasksWorld) {
    assert!(w.meta.as_ref().unwrap().supports_tasks());
}

fn main() {
    futures::executor::block_on(TasksWorld::run("tests/features/mcp_tasks.feature"));
}
