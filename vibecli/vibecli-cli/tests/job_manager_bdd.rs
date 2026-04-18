/*!
 * BDD tests for the durable async-job queue (JobManager + JobsDb).
 * Run with: cargo test --test job_manager_bdd
 */
use cucumber::{given, then, when, World};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::broadcast;
use vibecli_cli::job_manager::{
    AgentEventPayload, CreateJobReq, JobManager, JobRecord, JobStatus, JobsDb,
};

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

#[derive(Debug, Default, World)]
pub struct JmWorld {
    manager: Option<Arc<JobManager>>,
    tmp: Option<Arc<TempDir>>,
    /// Most recently created job id (tests that only touch one job).
    last_job_id: Option<String>,
    /// task → session_id map for scenarios that create more than one job.
    jobs_by_task: HashMap<String, String>,
    /// Last job record fetched from the manager.
    last_record: Option<JobRecord>,
    /// Last full list from `JobManager::list`.
    last_list: Vec<JobRecord>,
    /// Return value of `recover_interrupted`.
    last_recovery: usize,
    /// Return value of `migrate_json_jobs.imported`.
    last_import_count: usize,
    /// Legacy jobs directory for migration scenarios.
    legacy_jobs_dir: Option<PathBuf>,
    /// Cached event sender for the last open_stream call.
    stream_tx: Option<broadcast::Sender<AgentEventPayload>>,
    /// Cached event receiver for the last subscribe call.
    stream_rx: Option<broadcast::Receiver<AgentEventPayload>>,
    /// Event received from a subscriber (if any).
    received_event: Option<AgentEventPayload>,
    /// Whether the most recent subscribe attempt returned Some(_).
    subscribe_some: Option<bool>,
}

impl JmWorld {
    fn manager(&self) -> &JobManager {
        self.manager
            .as_ref()
            .expect("JobManager not initialised — missing Given step")
    }
    fn session_id_for(&self, task: &str) -> String {
        self.jobs_by_task
            .get(task)
            .cloned()
            .unwrap_or_else(|| panic!("no job created for task {task:?}"))
    }
    fn require_last_id(&self) -> String {
        self.last_job_id
            .clone()
            .expect("no last job id — create a job first")
    }
    async fn refresh_last_record(&mut self) {
        let id = self.require_last_id();
        self.last_record = self.manager().get(&id).await;
    }
}

fn default_req(task: &str) -> CreateJobReq {
    CreateJobReq {
        task: task.into(),
        provider: "mock".into(),
        approval: "auto".into(),
        workspace_root: "/tmp/ws".into(),
        priority: 5,
        webhook_url: None,
        tags: vec![],
        quota_bucket: None,
    }
}

// ---------------------------------------------------------------------------
// Given
// ---------------------------------------------------------------------------

#[given("a fresh JobManager")]
fn given_fresh_manager(world: &mut JmWorld) {
    let tmp = TempDir::new().unwrap();
    let db = JobsDb::open_with(&tmp.path().join("jobs.db"), [42u8; 32]).unwrap();
    world.manager = Some(Arc::new(JobManager::new_with(db)));
    world.tmp = Some(Arc::new(tmp));
    world.jobs_by_task.clear();
    world.last_job_id = None;
    world.last_record = None;
    world.last_list.clear();
    world.last_recovery = 0;
    world.last_import_count = 0;
    world.legacy_jobs_dir = None;
    world.stream_tx = None;
    world.stream_rx = None;
    world.received_event = None;
    world.subscribe_some = None;
}

#[given(expr = "a created job with task {string}")]
async fn given_created_job(world: &mut JmWorld, task: String) {
    let id = world
        .manager()
        .create(default_req(&task))
        .await
        .expect("create job");
    world.jobs_by_task.insert(task, id.clone());
    world.last_job_id = Some(id);
    world.refresh_last_record().await;
}

#[given(expr = "a legacy jobs directory with a running JSON record {string}")]
fn given_legacy_dir_with_running(world: &mut JmWorld, id: String) {
    let tmp = world.tmp.as_ref().expect("world tmp").clone();
    let dir = tmp.path().join("legacy-jobs");
    std::fs::create_dir_all(&dir).unwrap();
    // Write a legacy (pre-M1) JSON shape — 7 fields only.
    let json = serde_json::json!({
        "session_id": id,
        "task": "interrupted",
        "status": "running",
        "provider": "ollama",
        "started_at": 500,
        "finished_at": null,
        "summary": null,
    });
    std::fs::write(
        dir.join(format!("{id}.json")),
        serde_json::to_string(&json).unwrap(),
    )
    .unwrap();
    world.legacy_jobs_dir = Some(dir);
}

