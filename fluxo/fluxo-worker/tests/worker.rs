//! Integration test: a real `fluxo-server` on an ephemeral port, driven by a `Worker`.

use fluxo_core::parse_workflow_def;
use fluxo_core::run::WorkflowStatus;
use fluxo_engine::Engine;
use fluxo_store::memory::MemoryStore;
use fluxo_worker::Worker;
use serde_json::json;
use std::sync::Arc;

async fn spawn_server(engine: Arc<Engine<MemoryStore>>) -> String {
    let app = fluxo_server::router(engine);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn worker_drives_a_two_step_workflow() {
    let engine = Arc::new(Engine::new(MemoryStore::new()));
    let def = parse_workflow_def(
        r#"{ "name": "pipeline", "tasks": [
            { "name": "step_one", "taskReferenceName": "one",
              "inputParameters": { "seed": "${workflow.input.seed}" } },
            { "name": "step_two", "taskReferenceName": "two",
              "inputParameters": { "prev": "${one.output.value}" } }
        ], "outputParameters": { "final": "${two.output.value}" } }"#,
    )
    .expect("parse");
    engine.register(&def).await.expect("register");
    let id = engine.start("pipeline", None, json!({ "seed": 10 }), None).await.expect("start");

    let base = spawn_server(engine.clone()).await;

    let mut worker = Worker::new(&base, "worker-1");
    worker.register("step_one", |ctx| async move {
        let seed = ctx.input.get("seed").and_then(|v| v.as_i64()).unwrap_or(0);
        Ok(json!({ "value": seed + 1 }))
    });
    worker.register("step_two", |ctx| async move {
        let prev = ctx.input.get("prev").and_then(|v| v.as_i64()).unwrap_or(0);
        Ok(json!({ "value": prev * 2 }))
    });

    let processed = worker.run_until_idle().await.expect("run worker");
    assert_eq!(processed, 2, "both steps should run");

    let run = engine.get_run(&id).await.expect("run");
    assert_eq!(run.status, WorkflowStatus::Completed);
    // step_one: 10 + 1 = 11 ; step_two: 11 * 2 = 22
    assert_eq!(run.output, json!({ "final": 22 }));
}

#[tokio::test]
async fn worker_reports_failure() {
    let engine = Arc::new(Engine::new(MemoryStore::new()));
    let def = parse_workflow_def(
        r#"{ "name": "flaky", "tasks": [
            { "name": "risky", "taskReferenceName": "r" }
        ]}"#,
    )
    .expect("parse");
    engine.register(&def).await.expect("register");
    let id = engine.start("flaky", None, json!({}), None).await.expect("start");

    let base = spawn_server(engine.clone()).await;

    let mut worker = Worker::new(&base, "worker-1");
    worker.register("risky", |_ctx| async move { Err("boom".to_string()) });

    let processed = worker.run_until_idle().await.expect("run worker");
    assert_eq!(processed, 1);

    let run = engine.get_run(&id).await.expect("run");
    assert_eq!(run.status, WorkflowStatus::Failed);
}
