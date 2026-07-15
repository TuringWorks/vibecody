//! Backend-agnostic conformance tests, run against every compiled backend.

use fluxo_core::model::TaskType;
use fluxo_core::run::{TaskExecution, TaskStatus, WorkflowRun, WorkflowStatus};
use fluxo_core::{parse_workflow_def, WorkflowDef};
use fluxo_store::memory::MemoryStore;
use fluxo_store::Store;
use serde_json::{json, Map, Value};

fn sample_def(version: u32) -> WorkflowDef {
    let mut def = parse_workflow_def(
        r#"{ "name": "demo", "tasks": [
            { "name": "do_work", "taskReferenceName": "w" }
        ]}"#,
    )
    .expect("parse");
    def.version = version;
    def
}

fn run_with_scheduled_task(id: &str) -> WorkflowRun {
    WorkflowRun {
        workflow_id: id.into(),
        workflow_name: "demo".into(),
        workflow_version: 1,
        status: WorkflowStatus::Running,
        input: json!({}),
        output: Value::Null,
        variables: Map::new(),
        tasks: vec![TaskExecution {
            task_id: "t1".into(),
            reference_name: "w".into(),
            task_type: TaskType::Simple,
            task_name: "do_work".into(),
            status: TaskStatus::Scheduled,
            input: json!({}),
            output: Value::Null,
            retry_count: 0,
            scheduled_at: 1,
            updated_at: 1,
            worker_id: None,
            reason_for_incompletion: None,
        }],
        correlation_id: None,
        reason_for_incompletion: None,
        created_at: 1,
        updated_at: 1,
    }
}

async fn exercise(store: &dyn Store) {
    // Definition versioning.
    store.put_workflow_def(&sample_def(1)).await.unwrap();
    store.put_workflow_def(&sample_def(2)).await.unwrap();
    assert_eq!(store.get_workflow_def("demo", None).await.unwrap().unwrap().version, 2);
    assert_eq!(store.get_workflow_def("demo", Some(1)).await.unwrap().unwrap().version, 1);
    assert!(store.get_workflow_def("missing", None).await.unwrap().is_none());
    assert_eq!(store.list_workflow_defs().await.unwrap().len(), 2);

    // Run round-trip + durability.
    let run = run_with_scheduled_task("wf-a");
    store.create_run(&run).await.unwrap();
    let fetched = store.get_run("wf-a").await.unwrap().unwrap();
    assert_eq!(fetched.tasks.len(), 1);

    // Status filter.
    assert_eq!(store.list_runs(Some(WorkflowStatus::Running)).await.unwrap().len(), 1);
    assert_eq!(store.list_runs(Some(WorkflowStatus::Completed)).await.unwrap().len(), 0);

    // Poll claims the scheduled worker task and flips it to InProgress.
    let polled = store.poll_task("do_work", "worker-x").await.unwrap().unwrap();
    assert_eq!(polled.task.reference_name, "w");
    assert_eq!(polled.task.status, TaskStatus::InProgress);
    assert!(store.poll_task("do_work", "worker-x").await.unwrap().is_none());

    // The claim persisted.
    let after = store.get_run("wf-a").await.unwrap().unwrap();
    assert_eq!(after.tasks[0].status, TaskStatus::InProgress);
    assert_eq!(after.tasks[0].worker_id.as_deref(), Some("worker-x"));
}

#[tokio::test]
async fn memory_backend_conforms() {
    exercise(&MemoryStore::new()).await;
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn sqlite_backend_conforms() {
    let store = fluxo_store::sqlite::SqliteStore::open_in_memory().unwrap();
    exercise(&store).await;
}