// ---------------------------------------------------------------------------
// When
// ---------------------------------------------------------------------------

#[when(expr = "I create a job with task {string}")]
async fn when_create_job(world: &mut JmWorld, task: String) {
    let id = world
        .manager()
        .create(default_req(&task))
        .await
        .expect("create job");
    world.jobs_by_task.insert(task, id.clone());
    world.last_job_id = Some(id);
    world.refresh_last_record().await;
}

#[when("I mark the job running")]
async fn when_mark_running(world: &mut JmWorld) {
    let id = world.require_last_id();
    world.manager().mark_running(&id).await.unwrap();
    world.refresh_last_record().await;
}

#[when(expr = "I mark the job {string} running")]
async fn when_mark_named_running(world: &mut JmWorld, task: String) {
    let id = world.session_id_for(&task);
    world.manager().mark_running(&id).await.unwrap();
    world.last_job_id = Some(id);
    world.refresh_last_record().await;
}

#[when(expr = "I mark the job complete with summary {string}")]
async fn when_mark_complete(world: &mut JmWorld, summary: String) {
    let id = world.require_last_id();
    world
        .manager()
        .mark_terminal(&id, JobStatus::Complete, Some(summary), None)
        .await
        .unwrap();
    world.refresh_last_record().await;
}

#[when(expr = "I cancel the job with reason {string}")]
async fn when_cancel_with_reason(world: &mut JmWorld, reason: String) {
    let id = world.require_last_id();
    world.manager().cancel(&id, Some(reason)).await;
    world.refresh_last_record().await;
}

#[when("I call recover_interrupted")]
async fn when_recover(world: &mut JmWorld) {
    world.last_recovery = world.manager().recover_interrupted().await.unwrap();
    world.refresh_last_record().await;
}

#[when(expr = "I wait {int} ms")]
async fn when_wait(_world: &mut JmWorld, ms: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
}

#[when("I call migrate_json_jobs")]
async fn when_migrate(world: &mut JmWorld) {
    let dir = world
        .legacy_jobs_dir
        .clone()
        .expect("legacy jobs dir not set");
    let rep = world.manager().migrate_json_jobs(&dir).await.unwrap();
    world.last_import_count = rep.imported;
}

#[when("I call migrate_json_jobs again")]
async fn when_migrate_again(world: &mut JmWorld) {
    when_migrate(world).await;
}

#[when("I open a stream for the job")]
async fn when_open_stream(world: &mut JmWorld) {
    let id = world.require_last_id();
    world.stream_tx = Some(world.manager().open_stream(&id).await);
}

#[when("I subscribe to the job's stream")]
async fn when_subscribe(world: &mut JmWorld) {
    let id = world.require_last_id();
    let rx = world.manager().subscribe(&id).await;
    world.subscribe_some = Some(rx.is_some());
    world.stream_rx = rx;
}

#[when(expr = "I publish a chunk event with content {string}")]
async fn when_publish_chunk(world: &mut JmWorld, content: String) {
    let tx = world
        .stream_tx
        .as_ref()
        .expect("stream_tx not set — open the stream first");
    let _ = tx.send(AgentEventPayload::chunk(content));
    // Pull the event off the subscriber's receiver synchronously so
    // subsequent Then steps have something to assert on.
    if let Some(rx) = world.stream_rx.as_mut() {
        if let Ok(ev) = rx.recv().await {
            world.received_event = Some(ev);
        }
    }
}

#[when("I close the stream")]
async fn when_close_stream(world: &mut JmWorld) {
    let id = world.require_last_id();
    world.manager().close_stream(&id).await;
}

// ---------------------------------------------------------------------------
// Then
// ---------------------------------------------------------------------------

#[then("the job should exist")]
fn then_job_exists(world: &mut JmWorld) {
    assert!(world.last_record.is_some(), "expected job to exist");
}

#[then(expr = "the job status should be {string}")]
fn then_status(world: &mut JmWorld, expected: String) {
    let rec = world.last_record.as_ref().expect("last_record");
    assert_eq!(rec.status, expected, "job status mismatch");
}

#[then(expr = "the job task should be {string}")]
fn then_task(world: &mut JmWorld, expected: String) {
    let rec = world.last_record.as_ref().expect("last_record");
    assert_eq!(rec.task, expected, "job task mismatch");
}

#[then(expr = "the job priority should be {int}")]
fn then_priority(world: &mut JmWorld, expected: u8) {
    let rec = world.last_record.as_ref().expect("last_record");
    assert_eq!(rec.priority, expected, "job priority mismatch");
}

#[then("the job queued_at should be non-zero")]
fn then_queued_at_nonzero(world: &mut JmWorld) {
    let rec = world.last_record.as_ref().expect("last_record");
    assert!(rec.queued_at > 0, "queued_at should be non-zero");
}

#[then("the job started_at should be zero")]
fn then_started_at_zero(world: &mut JmWorld) {
    let rec = world.last_record.as_ref().expect("last_record");
    assert_eq!(rec.started_at, 0, "started_at should be zero for queued job");
}

#[then("the job started_at should be non-zero")]
fn then_started_at_nonzero(world: &mut JmWorld) {
    let rec = world.last_record.as_ref().expect("last_record");
    assert!(rec.started_at > 0, "started_at should be non-zero");
}

#[then("the job finished_at should be non-zero")]
fn then_finished_at_nonzero(world: &mut JmWorld) {
    let rec = world.last_record.as_ref().expect("last_record");
    assert!(
        rec.finished_at.unwrap_or(0) > 0,
        "finished_at should be non-zero"
    );
}

#[then(expr = "the job summary should be {string}")]
fn then_summary(world: &mut JmWorld, expected: String) {
    let rec = world.last_record.as_ref().expect("last_record");
    assert_eq!(
        rec.summary.as_deref(),
        Some(expected.as_str()),
        "job summary mismatch"
    );
}

#[then(expr = "the job cancellation_reason should be {string}")]
fn then_cancellation_reason(world: &mut JmWorld, expected: String) {
    let rec = world.last_record.as_ref().expect("last_record");
    assert_eq!(
        rec.cancellation_reason.as_deref(),
        Some(expected.as_str()),
        "cancellation_reason mismatch"
    );
}

#[then(expr = "the recovery count should be {int}")]
fn then_recovery_count(world: &mut JmWorld, expected: usize) {
    assert_eq!(
        world.last_recovery, expected,
        "recover_interrupted count mismatch"
    );
}

#[then(expr = "the migration imported count should be {int}")]
fn then_imported_count(world: &mut JmWorld, expected: usize) {
    assert_eq!(
        world.last_import_count, expected,
        "migration imported count mismatch"
    );
}

#[then(expr = "the job {string} status should be {string}")]
async fn then_named_job_status(world: &mut JmWorld, task_or_id: String, expected: String) {
    // The task_or_id can be either a task name or a literal id string
    // (legacy migration tests use the id directly).
    let id = world
        .jobs_by_task
        .get(&task_or_id)
        .cloned()
        .unwrap_or(task_or_id);
    let rec = world
        .manager()
        .get(&id)
        .await
        .unwrap_or_else(|| panic!("job {id} not found"));
    assert_eq!(rec.status, expected, "job {id} status mismatch");
}

#[then(expr = "the job list should have {int} entries")]
async fn then_list_len(world: &mut JmWorld, expected: usize) {
    world.last_list = world.manager().list().await;
    assert_eq!(world.last_list.len(), expected, "list length mismatch");
}

#[then(expr = "the first listed job task should be {string}")]
fn then_first_list_task(world: &mut JmWorld, expected: String) {
    let list = &world.last_list;
    assert!(!list.is_empty(), "list is empty");
    assert_eq!(list[0].task, expected, "first list entry mismatch");
}

#[then(expr = "the second listed job task should be {string}")]
fn then_second_list_task(world: &mut JmWorld, expected: String) {
    let list = &world.last_list;
    assert!(list.len() >= 2, "list has fewer than 2 entries");
    assert_eq!(list[1].task, expected, "second list entry mismatch");
}

#[then(expr = "the subscriber should receive a chunk event with content {string}")]
fn then_received_chunk(world: &mut JmWorld, expected: String) {
    let ev = world
        .received_event
        .as_ref()
        .expect("no event received — publish first");
    assert_eq!(ev.kind, "chunk", "event kind mismatch");
    assert_eq!(
        ev.content.as_deref(),
        Some(expected.as_str()),
        "event content mismatch"
    );
}

#[then("subscribing to the job's stream should return nothing")]
async fn then_subscribe_empty(world: &mut JmWorld) {
    let id = world.require_last_id();
    assert!(
        world.manager().subscribe(&id).await.is_none(),
        "expected subscribe to return None after close_stream"
    );
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    JmWorld::run("tests/features/job_manager.feature").await;
}
